//! Signed requests between paired devices.
//!
//! ```text
//! Authorization: Device <device id>:<unix ts>:<nonce>:<base64url Ed25519 signature>
//!
//! signed bytes = "CMSYNC1\n" METHOD "\n" path?query "\n" audience "\n" ts "\n" nonce "\n" hex(sha256(body))
//! ```
//!
//! - `audience` is the receiver's device id, so a request meant for A cannot be replayed to C.
//! - Accepted when the device is paired and not revoked, `|now - ts| <= 60 s`, the signature
//!   verifies, and the `(device, nonce)` pair was not seen before (replay cache, kept for the
//!   window). Requests signed before this process started are refused too, so a restart (which
//!   empties the cache) does not reopen the window.
//! - The device row is read on every request: revoking takes effect on the next request.

use crate::{
    codec::{b64url, hex, random, sha256_hex, unb64url},
    identity::{Identity, verify},
};
use cm_core::{Clock, Device, DeviceRepo};
use std::{collections::HashMap, sync::Arc, sync::Mutex};

pub const SCHEME: &str = "Device";
/// Allowed clock difference between the two machines, seconds.
pub const WINDOW_SECS: u64 = 60;
/// Replay-cache cap; beyond it signed requests are refused until entries expire.
const MAX_NONCES: usize = 200_000;

/// Bytes covered by the signature.
pub fn canonical(method: &str, path_and_query: &str, audience: &str, ts: u64, nonce: &str, body: &[u8]) -> Vec<u8> {
    format!("CMSYNC1\n{}\n{path_and_query}\n{audience}\n{ts}\n{nonce}\n{}", method.to_ascii_uppercase(), sha256_hex(body)).into_bytes()
}

/// `Authorization` header value for a request from `me` to the device `audience`.
pub fn sign_request(me: &Identity, audience: &str, method: &str, path_and_query: &str, body: &[u8], now: u64) -> anyhow::Result<String> {
    let nonce = hex(&random(16)?);
    Ok(sign_with_nonce(me, audience, method, path_and_query, body, now, &nonce))
}

pub(crate) fn sign_with_nonce(
    me: &Identity,
    audience: &str,
    method: &str,
    path_and_query: &str,
    body: &[u8],
    ts: u64,
    nonce: &str,
) -> String {
    let sig = me.sign(&canonical(method, path_and_query, audience, ts, nonce, body));
    format!("{SCHEME} {}:{ts}:{nonce}:{}", me.id(), b64url(&sig))
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParsedAuth<'a> {
    pub device: &'a str,
    pub ts: u64,
    pub nonce: &'a str,
    pub sig: Vec<u8>,
}

pub fn parse_header(v: &str) -> Option<ParsedAuth<'_>> {
    let rest = v.strip_prefix(SCHEME)?.strip_prefix(' ')?;
    let mut it = rest.splitn(4, ':');
    let (device, ts, nonce, sig) = (it.next()?, it.next()?, it.next()?, it.next()?);
    let id_ok = device.len() == 16 && device.bytes().all(|c| c.is_ascii_hexdigit());
    let nonce_ok = (16..=64).contains(&nonce.len()) && nonce.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_');
    if !id_ok || !nonce_ok || ts.len() > 12 {
        return None;
    }
    let sig = unb64url(sig).filter(|s| s.len() == 64)?;
    Some(ParsedAuth { device, ts: ts.parse().ok()?, nonce, sig })
}

/// Why a signed request was refused. All are 401 on the wire; `code()` goes in the JSON body.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum AuthError {
    #[error("missing `Authorization: Device …` header")]
    Missing,
    #[error("malformed Authorization header")]
    Malformed,
    #[error("the calling device is not paired (or was removed)")]
    UnknownDevice,
    #[error("the calling device was revoked")]
    Revoked,
    #[error("request time is {0} s away from this machine's clock (allowed: 60 s); check both clocks")]
    Expired(i64),
    #[error("bad signature")]
    BadSignature,
    #[error("request already used (replay)")]
    Replayed,
    #[error("Tailscale user does not match the one seen at pairing")]
    LoginMismatch,
    #[error("too many requests, retry in a minute")]
    Busy,
}

impl AuthError {
    pub fn code(&self) -> &'static str {
        match self {
            AuthError::Missing => "auth_missing",
            AuthError::Malformed => "auth_malformed",
            AuthError::UnknownDevice => "unknown_device",
            AuthError::Revoked => "device_revoked",
            AuthError::Expired(_) => "clock_skew",
            AuthError::BadSignature => "bad_signature",
            AuthError::Replayed => "replayed",
            AuthError::LoginMismatch => "login_mismatch",
            AuthError::Busy => "busy",
        }
    }
}

