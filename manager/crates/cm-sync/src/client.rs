//! Signed HTTP client for one paired peer, with connection failures turned into plain advice.
//!
//! ```ignore
//! let peer = hub.client(&device_id).await?;              // or PeerClient::new(...)
//! let hello: Hello = peer.get_json("/sync/v1/hello").await?;
//! let resp = peer.send(Method::POST, "/sync/v1/x", body, Some("application/octet-stream")).await?;
//! ```
//!
//! Every request carries `Authorization: Device …` ([`crate::auth`]) for the peer's device id.
//! Non-2xx answers become [`PeerError::Status`] with the peer's `{error, code}` message.

use crate::{auth::sign_request, identity::Identity};
use cm_core::Clock;
use reqwest::{Method, StatusCode};
use serde::{Serialize, de::DeserializeOwned};
use std::{sync::Arc, time::Duration};

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(6);
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// A failed peer request, phrased for the user.
#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error(
        "Cannot find {host}. Check that MagicDNS is on in Tailscale and that both machines are logged in, then run `tailscale ping {host}`."
    )]
    Dns { host: String },
    #[error(
        "{host} is reachable but nothing answers on port {port}. Open Camoufox Manager there and make it reachable (Devices → Make reachable)."
    )]
    Refused { host: String, port: u16 },
    #[error("{host} did not answer in time. Check the connection with `tailscale ping {host}`.")]
    Timeout { host: String },
    #[error("Cannot reach {host}. Check that both machines are on Tailscale with `tailscale ping {host}`. ({detail})")]
    Unreachable { host: String, detail: String },
    #[error(
        "Secure connection to {host} failed ({detail}). HTTPS certificates may be off for your tailnet: enable them in the Tailscale admin console (DNS → HTTPS Certificates)."
    )]
    Tls { host: String, detail: String },
    #[error("{host} refused this device: {message}. To use it again, pair with a new link from {host}.")]
    Unauthorized { host: String, code: String, message: String },
    #[error("{message} ({host}, HTTP {status})")]
    Status { host: String, status: u16, code: String, message: String },
    #[error("unexpected answer from {host}: {detail}")]
    Protocol { host: String, detail: String },
    #[error("{0}")]
    Local(String),
}

impl PeerError {
    /// Stable code for the UI.
    pub fn code(&self) -> &str {
        match self {
            PeerError::Dns { .. } => "peer_dns",
            PeerError::Refused { .. } => "peer_refused",
            PeerError::Timeout { .. } => "peer_timeout",
            PeerError::Unreachable { .. } => "peer_unreachable",
            PeerError::Tls { .. } => "peer_tls",
            PeerError::Unauthorized { code, .. } | PeerError::Status { code, .. } => code,
            PeerError::Protocol { .. } => "peer_protocol",
            PeerError::Local(_) => "local",
        }
    }

    /// From a transport error of a request to `host:port`.
    pub fn from_reqwest(e: &reqwest::Error, host: &str, port: u16) -> PeerError {
        let mut chain = String::new();
        let mut src: Option<&dyn std::error::Error> = Some(e);
        while let Some(s) = src {
            chain.push_str(&s.to_string().to_lowercase());
            chain.push(' ');
            src = s.source();
        }
        let host = host.to_owned();
        if e.is_timeout() {
            PeerError::Timeout { host }
        } else if chain.contains("dns")
            || chain.contains("lookup")
            || chain.contains("nodename nor servname")
            || chain.contains("name or service")
        {
            PeerError::Dns { host }
        } else if chain.contains("refused") {
            PeerError::Refused { host, port }
        } else if chain.contains("certificate") || chain.contains("tls") || chain.contains("handshake") {
            PeerError::Tls { host, detail: first_line(&chain) }
        } else {
            PeerError::Unreachable { host, detail: first_line(&chain) }
        }
    }
}

fn first_line(s: &str) -> String {
    s.trim().chars().take(160).collect()
}

/// Map a non-2xx answer (`{error, code}` JSON when it comes from a Camoufox Manager).
pub(crate) fn status_error(host: &str, status: StatusCode, body: &[u8]) -> PeerError {
    let v: serde_json::Value = serde_json::from_slice(body).unwrap_or_default();
    let message = v
        .get("error")
        .and_then(|m| m.as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| status.canonical_reason().unwrap_or("error").to_owned());
    let code = v.get("code").and_then(|m| m.as_str()).unwrap_or("").to_owned();
    if status == StatusCode::UNAUTHORIZED {
        PeerError::Unauthorized { host: host.to_owned(), code: if code.is_empty() { "unauthorized".into() } else { code }, message }
    } else {
        PeerError::Status { host: host.to_owned(), status: status.as_u16(), code, message }
    }
}

/// Shared reqwest client for sync traffic (no proxy: tailnet traffic must not go through one).
pub fn http_client(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder().no_proxy().connect_timeout(CONNECT_TIMEOUT).timeout(timeout).build().unwrap_or_default()
}

