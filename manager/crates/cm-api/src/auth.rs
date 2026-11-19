//! API-key middleware for the `/api` routes (phase 12). Static UI assets are never checked.
//!
//! The server only listens on 127.0.0.1. The rule, in order (first match wins):
//!
//! 1. **Key management** (`/api/api-keys…`): UI only. A caller holding a key cannot mint or revoke
//!    keys, otherwise revoking one would be pointless. Not UI → 403 `ui_only`.
//! 2. **Request carries `X-API-Key`**: it must be a valid, non-revoked key, whoever sends it
//!    (UI or not). Valid → pass and record "last used"; otherwise 401 `invalid_api_key`.
//! 3. **UI request** (same-origin, no key needed): `Host` is a loopback name (`127.0.0.1`,
//!    `localhost`, `[::1]`, which defeats DNS rebinding) and the browser says it comes from that
//!    same origin (`Origin`, else `Referer`, else `Sec-Fetch-Site: same-origin`). Always passes.
//! 4. **Foreign browser origin** (an `Origin` header that is not this server, e.g. a web page
//!    trying to call the local API): 403 `forbidden_origin`, whatever the settings.
//! 5. **Other clients** (CLI, scripts, agents; no Origin): pass, unless the setting
//!    "require API key for non-UI clients" is on → 401 `api_key_required`.
//!
//! This separates well-behaved clients; it is not a defence against a hostile local process
//! (which can forge `Origin`), which could read the database anyway.

use super::Shared;
use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use cm_service::settings::KeyError;
use serde_json::json;

const KEY_HEADER: &str = "x-api-key";
const KEY_MANAGEMENT: &str = "/api/api-keys";

fn refuse(status: StatusCode, code: &str, message: &str) -> Response {
    (status, Json(json!({ "error": message, "code": code }))).into_response()
}

fn header_str(h: &HeaderMap, name: impl header::AsHeaderName) -> Option<&str> {
    h.get(name).and_then(|v| v.to_str().ok())
}

/// `http://127.0.0.1:7878/x` → `127.0.0.1:7878`.
fn authority(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    rest.split(['/', '?', '#']).next().unwrap_or("")
}

fn loopback(host: &str) -> bool {
    let name = if host.starts_with('[') {
        host.split_once(']').map_or(host, |(h, _)| &host[..=h.len()])
    } else {
        host.split(':').next().unwrap_or("")
    };
    matches!(name, "127.0.0.1" | "localhost" | "[::1]")
}

/// Does the request come from the embedded UI (rule 3)?
pub(crate) fn is_ui_request(h: &HeaderMap) -> bool {
    let Some(host) = header_str(h, header::HOST) else { return false };
    if !loopback(host) {
        return false;
    }
    match header_str(h, header::ORIGIN) {
        Some(origin) => authority(origin) == host,
        None => {
            header_str(h, header::REFERER).is_some_and(|r| authority(r) == host) || header_str(h, "sec-fetch-site") == Some("same-origin")
        }
    }
}

async fn check(State(m): State<Shared>, req: Request, next: Next) -> Response {
    let ui = is_ui_request(req.headers());
    if req.uri().path().starts_with(KEY_MANAGEMENT) && !ui {
        return refuse(StatusCode::FORBIDDEN, "ui_only", "API keys can only be managed from the app window");
    }
    if let Some(key) = req.headers().get(KEY_HEADER) {
        let verdict = match key.to_str() {
            Ok(k) => m.verify_api_key(k).await.map(|_| ()),
            Err(_) => Err(KeyError::Malformed),
        };
        return match verdict {
            Ok(()) => next.run(req).await,
            Err(e) => {
                let why = match e {
                    KeyError::Revoked => "this API key was revoked",
                    KeyError::Malformed | KeyError::Unknown => "unknown or malformed API key",
                };
                refuse(StatusCode::UNAUTHORIZED, "invalid_api_key", why)
            }
        };
    }
    if ui {
        return next.run(req).await;
    }
    if req.headers().contains_key(header::ORIGIN) {
        return refuse(StatusCode::FORBIDDEN, "forbidden_origin", "cross-origin browser requests to the local API are not allowed");
    }
    if m.require_api_key().await {
        return refuse(
            StatusCode::UNAUTHORIZED,
            "api_key_required",
            "this app requires an API key: send X-API-Key (create one in Settings)",
        );
    }
    next.run(req).await
}

/// Wrap the `/api` routes with the API-key check.
pub fn apply(api: Router<Shared>, m: Shared) -> Router<Shared> {
    api.layer(axum::middleware::from_fn_with_state(m, check))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn hm(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, HeaderValue::from_static(v));
        }
        h
    }

    #[test]
    fn ui_detection() {
        let host = ("host", "127.0.0.1:7878");
        assert!(is_ui_request(&hm(&[host, ("origin", "http://127.0.0.1:7878")])));
        assert!(is_ui_request(&hm(&[host, ("referer", "http://127.0.0.1:7878/#/settings")])));
        assert!(is_ui_request(&hm(&[host, ("sec-fetch-site", "same-origin")])));
        assert!(!is_ui_request(&hm(&[host])), "curl / CLI");
        assert!(!is_ui_request(&hm(&[host, ("origin", "https://evil.example")])));
        assert!(!is_ui_request(&hm(&[("host", "evil.example"), ("origin", "http://evil.example")])), "DNS rebinding");
        assert!(is_ui_request(&hm(&[("host", "localhost:1"), ("origin", "http://localhost:1")])));
        assert!(is_ui_request(&hm(&[("host", "[::1]:1"), ("origin", "http://[::1]:1")])));
    }
}