/// The parts of an incoming request the verifier needs.
pub struct Incoming<'a> {
    pub method: &'a str,
    pub path_and_query: &'a str,
    pub authorization: Option<&'a str>,
    pub body: &'a [u8],
    /// `Tailscale-User-Login` set by `tailscale serve`, if any.
    pub tailscale_login: Option<&'a str>,
    /// Peer address, recorded as "last seen from".
    pub source: Option<String>,
}

pub struct Verifier {
    me_id: String,
    repo: Arc<dyn DeviceRepo>,
    clock: Arc<dyn Clock>,
    not_before: u64,
    /// `device:nonce` → time after which it can be forgotten.
    seen: Mutex<HashMap<String, u64>>,
}

impl Verifier {
    pub fn new(me_id: String, repo: Arc<dyn DeviceRepo>, clock: Arc<dyn Clock>) -> Verifier {
        let not_before = clock.now();
        Verifier { me_id, repo, clock, not_before, seen: Mutex::new(HashMap::new()) }
    }

    /// Check a request; on success returns the (non-revoked) device and records "last seen".
    pub async fn verify(&self, req: &Incoming<'_>) -> Result<Device, AuthError> {
        let auth = parse_header(req.authorization.ok_or(AuthError::Missing)?).ok_or(AuthError::Malformed)?;
        let device = self.repo.get(auth.device).await.map_err(|_| AuthError::UnknownDevice)?.ok_or(AuthError::UnknownDevice)?;
        if device.revoked_at.is_some() {
            return Err(AuthError::Revoked);
        }
        let now = self.clock.now();
        let skew = auth.ts as i64 - now as i64;
        if skew.unsigned_abs() > WINDOW_SECS {
            return Err(AuthError::Expired(skew));
        }
        if auth.ts < self.not_before {
            // Signed before this process started: the replay cache cannot vouch for it.
            return Err(AuthError::Expired(skew));
        }
        let msg = canonical(req.method, req.path_and_query, &self.me_id, auth.ts, auth.nonce, req.body);
        if !verify(&device.public_key, &msg, &auth.sig) {
            return Err(AuthError::BadSignature);
        }
        if let (Some(expected), Some(got)) = (device.tailscale_login.as_deref(), req.tailscale_login)
            && expected != got
        {
            return Err(AuthError::LoginMismatch);
        }
        {
            let mut seen = self.seen.lock().unwrap_or_else(|e| e.into_inner());
            if seen.len() >= MAX_NONCES / 2 {
                seen.retain(|_, until| *until >= now);
            }
            if seen.len() >= MAX_NONCES {
                return Err(AuthError::Busy);
            }
            let key = format!("{}:{}", auth.device, auth.nonce);
            if seen.contains_key(&key) {
                return Err(AuthError::Replayed);
            }
            seen.insert(key, auth.ts + WINDOW_SECS + 1);
        }
        let _ = self.repo.touch(&device.id, &self.clock.now_iso(), req.source.as_deref()).await;
        Ok(device)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{FakeClock, MemDevices, device_for};

    struct Setup {
        a: Identity,
        b: Identity,
        clock: Arc<FakeClock>,
        repo: Arc<MemDevices>,
        v: Verifier,
    }

    fn setup() -> Setup {
        let (a, b) = (Identity::generate().unwrap(), Identity::generate().unwrap());
        let clock = Arc::new(FakeClock::new(1_800_000_000));
        let repo = Arc::new(MemDevices::default());
        repo.put(device_for(&b));
        let v = Verifier::new(a.id(), repo.clone(), clock.clone());
        Setup { a, b, clock, repo, v }
    }

    fn incoming<'a>(h: &'a str, body: &'a [u8]) -> Incoming<'a> {
        Incoming { method: "POST", path_and_query: "/sync/v1/hello?x=1", authorization: Some(h), body, tailscale_login: None, source: None }
    }

    #[tokio::test]
    async fn valid_then_replayed() {
        let s = setup();
        let h = sign_request(&s.b, &s.a.id(), "POST", "/sync/v1/hello?x=1", b"{}", s.clock.now()).unwrap();
        assert_eq!(s.v.verify(&incoming(&h, b"{}")).await.unwrap().id, s.b.id());
        assert_eq!(s.v.verify(&incoming(&h, b"{}")).await, Err(AuthError::Replayed));
        assert!(s.repo.get_sync(&s.b.id()).unwrap().last_seen_at.is_some());
    }

    #[tokio::test]
    async fn expired_both_ways_and_skew_inside_window() {
        let s = setup();
        let now = s.clock.now();
        for ts in [now - 61, now + 61] {
            let h = sign_request(&s.b, &s.a.id(), "POST", "/sync/v1/hello?x=1", b"{}", ts).unwrap();
            assert!(matches!(s.v.verify(&incoming(&h, b"{}")).await, Err(AuthError::Expired(_))), "ts {ts}");
        }
        s.clock.set(now + 50);
        let h = sign_request(&s.b, &s.a.id(), "POST", "/sync/v1/hello?x=1", b"{}", now + 100).unwrap();
        assert!(s.v.verify(&incoming(&h, b"{}")).await.is_ok(), "50 s skew is fine");
        // Signed before the verifier started (replay cache was empty then).
        let h = sign_request(&s.b, &s.a.id(), "POST", "/sync/v1/hello?x=1", b"{}", now - 5).unwrap();
        assert!(matches!(s.v.verify(&incoming(&h, b"{}")).await, Err(AuthError::Expired(_))));
    }

    #[tokio::test]
    async fn tampered_parts_fail() {
        let s = setup();
        let now = s.clock.now();
        let h = sign_request(&s.b, &s.a.id(), "POST", "/sync/v1/hello?x=1", b"{}", now).unwrap();
        assert_eq!(s.v.verify(&incoming(&h, b"{ }")).await, Err(AuthError::BadSignature), "body");
        let mut r = incoming(&h, b"{}");
        r.path_and_query = "/sync/v1/hello?x=2";
        assert_eq!(s.v.verify(&r).await, Err(AuthError::BadSignature), "query");
        let mut r = incoming(&h, b"{}");
        r.method = "PUT";
        assert_eq!(s.v.verify(&r).await, Err(AuthError::BadSignature), "method");
        // For another audience.
        let other = Identity::generate().unwrap();
        let h2 = sign_request(&s.b, &other.id(), "POST", "/sync/v1/hello?x=1", b"{}", now).unwrap();
        assert_eq!(s.v.verify(&incoming(&h2, b"{}")).await, Err(AuthError::BadSignature), "audience");
        // Timestamp edited in the header.
        let forged = h.replacen(&format!(":{now}:"), &format!(":{}:", now + 1), 1);
        assert_eq!(s.v.verify(&incoming(&forged, b"{}")).await, Err(AuthError::BadSignature), "ts");
        // Signed by a key that is not the paired one but claims its id.
        let fake = sign_with_nonce(&other, &s.a.id(), "POST", "/sync/v1/hello?x=1", b"{}", now, "0123456789abcdef0123");
        let fake = fake.replacen(&other.id(), &s.b.id(), 1);
        assert_eq!(s.v.verify(&incoming(&fake, b"{}")).await, Err(AuthError::BadSignature), "key");
        // The genuine request still works: failures did not burn its nonce.
        assert!(s.v.verify(&incoming(&h, b"{}")).await.is_ok());
    }

    fn plain(h: &str) -> Incoming<'_> {
        Incoming { method: "POST", path_and_query: "/p", authorization: Some(h), body: b"", tailscale_login: None, source: None }
    }

