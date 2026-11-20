//! Pairing (D9). A shows a link (+ QR); B pastes or scans it; both end up holding the other's
//! Ed25519 public key. The link carries no long-lived secret:
//!
//! ```text
//! cmsync://<host>:<port>/pair?c=<code>&fp=<sha256(A pubkey) hex>[&plain=1]
//! ```
//!
//! - `c`: one-time code, 128 random bits as 26 base32 chars, valid 10 minutes, single use;
//!   making a new one replaces the unused one. Too many wrong guesses also burn it.
//! - `fp`: pins A's key, so B detects a wrong or intercepted endpoint.
//! - `plain=1`: the endpoint is plain HTTP (tailnet-IP fallback, still inside WireGuard), else HTTPS
//!   through `tailscale serve`.
//!
//! Exchange (`POST /sync/v1/pair` on A's sync listener, JSON):
//!
//! ```text
//! B → A  PairRequest  { code, deviceId, name, publicKey, url?, sig = B.sign(request_transcript) }
//! A      checks B's signature (key possession), then consumes the code, stores B
//! A → B  PairResponse { deviceId, name, publicKey, sig = A.sign(response_transcript) }
//! B      checks sha256(publicKey) == fp and A's signature, stores A with the link's endpoint
//! ```

use crate::{
    codec::{base32, random, same, unhex},
    identity::{device_id, fingerprint, parse_public_key, verify},
};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

pub const URL_SCHEME: &str = "cmsync";
pub const CODE_TTL_SECS: u64 = 10 * 60;
pub const CODE_CHARS: usize = 26;
/// Wrong codes before the current one is burnt.
pub const MAX_FAILURES: u32 = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub code: String,
    pub expires_at: u64,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum CodeError {
    /// Wrong, expired, already used or replaced: deliberately one answer.
    #[error("this pairing code is wrong, expired or already used. Make a new one on the other device")]
    Invalid,
}

#[derive(Default)]
struct Codes {
    current: Option<Offer>,
    failures: u32,
}

/// The one live pairing code of this install (in memory: a restart invalidates it).
#[derive(Default)]
pub struct PairingCodes(Mutex<Codes>);

fn new_code() -> Result<String> {
    Ok(base32(&random(16)?))
}

impl PairingCodes {
    fn lock(&self) -> std::sync::MutexGuard<'_, Codes> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The live code, or a new one if none is live.
    pub fn current(&self, now: u64) -> Result<Offer> {
        let mut c = self.lock();
        match &c.current {
            Some(o) if o.expires_at > now => Ok(o.clone()),
            _ => {
                let o = Offer { code: new_code()?, expires_at: now + CODE_TTL_SECS };
                c.current = Some(o.clone());
                c.failures = 0;
                Ok(o)
            }
        }
    }

    /// Replace the live code (the old one stops working at once).
    pub fn regenerate(&self, now: u64) -> Result<Offer> {
        let mut c = self.lock();
        let o = Offer { code: new_code()?, expires_at: now + CODE_TTL_SECS };
        c.current = Some(o.clone());
        c.failures = 0;
        Ok(o)
    }

    /// Is `code` the live one? Does not consume it (see [`PairingCodes::consume`]).
    pub fn check(&self, code: &str, now: u64) -> Result<(), CodeError> {
        let mut c = self.lock();
        let ok = matches!(&c.current, Some(o) if o.expires_at > now && same(o.code.as_bytes(), code.as_bytes()));
        if ok {
            return Ok(());
        }
        c.failures += 1;
        if c.failures >= MAX_FAILURES {
            c.current = None;
        }
        Err(CodeError::Invalid)
    }

    /// Use the code: valid once.
    pub fn consume(&self, code: &str, now: u64) -> Result<(), CodeError> {
        self.check(code, now)?;
        let mut c = self.lock();
        // Re-check under the same lock that clears it, so two concurrent uses cannot both win.
        match &c.current {
            Some(o) if o.expires_at > now && same(o.code.as_bytes(), code.as_bytes()) => {
                c.current = None;
                Ok(())
            }
            _ => Err(CodeError::Invalid),
        }
    }
}

/// A parsed pairing link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairUrl {
    /// Host name or IP literal (IPv6 without brackets).
    pub host: String,
    pub port: u16,
    pub code: String,
    /// sha256 of A's public key, 64 hex.
    pub fingerprint: String,
    pub plain: bool,
}

