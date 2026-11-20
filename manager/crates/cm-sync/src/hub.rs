//! [`DeviceHub`]: everything about devices one install needs, behind one object that the API
//! layer (UI routes + sync listener) and the sync engine (phase 17) share.
//!
//! Reachability — how the other device reaches this one's sync listener (`127.0.0.1:sync_port`):
//!
//! | mode | endpoint in the pairing link | set up by |
//! |---|---|---|
//! | `serve` | `https://<magicdns>:<https_port>` | `tailscale serve --bg …`, run only after the user confirms |
//! | `tailnetIp` | `http://<100.x>:<sync_port>` (plain HTTP inside WireGuard) | an extra listener on the 100.x IP, after the user confirms |
//! | `advertise` | `http://<CM_SYNC_ADVERTISE>` | env, for tests / development (e.g. two instances on loopback) |
//!
//! Tailscale missing / stopped / logged out: the hub still answers (status says why) and every
//! operation that needs the tailnet fails with a hint; nothing else in the app depends on it.

use crate::{
    auth::{AuthError, Incoming, Verifier},
    client::{PeerClient, PeerError, REQUEST_TIMEOUT, http_client, status_error},
    codec::b64url,
    identity::Identity,
    pairing::{self, CodeError, PairRequest, PairResponse, PairUrl, PairingCodes},
};
use anyhow::{Result, anyhow, bail};
use cm_core::{Clock, Device, DeviceRepo, SettingsRepo};
use cm_tailscale::{Confirmed, ServePlan, Status as TsStatus, Tailscale};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::Duration,
};

/// Setting: the user chose the tailnet-IP fallback; rebind it on startup.
pub const SETTING_TAILNET_IP: &str = "sync.reach_tailnet_ip";
/// Default port of the sync listener (D7 style: `CM_SYNC_PORT`).
pub const DEFAULT_SYNC_PORT: u16 = 7879;
/// Tailscale status is cached this long (seconds) to avoid spawning the CLI on every poll.
const STATUS_TTL: u64 = 5;

#[derive(Clone, Debug)]
pub struct HubConfig {
    /// Port of the loopback sync listener.
    pub sync_port: u16,
    /// HTTPS port used with `tailscale serve`.
    pub https_port: u16,
    /// `host:port` advertised as plain HTTP instead of the tailnet (tests / development only).
    pub advertise: Option<String>,
    /// This machine's name as shown on the other device.
    pub device_name: String,
}