    #[tokio::test]
    async fn unknown_revoked_malformed_login() {
        let s = setup();
        let now = s.clock.now();
        let stranger = Identity::generate().unwrap();
        let h = sign_request(&stranger, &s.a.id(), "POST", "/p", b"", now).unwrap();
        assert_eq!(s.v.verify(&plain(&h)).await, Err(AuthError::UnknownDevice));
        assert_eq!(s.v.verify(&plain("Bearer x")).await, Err(AuthError::Malformed));
        assert_eq!(s.v.verify(&plain("Device a:b:c:d")).await, Err(AuthError::Malformed));
        let mut none = plain("");
        none.authorization = None;
        assert_eq!(s.v.verify(&none).await, Err(AuthError::Missing));

        let mut d = device_for(&s.b);
        d.tailscale_login = Some("tester@example.com".into());
        s.repo.put(d);
        let h = sign_request(&s.b, &s.a.id(), "POST", "/p", b"", now).unwrap();
        let mut req = plain(&h);
        req.tailscale_login = Some("someone-else@example.com");
        assert_eq!(s.v.verify(&req).await, Err(AuthError::LoginMismatch));

        s.repo.revoke_sync(&s.b.id());
        let h = sign_request(&s.b, &s.a.id(), "POST", "/p", b"", now).unwrap();
        assert_eq!(s.v.verify(&plain(&h)).await, Err(AuthError::Revoked), "revoke is immediate");
    }

    #[test]
    fn header_roundtrip() {
        let me = Identity::generate().unwrap();
        let h = sign_with_nonce(&me, "0011223344556677", "GET", "/x", b"", 42, "abcdefabcdefabcd");
        let p = parse_header(&h).unwrap();
        assert_eq!((p.device, p.ts, p.nonce, p.sig.len()), (me.id().as_str(), 42, "abcdefabcdefabcd", 64));
        assert!(parse_header("Device ").is_none());
        assert!(parse_header(&h.replace("Device ", "device ")).is_none());
    }
}