fn bracket(host: &str) -> String {
    if host.contains(':') { format!("[{host}]") } else { host.to_owned() }
}

impl PairUrl {
    pub fn to_link(&self) -> String {
        let plain = if self.plain { "&plain=1" } else { "" };
        format!("{URL_SCHEME}://{}:{}/pair?c={}&fp={}{plain}", bracket(&self.host), self.port, self.code, self.fingerprint)
    }

    /// `https://host:port` or `http://host:port` (plain).
    pub fn endpoint(&self) -> String {
        format!("{}://{}:{}", if self.plain { "http" } else { "https" }, bracket(&self.host), self.port)
    }

    /// Strict parse: scheme `cmsync`, path exactly `/pair`, a well-formed code and fingerprint,
    /// and an IP literal only if it is loopback or tailnet.
    pub fn parse(input: &str) -> Result<PairUrl> {
        let input = input.trim();
        if input.len() > 512 {
            bail!("this is not a pairing link (too long)");
        }
        let url = reqwest::Url::parse(input).map_err(|_| anyhow::anyhow!("this is not a pairing link: it should start with cmsync://"))?;
        if url.scheme() != URL_SCHEME {
            bail!("this is not a pairing link: it should start with cmsync://");
        }
        if url.path() != "/pair" || url.username() != "" || url.password().is_some() || url.fragment().is_some() {
            bail!("this is not a pairing link (expected cmsync://<host>:<port>/pair?…)");
        }
        let host = match url.host() {
            Some(url::Host::Domain(d)) => d.to_ascii_lowercase(),
            Some(url::Host::Ipv4(ip)) => ip.to_string(),
            Some(url::Host::Ipv6(ip)) => ip.to_string(),
            None => bail!("the pairing link has no host"),
        };
        if let Ok(ip) = host.parse::<std::net::IpAddr>()
            && !crate::policy::allowed(ip)
        {
            bail!("the pairing link points to {ip}, which is not a Tailscale address. Device sync only works over Tailscale");
        }
        let Some(port) = url.port() else { bail!("the pairing link has no port") };
        let (mut code, mut fp, mut plain) = (None, None, false);
        for (k, v) in url.query_pairs() {
            match k.as_ref() {
                "c" => code = Some(v.to_ascii_lowercase()),
                "fp" => fp = Some(v.to_ascii_lowercase()),
                "plain" => plain = v == "1",
                _ => {}
            }
        }
        let code = code.filter(|c| c.len() == CODE_CHARS && c.bytes().all(|b| b.is_ascii_lowercase() || (b'2'..=b'7').contains(&b)));
        let Some(code) = code else { bail!("the pairing link has no valid code") };
        let Some(fingerprint) = fp.filter(|f| f.len() == 64 && unhex(f).is_some()) else {
            bail!("the pairing link has no valid fingerprint")
        };
        Ok(PairUrl { host, port, code, fingerprint, plain })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairRequest {
    pub code: String,
    pub device_id: String,
    pub name: String,
    pub public_key: String,
    /// B's own sync endpoint, so A can reach B later (optional: B may not be reachable).
    #[serde(default)]
    pub url: Option<String>,
    /// base64url Ed25519 signature over [`request_transcript`].
    pub sig: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairResponse {
    pub device_id: String,
    pub name: String,
    pub public_key: String,
    /// base64url Ed25519 signature over [`response_transcript`].
    pub sig: String,
}

pub fn request_transcript(code: &str, target_fp: &str, r: &PairRequest) -> Vec<u8> {
    format!("CMSYNC1-PAIR-REQ\n{code}\n{target_fp}\n{}\n{}\n{}\n{}", r.device_id, r.public_key, r.url.as_deref().unwrap_or(""), r.name)
        .into_bytes()
}

pub fn response_transcript(code: &str, requester_key: &str, r: &PairResponse) -> Vec<u8> {
    format!("CMSYNC1-PAIR-RESP\n{code}\n{requester_key}\n{}\n{}\n{}", r.device_id, r.public_key, r.name).into_bytes()
}

/// Shape checks shared by both sides: a valid key whose id matches, a sane name.
pub fn check_peer(device_id_claimed: &str, public_key: &str, name: &str) -> Result<()> {
    let Some(k) = parse_public_key(public_key) else { bail!("invalid public key") };
    if device_id(&k.to_bytes()) != device_id_claimed {
        bail!("device id does not match its key");
    }
    if name.trim().is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        bail!("invalid device name");
    }
    Ok(())
}

/// A's check of B's request signature, against A's own fingerprint.
pub fn verify_request(my_fp: &str, r: &PairRequest) -> bool {
    let Some(sig) = crate::codec::unb64url(&r.sig) else { return false };
    verify(&r.public_key, &request_transcript(&r.code, my_fp, r), &sig)
}

/// B's check of A's response: the key is the one pinned by the link, and A signed this exchange.
pub fn verify_response(link: &PairUrl, my_key: &str, r: &PairResponse) -> Result<()> {
    let Some(k) = parse_public_key(&r.public_key) else { bail!("the other device sent an invalid key") };
    if fingerprint(&k.to_bytes()) != link.fingerprint {
        bail!(
            "the device at {} is not the one in the pairing link (key fingerprint mismatch). Do not continue: make a new link on the device you want to pair",
            link.host
        );
    }
    let ok = crate::codec::unb64url(&r.sig).is_some_and(|sig| verify(&r.public_key, &response_transcript(&link.code, my_key, r), &sig));
    if !ok {
        bail!("the other device's answer is not signed by its key");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ttl_single_use_regenerate() {
        let c = PairingCodes::default();
        let a = c.current(1000).unwrap();
        assert_eq!(a.code.len(), CODE_CHARS);
        assert_eq!(a.expires_at, 1000 + 600);
        assert_eq!(c.current(1100).unwrap(), a, "same code while live");
        assert_eq!(c.check(&a.code, 1599), Ok(()));
        assert_eq!(c.check(&a.code, 1600), Err(CodeError::Invalid), "expired at 10 min");
        // Single use.
        let b = c.regenerate(2000).unwrap();
        assert_ne!(a.code, b.code);
        assert_eq!(c.consume(&a.code, 2001), Err(CodeError::Invalid), "replaced code is dead");
        assert_eq!(c.consume(&b.code, 2001), Ok(()));
        assert_eq!(c.consume(&b.code, 2002), Err(CodeError::Invalid), "used once");
        let fresh = c.current(2003).unwrap();
        assert_ne!(fresh.code, b.code, "a new code after use");
    }

    #[test]
    fn wrong_guesses_burn_the_code() {
        let c = PairingCodes::default();
        let a = c.current(0).unwrap();
        for _ in 0..MAX_FAILURES {
            let _ = c.check("aaaaaaaaaaaaaaaaaaaaaaaaaa", 1);
        }
        assert_eq!(c.check(&a.code, 2), Err(CodeError::Invalid));
    }

    #[test]
    fn url_roundtrip_and_strictness() {
        let u = PairUrl { host: "mac.tail0000.ts.net".into(), port: 8443, code: "a".repeat(26), fingerprint: "0".repeat(64), plain: false };
        let link = u.to_link();
        assert_eq!(PairUrl::parse(&link).unwrap(), u);
        assert_eq!(u.endpoint(), "https://mac.tail0000.ts.net:8443");
        let v6 = PairUrl { host: "fd7a:115c:a1e0::1".into(), plain: true, ..u.clone() };
        assert_eq!(PairUrl::parse(&v6.to_link()).unwrap(), v6);
        assert_eq!(v6.endpoint(), "http://[fd7a:115c:a1e0::1]:8443");
        let loop4 = PairUrl { host: "127.0.0.1".into(), ..u.clone() };
        assert!(PairUrl::parse(&loop4.to_link()).is_ok());

        let fp = "0".repeat(64);
        let code = "a".repeat(26);
        for bad in [
            format!("https://h:1/pair?c={code}&fp={fp}"),
            format!("cmsync://h:1/pairing?c={code}&fp={fp}"),
            format!("cmsync://h/pair?c={code}&fp={fp}"),
            format!("cmsync://h:1/pair?c=short&fp={fp}"),
            format!("cmsync://h:1/pair?c={code}&fp=xyz"),
            format!("cmsync://8.8.8.8:1/pair?c={code}&fp={fp}"),
            format!("cmsync://192.168.1.2:1/pair?c={code}&fp={fp}"),
            format!("cmsync://u:p@h:1/pair?c={code}&fp={fp}"),
            "not a url".into(),
        ] {
            assert!(PairUrl::parse(&bad).is_err(), "{bad}");
        }
    }
}