pub struct HubDeps {
    pub identity: Arc<Identity>,
    pub repo: Arc<dyn DeviceRepo>,
    pub tailscale: Arc<dyn Tailscale>,
    pub settings: Arc<dyn SettingsRepo>,
    pub clock: Arc<dyn Clock>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReachMode {
    Serve,
    TailnetIp,
    Advertise,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reach {
    pub mode: ReachMode,
    /// Base URL peers use (`https://host:port`).
    pub endpoint: String,
    pub host: String,
    pub port: u16,
    /// Plain HTTP (inside WireGuard) rather than HTTPS.
    pub plain: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Me {
    pub id: String,
    pub name: String,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HubStatus {
    /// Device sync can be used (Tailscale running, or an advertise override for tests).
    pub enabled: bool,
    pub tailscale: TsStatus,
    pub me: Me,
    pub sync_port: u16,
    /// Addresses the sync listener is bound to.
    pub listeners: Vec<String>,
    /// How peers reach this device now (`None`: not reachable yet).
    pub reach: Option<Reach>,
    /// The `tailscale serve` command "Make reachable" would run (needs HTTPS certs).
    pub serve_plan: ServePlan,
    /// `tailscale serve` can work (HTTPS certificates are enabled for the tailnet).
    pub serve_available: bool,
    /// The 100.x address the fallback would bind.
    pub tailnet_ip: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferView {
    /// `cmsync://…` link (also the QR content).
    pub url: String,
    pub code: String,
    pub expires_at: u64,
    pub expires_in: u64,
    pub endpoint: String,
    pub plain: bool,
}

/// Answer of the signed `GET /sync/v1/hello`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hello {
    pub device_id: String,
    pub name: String,
    /// The caller as the peer sees it.
    pub you: String,
    pub time: u64,
    pub protocol: u32,
}

pub const PROTOCOL: u32 = 1;

/// Failure of `POST /sync/v1/pair` on the receiving side (all 4xx).
#[derive(Debug, thiserror::Error)]
pub enum PairRefusal {
    #[error("{0}")]
    Code(#[from] CodeError),
    #[error("invalid pairing request: {0}")]
    Invalid(String),
    #[error("pairing request is not signed by the key it presents")]
    BadSignature,
    #[error("cannot pair a device with itself")]
    SelfPair,
}

impl PairRefusal {
    pub fn code(&self) -> &'static str {
        match self {
            PairRefusal::Code(_) => "pair_code_invalid",
            PairRefusal::Invalid(_) => "pair_invalid",
            PairRefusal::BadSignature => "pair_bad_signature",
            PairRefusal::SelfPair => "pair_self",
        }
    }
}

pub struct DeviceHub {
    cfg: HubConfig,
    d: HubDeps,
    codes: PairingCodes,
    verifier: Verifier,
    status_cache: Mutex<Option<(u64, TsStatus)>>,
    listeners: Mutex<Vec<SocketAddr>>,
}

impl DeviceHub {
    pub fn new(cfg: HubConfig, d: HubDeps) -> DeviceHub {
        let verifier = Verifier::new(d.identity.id(), d.repo.clone(), d.clock.clone());
        DeviceHub { cfg, d, codes: PairingCodes::default(), verifier, status_cache: Mutex::new(None), listeners: Mutex::new(Vec::new()) }
    }

    pub fn config(&self) -> &HubConfig {
        &self.cfg
    }

    pub fn identity(&self) -> &Arc<Identity> {
        &self.d.identity
    }

    pub fn clock(&self) -> &Arc<dyn Clock> {
        &self.d.clock
    }

    pub fn me(&self) -> Me {
        Me { id: self.d.identity.id(), name: self.cfg.device_name.clone(), fingerprint: self.d.identity.fingerprint().to_owned() }
    }

    pub fn serve_plan(&self) -> ServePlan {
        ServePlan::new(self.cfg.https_port, self.cfg.sync_port)
    }

    /// Record a bound sync listener (the API layer binds; the hub reports).
    pub fn add_listener(&self, addr: SocketAddr) {
        let mut l = self.listeners.lock().unwrap_or_else(|e| e.into_inner());
        if !l.contains(&addr) {
            l.push(addr);
        }
    }

    pub fn listeners(&self) -> Vec<SocketAddr> {
        self.listeners.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn tailnet_listener(&self) -> Option<SocketAddr> {
        self.listeners().into_iter().find(|a| crate::policy::classify(a.ip()) == crate::policy::Source::Tailnet)
    }

    /// Tailscale status, cached for a few seconds (`fresh` bypasses the cache).
    pub async fn tailscale(&self, fresh: bool) -> TsStatus {
        let now = self.d.clock.now();
        if !fresh
            && let Some((at, s)) = self.status_cache.lock().unwrap_or_else(|e| e.into_inner()).clone()
            && now.saturating_sub(at) < STATUS_TTL
        {
            return s;
        }
        let s = self.d.tailscale.status().await;
        *self.status_cache.lock().unwrap_or_else(|e| e.into_inner()) = Some((now, s.clone()));
        s
    }

    /// How peers reach this device right now.
    pub async fn reach(&self, ts: &TsStatus) -> Option<Reach> {
        if let Some(adv) = &self.cfg.advertise {
            let (host, port) = adv.rsplit_once(':')?;
            let port: u16 = port.parse().ok()?;
            let host = host.trim_matches(['[', ']']).to_owned();
            let url = PairUrl { host: host.clone(), port, code: String::new(), fingerprint: String::new(), plain: true };
            return Some(Reach { mode: ReachMode::Advertise, endpoint: url.endpoint(), host, port, plain: true });
        }
        if !ts.running() {
            return None;
        }
        if let Some(dns) = &ts.dns_name
            && ts.https_certs
            && self.d.tailscale.serving(&self.serve_plan()).await
        {
            let u = PairUrl { host: dns.clone(), port: self.cfg.https_port, code: String::new(), fingerprint: String::new(), plain: false };
            return Some(Reach {
                mode: ReachMode::Serve,
                endpoint: u.endpoint(),
                host: dns.clone(),
                port: self.cfg.https_port,
                plain: false,
            });
        }
        let addr = self.tailnet_listener()?;
        let host = addr.ip().to_string();
        let u = PairUrl { host: host.clone(), port: addr.port(), code: String::new(), fingerprint: String::new(), plain: true };
        Some(Reach { mode: ReachMode::TailnetIp, endpoint: u.endpoint(), host, port: addr.port(), plain: true })
    }

    pub async fn status(&self, fresh: bool) -> HubStatus {
        let ts = self.tailscale(fresh).await;
        let reach = self.reach(&ts).await;
        HubStatus {
            enabled: ts.running() || self.cfg.advertise.is_some(),
            serve_available: ts.running() && ts.https_certs && ts.dns_name.is_some(),
            tailnet_ip: ts.ipv4().map(str::to_owned),
            me: self.me(),
            sync_port: self.cfg.sync_port,
            listeners: self.listeners().iter().map(ToString::to_string).collect(),
            reach,
            serve_plan: self.serve_plan(),
            tailscale: ts,
        }
    }

    fn not_enabled(ts: &TsStatus) -> anyhow::Error {
        anyhow!("device sync needs Tailscale: {}", ts.hint())
    }

    /// The live pairing link (a new code when none is live).
    pub async fn offer(&self) -> Result<OfferView> {
        let o = self.codes.current(self.d.clock.now())?;
        self.offer_view(o).await
    }

    /// Replace the pairing code.
    pub async fn regenerate(&self) -> Result<OfferView> {
        let o = self.codes.regenerate(self.d.clock.now())?;
        self.offer_view(o).await
    }

    async fn offer_view(&self, o: pairing::Offer) -> Result<OfferView> {
        let ts = self.tailscale(false).await;
        let Some(r) = self.reach(&ts).await else {
            if !ts.running() && self.cfg.advertise.is_none() {
                return Err(Self::not_enabled(&ts));
            }
            bail!("this device is not reachable from your tailnet yet: use \"Make reachable\" first");
        };
        let link = PairUrl {
            host: r.host.clone(),
            port: r.port,
            code: o.code.clone(),
            fingerprint: self.d.identity.fingerprint().to_owned(),
            plain: r.plain,
        };
        let now = self.d.clock.now();
        Ok(OfferView {
            url: link.to_link(),
            code: o.code,
            expires_at: o.expires_at,
            expires_in: o.expires_at.saturating_sub(now),
            endpoint: r.endpoint,
            plain: r.plain,
        })
    }

    /// Run `tailscale serve` — only when the user confirmed exactly `command` in the app.
    pub async fn confirm_serve(&self, command: &str, confirmed: Confirmed) -> Result<String> {
        let ts = self.tailscale(true).await;
        if !ts.running() {
            return Err(Self::not_enabled(&ts));
        }
        if !ts.https_certs {
            bail!(
                "HTTPS certificates are off for your tailnet, so `tailscale serve --https` cannot work. Enable them in the Tailscale admin console (DNS → HTTPS Certificates), or use the plain-HTTP fallback"
            );
        }
        let plan = self.serve_plan();
        if command.trim() != plan.command {
            bail!("the confirmed command does not match what would run; reload the page and confirm again");
        }
        let out = self.d.tailscale.serve(&plan, confirmed).await?;
        *self.status_cache.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(out)
    }

    /// Address for the tailnet-IP fallback listener (`<100.x>:<sync_port>`).
    pub async fn tailnet_bind_addr(&self) -> Result<SocketAddr> {
        let ts = self.tailscale(true).await;
        if !ts.running() {
            return Err(Self::not_enabled(&ts));
        }
        let ip: IpAddr = ts.ipv4().ok_or_else(|| anyhow!("Tailscale has no 100.x address for this machine yet"))?.parse()?;
        Ok(SocketAddr::new(ip, self.cfg.sync_port))
    }

    /// Remember (or forget) that the user chose the tailnet-IP fallback.
    pub async fn set_tailnet_fallback(&self, on: bool) -> Result<()> {
        self.d.settings.set(SETTING_TAILNET_IP, &json!(on)).await
    }

    pub async fn tailnet_fallback_chosen(&self) -> bool {
        matches!(self.d.settings.get(SETTING_TAILNET_IP).await, Ok(Some(serde_json::Value::Bool(true))))
    }

    // ---- devices ----

    pub async fn list(&self) -> Result<Vec<Device>> {
        self.d.repo.list().await
    }

    pub async fn get(&self, id: &str) -> Result<Option<Device>> {
        self.d.repo.get(id).await
    }

    /// Revoke: the next request signed by that device is refused (401).
    pub async fn revoke(&self, id: &str) -> Result<bool> {
        self.d.repo.revoke(id, &self.d.clock.now_iso()).await
    }

    /// Remove a device row (revoked or not).
    pub async fn forget(&self, id: &str) -> Result<bool> {
        self.d.repo.delete(id).await
    }

    /// Verify a signed request to the sync listener (server side).
    pub async fn verify(&self, req: &Incoming<'_>) -> Result<Device, AuthError> {
        self.verifier.verify(req).await
    }

    /// Signed client for a paired, non-revoked device with a known endpoint (phase 17 uses this).
    pub async fn client(&self, device_id: &str) -> Result<PeerClient> {
        self.client_with_timeout(device_id, REQUEST_TIMEOUT).await
    }

    pub async fn client_with_timeout(&self, device_id: &str, timeout: Duration) -> Result<PeerClient> {
        let d = self.d.repo.get(device_id).await?.ok_or_else(|| cm_core::CmError::NotFound(format!("device {device_id}")))?;
        if d.revoked_at.is_some() {
            return Err(cm_core::CmError::Conflict(format!("device \"{}\" was revoked; pair it again first", d.name)).into());
        }
        let Some(url) = d.url.as_deref() else {
            return Err(cm_core::CmError::Conflict(format!(
                "\"{}\" did not share an address when pairing (it was not reachable). Make it reachable, then pair again from it",
                d.name
            ))
            .into());
        };
        Ok(PeerClient::new(url, &d.id, self.d.identity.clone(), self.d.clock.clone(), timeout)?)
    }

    /// Signed ping of a paired device; updates its "last seen" on success.
    pub async fn hello(&self, device_id: &str) -> Result<Hello> {
        let c = self.client(device_id).await?;
        let h: Hello = c.get_json("/sync/v1/hello").await?;
        if h.device_id != device_id {
            bail!("{} answered as another device ({}); pair again", c.host(), h.device_id);
        }
        let _ = self.d.repo.touch(device_id, &self.d.clock.now_iso(), Some(c.host())).await;
        Ok(h)
    }

    /// Answer of the signed hello for `peer`.
    pub fn hello_for(&self, peer: &Device) -> Hello {
        Hello {
            device_id: self.d.identity.id(),
            name: self.cfg.device_name.clone(),
            you: peer.id.clone(),
            time: self.d.clock.now(),
            protocol: PROTOCOL,
        }
    }

    /// Server side of `POST /sync/v1/pair`.
    pub async fn accept_pair(
        &self,
        req: PairRequest,
        tailscale_login: Option<String>,
        source: Option<String>,
    ) -> Result<PairResponse, PairRefusal> {
        pairing::check_peer(&req.device_id, &req.public_key, &req.name).map_err(|e| PairRefusal::Invalid(e.to_string()))?;
        if req.public_key == self.d.identity.public_hex() {
            return Err(PairRefusal::SelfPair);
        }
        if let Some(u) = &req.url {
            let ok =
                reqwest::Url::parse(u).is_ok_and(|p| matches!(p.scheme(), "http" | "https") && p.host_str().is_some() && p.path() == "/");
            if !ok || u.len() > 300 {
                return Err(PairRefusal::Invalid("bad url".into()));
            }
        }
        let now = self.d.clock.now();
        // Cheap reject of wrong codes first (counts toward burning the code), then key possession.
        self.codes.check(&req.code, now)?;
        if !pairing::verify_request(self.d.identity.fingerprint(), &req) {
            return Err(PairRefusal::BadSignature);
        }
        self.codes.consume(&req.code, now)?;
        let device = Device {
            id: req.device_id.clone(),
            name: req.name.trim().to_owned(),
            public_key: req.public_key.clone(),
            url: req.url.as_ref().map(|u| u.trim_end_matches('/').to_owned()),
            tailscale_login,
            paired_at: self.d.clock.now_iso(),
            last_seen_at: Some(self.d.clock.now_iso()),
            last_seen_addr: source,
            revoked_at: None,
        };
        self.d.repo.upsert(&device).await.map_err(|e| PairRefusal::Invalid(e.to_string()))?;
        let mut resp = PairResponse {
            device_id: self.d.identity.id(),
            name: self.cfg.device_name.clone(),
            public_key: self.d.identity.public_hex().to_owned(),
            sig: String::new(),
        };
        resp.sig = b64url(&self.d.identity.sign(&pairing::response_transcript(&req.code, &req.public_key, &resp)));
        Ok(resp)
    }

    /// Client side: pair with the device behind a pasted / scanned `cmsync://` link, then say hello.
    pub async fn pair_with_url(&self, link: &str) -> Result<Device> {
        let link = PairUrl::parse(link)?;
        if link.fingerprint == self.d.identity.fingerprint() {
            bail!("this is this device's own pairing link: open it on the other device");
        }
        let ts = self.tailscale(false).await;
        if !ts.running() && self.cfg.advertise.is_none() {
            return Err(Self::not_enabled(&ts));
        }
        let my_url = self.reach(&ts).await.map(|r| r.endpoint);
        let mut req = PairRequest {
            code: link.code.clone(),
            device_id: self.d.identity.id(),
            name: self.cfg.device_name.clone(),
            public_key: self.d.identity.public_hex().to_owned(),
            url: my_url,
            sig: String::new(),
        };
        req.sig = b64url(&self.d.identity.sign(&pairing::request_transcript(&link.code, &link.fingerprint, &req)));

        let endpoint = link.endpoint();
        let http = http_client(REQUEST_TIMEOUT);
        let resp = http
            .post(format!("{endpoint}/sync/v1/pair"))
            .json(&req)
            .send()
            .await
            .map_err(|e| PeerError::from_reqwest(&e, &link.host, link.port))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.bytes().await.unwrap_or_default();
            // A refused code is not a revoked device: keep the server's plain message only.
            return Err(match status_error(&link.host, status, &body) {
                PeerError::Unauthorized { host, code, message } => PeerError::Status { host, status: 401, code, message },
                e => e,
            }
            .into());
        }
        let answer: PairResponse = resp.json().await.map_err(|e| PeerError::Protocol { host: link.host.clone(), detail: e.to_string() })?;
        pairing::check_peer(&answer.device_id, &answer.public_key, &answer.name)?;
        pairing::verify_response(&link, self.d.identity.public_hex(), &answer)?;
        let device = Device {
            id: answer.device_id.clone(),
            name: answer.name.trim().to_owned(),
            public_key: answer.public_key,
            url: Some(endpoint),
            tailscale_login: None,
            paired_at: self.d.clock.now_iso(),
            last_seen_at: Some(self.d.clock.now_iso()),
            last_seen_addr: Some(link.host.clone()),
            revoked_at: None,
        };
        self.d.repo.upsert(&device).await?;
        self.hello(&device.id).await.map_err(|e| anyhow!("paired with \"{}\", but the signed hello failed: {e:#}", device.name))?;
        Ok(self.d.repo.get(&device.id).await?.unwrap_or(device))
    }
}

/// Settings + device repo need a key/value store; tests use this one.
#[cfg(test)]
pub(crate) mod mem_settings {
    use async_trait::async_trait;
    use cm_core::SettingsRepo;
    use serde_json::Value;
    use std::{collections::HashMap, sync::Mutex};

    #[derive(Default)]
    pub struct MemSettings(Mutex<HashMap<String, Value>>);

    #[async_trait]
    impl SettingsRepo for MemSettings {
        async fn get(&self, key: &str) -> anyhow::Result<Option<Value>> {
            Ok(self.0.lock().unwrap().get(key).cloned())
        }
        async fn set(&self, key: &str, value: &Value) -> anyhow::Result<()> {
            self.0.lock().unwrap().insert(key.into(), value.clone());
            Ok(())
        }
        async fn delete(&self, key: &str) -> anyhow::Result<()> {
            self.0.lock().unwrap().remove(key);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{FakeClock, MemDevices};
    use cm_tailscale::{Fake, State};

    fn hub(ts: Fake, advertise: Option<&str>) -> (DeviceHub, Arc<MemDevices>, Arc<Fake>) {
        let repo = Arc::new(MemDevices::default());
        let ts = Arc::new(ts);
        let h = DeviceHub::new(
            HubConfig { sync_port: 47999, https_port: 8443, advertise: advertise.map(Into::into), device_name: "Mac A".into() },
            HubDeps {
                identity: Arc::new(Identity::generate().unwrap()),
                repo: repo.clone(),
                tailscale: ts.clone(),
                settings: Arc::new(mem_settings::MemSettings::default()),
                clock: Arc::new(FakeClock::new(1_800_000_000)),
            },
        );
        (h, repo, ts)
    }

    fn request_from(b: &Identity, code: &str, a_fp: &str) -> PairRequest {
        let mut r = PairRequest {
            code: code.into(),
            device_id: b.id(),
            name: "Mac B".into(),
            public_key: b.public_hex().into(),
            url: None,
            sig: String::new(),
        };
        r.sig = b64url(&b.sign(&pairing::request_transcript(code, a_fp, &r)));
        r
    }

    #[tokio::test]
    async fn disabled_without_tailscale() {
        let (h, _, _) = hub(Fake::new(TsStatus::with_state(State::NotInstalled, None)), None);
        let s = h.status(false).await;
        assert!(!s.enabled && s.reach.is_none());
        let e = h.offer().await.unwrap_err().to_string();
        assert!(e.contains("not installed"), "{e}");
        assert!(h.pair_with_url(&format!("cmsync://x.ts.net:8443/pair?c={}&fp={}", "a".repeat(26), "0".repeat(64))).await.is_err());
    }

    #[tokio::test]
    async fn reach_modes_and_confirmed_serve() {
        let (h, _, ts) = hub(Fake::running(), None);
        assert!(h.status(true).await.reach.is_none(), "running but not served yet");
        assert!(h.offer().await.unwrap_err().to_string().contains("Make reachable"));
        // Wrong command is refused, nothing runs.
        assert!(h.confirm_serve("tailscale funnel 8443", Confirmed::by_user()).await.is_err());
        assert!(ts.serve_calls.lock().unwrap().is_empty());
        h.confirm_serve(&h.serve_plan().command, Confirmed::by_user()).await.unwrap();
        assert_eq!(ts.serve_calls.lock().unwrap().as_slice(), ["tailscale serve --bg --https=8443 http://127.0.0.1:47999"]);
        let r = h.status(true).await.reach.unwrap();
        assert_eq!((r.mode, r.endpoint.as_str()), (ReachMode::Serve, "https://test-mac.tail0000.ts.net:8443"));
        let o = h.offer().await.unwrap();
        assert!(o.url.starts_with("cmsync://test-mac.tail0000.ts.net:8443/pair?c="), "{}", o.url);
        assert!(o.url.ends_with(&format!("&fp={}", h.identity().fingerprint())));
        assert_eq!(o.expires_in, 600);

        // Fallback: a listener on the 100.x IP.
        let (h2, _, _) = hub(Fake::running(), None);
        assert_eq!(h2.tailnet_bind_addr().await.unwrap().to_string(), "100.100.1.2:47999");
        h2.add_listener("100.100.1.2:47999".parse().unwrap());
        let r = h2.status(true).await.reach.unwrap();
        assert_eq!((r.mode, r.endpoint.as_str(), r.plain), (ReachMode::TailnetIp, "http://100.100.1.2:47999", true));
        assert!(h2.offer().await.unwrap().url.ends_with("&plain=1"));
    }

    #[tokio::test]
    async fn accept_pair_rules() {
        let (a, repo, _) = hub(Fake::running(), Some("127.0.0.1:47999"));
        let b = Identity::generate().unwrap();
        let offer = a.offer().await.unwrap();
        let fp = a.identity().fingerprint().to_owned();

        // Wrong code.
        assert!(matches!(a.accept_pair(request_from(&b, &"b".repeat(26), &fp), None, None).await, Err(PairRefusal::Code(_))));
        // Signature for another target fingerprint.
        let r = request_from(&b, &offer.code, &"0".repeat(64));
        assert!(matches!(a.accept_pair(r, None, None).await, Err(PairRefusal::BadSignature)));
        // Tampered name after signing.
        let mut r = request_from(&b, &offer.code, &fp);
        r.name = "Evil".into();
        assert!(matches!(a.accept_pair(r, None, None).await, Err(PairRefusal::BadSignature)));
        // Self.
        let me = a.identity().clone();
        assert!(matches!(a.accept_pair(request_from(&me, &offer.code, &fp), None, None).await, Err(PairRefusal::SelfPair)));

        // Good one, then the code is gone.
        let resp =
            a.accept_pair(request_from(&b, &offer.code, &fp), Some("tester@example.com".into()), Some("127.0.0.1".into())).await.unwrap();
        let link = PairUrl::parse(&offer.url).unwrap();
        pairing::verify_response(&link, b.public_hex(), &resp).unwrap();
        let stored = repo.get_sync(&b.id()).unwrap();
        assert_eq!(stored.tailscale_login.as_deref(), Some("tester@example.com"));
        let c = Identity::generate().unwrap();
        assert!(matches!(a.accept_pair(request_from(&c, &offer.code, &fp), None, None).await, Err(PairRefusal::Code(_))), "single use");

        // B detects a response from a different key than the link pinned.
        let mut evil = resp.clone();
        let x = Identity::generate().unwrap();
        evil.public_key = x.public_hex().into();
        evil.device_id = x.id();
        assert!(pairing::verify_response(&link, b.public_hex(), &evil).unwrap_err().to_string().contains("fingerprint mismatch"));

        // Revoke → client() refuses locally too.
        assert!(a.revoke(&b.id()).await.unwrap());
        assert!(a.client(&b.id()).await.is_err());
    }
}