/// Signed client for one peer.
#[derive(Clone)]
pub struct PeerClient {
    endpoint: reqwest::Url,
    host: String,
    port: u16,
    audience: String,
    me: Arc<Identity>,
    clock: Arc<dyn Clock>,
    http: reqwest::Client,
}

impl PeerClient {
    /// `endpoint` is the peer's base URL (`https://host:port`), `audience` its device id.
    pub fn new(
        endpoint: &str,
        audience: &str,
        me: Arc<Identity>,
        clock: Arc<dyn Clock>,
        timeout: Duration,
    ) -> Result<PeerClient, PeerError> {
        let url = reqwest::Url::parse(endpoint).map_err(|_| PeerError::Local(format!("invalid device address {endpoint}")))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(PeerError::Local(format!("invalid device address {endpoint}")));
        }
        let host = url.host_str().unwrap_or_default().trim_matches(['[', ']']).to_owned();
        let port = url.port_or_known_default().unwrap_or(443);
        Ok(PeerClient { endpoint: url, host, port, audience: audience.to_owned(), me, clock, http: http_client(timeout) })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn endpoint(&self) -> &str {
        self.endpoint.as_str().trim_end_matches('/')
    }

    pub fn audience(&self) -> &str {
        &self.audience
    }

    /// Send a signed request; returns the response only when it is 2xx (streamable by the caller).
    pub async fn send(
        &self,
        method: Method,
        path_and_query: &str,
        body: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<reqwest::Response, PeerError> {
        if !path_and_query.starts_with("/sync/") {
            return Err(PeerError::Local("peer requests must target /sync/…".into()));
        }
        let url = self.endpoint.join(path_and_query).map_err(|e| PeerError::Local(e.to_string()))?;
        let auth = sign_request(&self.me, &self.audience, method.as_str(), path_and_query, &body, self.clock.now())
            .map_err(|e| PeerError::Local(e.to_string()))?;
        let mut rb = self.http.request(method, url).header(reqwest::header::AUTHORIZATION, auth);
        if let Some(ct) = content_type {
            rb = rb.header(reqwest::header::CONTENT_TYPE, ct);
        }
        let resp = rb.body(body).send().await.map_err(|e| PeerError::from_reqwest(&e, &self.host, self.port))?;
        if resp.status().is_success() {
            return Ok(resp);
        }
        let status = resp.status();
        let bytes = resp.bytes().await.unwrap_or_default();
        Err(status_error(&self.host, status, &bytes))
    }

    pub async fn get_json<T: DeserializeOwned>(&self, path_and_query: &str) -> Result<T, PeerError> {
        let r = self.send(Method::GET, path_and_query, Vec::new(), None).await?;
        r.json().await.map_err(|e| PeerError::Protocol { host: self.host.clone(), detail: e.to_string() })
    }

    pub async fn post_json<B: Serialize, T: DeserializeOwned>(&self, path_and_query: &str, body: &B) -> Result<T, PeerError> {
        let bytes = serde_json::to_vec(body).map_err(|e| PeerError::Local(e.to_string()))?;
        let r = self.send(Method::POST, path_and_query, bytes, Some("application/json")).await?;
        r.json().await.map_err(|e| PeerError::Protocol { host: self.host.clone(), detail: e.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_mapping() {
        let e = status_error("mac", StatusCode::UNAUTHORIZED, br#"{"error":"the calling device was revoked","code":"device_revoked"}"#);
        assert_eq!(e.code(), "device_revoked");
        assert!(e.to_string().contains("pair with a new link"));
        let e = status_error("mac", StatusCode::FORBIDDEN, b"nope");
        assert!(matches!(e, PeerError::Status { status: 403, .. }));
    }

    #[tokio::test]
    async fn refused_connection_gives_advice() {
        // Bind then drop: nothing listens on that port any more.
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let me = Arc::new(Identity::generate().unwrap());
        let c =
            PeerClient::new(&format!("http://127.0.0.1:{port}"), "0011223344556677", me, Arc::new(cm_core::SystemClock), REQUEST_TIMEOUT)
                .unwrap();
        let e = c.get_json::<serde_json::Value>("/sync/v1/hello").await.unwrap_err();
        assert!(matches!(e, PeerError::Refused { .. }), "{e:?}");
        assert!(e.to_string().contains("Make reachable"));
        let e = PeerClient::new("ftp://x", "0", Arc::new(Identity::generate().unwrap()), Arc::new(cm_core::SystemClock), REQUEST_TIMEOUT);
        assert!(e.is_err());
    }

    #[tokio::test]
    async fn dns_failure_mentions_tailscale_ping() {
        let me = Arc::new(Identity::generate().unwrap());
        let c = PeerClient::new("http://no-such-host.invalid:1", "0011223344556677", me, Arc::new(cm_core::SystemClock), REQUEST_TIMEOUT)
            .unwrap();
        let e = c.get_json::<serde_json::Value>("/sync/v1/hello").await.unwrap_err();
        assert!(e.to_string().contains("tailscale ping no-such-host.invalid"), "{e}");
    }
}
