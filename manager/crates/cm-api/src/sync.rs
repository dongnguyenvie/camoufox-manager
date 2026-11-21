//! The sync listener (D9, phase 16): a separate HTTP server on `127.0.0.1:CM_SYNC_PORT`
//! (default 7879) that peers reach through `tailscale serve`, or directly on the 100.x IP in the
//! fallback mode. It **never** serves the UI or `/api`: only `/sync/v1/…`.
//!
//! Layers, outermost first:
//! 1. **Source policy**: the TCP peer must be loopback or Tailscale (100.64.0.0/10,
//!    fd7a:115c:a1e0::/48), else 403. Requests with an `Origin` header (browsers) → 403.
//! 2. **Device signature** on every route except `POST /sync/v1/pair`: the body is buffered
//!    (≤ [`MAX_BODY`]) and checked by [`cm_sync::Verifier`]; handlers get [`AuthedDevice`].
//!
//! | route | auth | |
//! |---|---|---|
//! | `POST /sync/v1/pair` | one-time code + key possession | B registers its key, gets A's (see `cm_sync::pairing`) |
//! | `GET/POST /sync/v1/hello` | signed | ping: `{deviceId, name, you, time, protocol}` |
//!
//! Phase 17 adds its routes in [`signed_routes`] (one `.merge(...)`), reading
//! `Extension<AuthedDevice>` for the caller.

// Handlers return an early `Response` as the error: it is built once per request, size is irrelevant.
#![allow(clippy::result_large_err)]

use super::Shared;
use anyhow::{Context, Result};
use axum::{
    Extension, Json, Router,
    body::{Body, to_bytes},
    extract::{ConnectInfo, DefaultBodyLimit, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use cm_core::Device;
use cm_sync::{DeviceHub, Incoming, PairRequest, policy};
use serde_json::json;
use std::{net::SocketAddr, sync::Arc};

/// Largest signed body (phase 17 chunks are 4–8 MiB).
pub const MAX_BODY: usize = 16 * 1024 * 1024;

/// The authenticated caller of a signed sync route.
#[derive(Clone, Debug)]
pub struct AuthedDevice(pub Device);

fn refuse(status: StatusCode, code: &str, message: &str) -> Response {
    (status, Json(json!({ "error": message, "code": code }))).into_response()
}

fn hub(m: &Shared) -> Result<&Arc<DeviceHub>, Response> {
    m.device_hub().ok_or_else(|| refuse(StatusCode::SERVICE_UNAVAILABLE, "sync_unavailable", "device sync is not available on this device"))
}

fn peer(req: &Request) -> Option<SocketAddr> {
    req.extensions().get::<ConnectInfo<SocketAddr>>().map(|c| c.0)
}

/// `Tailscale-User-Login`, trusted only when the connection comes from loopback (i.e. from
/// `tailscale serve`); a direct tailnet peer could set it to anything.
fn tailscale_login(h: &HeaderMap, from: Option<SocketAddr>) -> Option<String> {
    let lo = from.is_some_and(|a| policy::classify(a.ip()) == policy::Source::Loopback);
    if !lo {
        return None;
    }
    h.get("tailscale-user-login").and_then(|v| v.to_str().ok()).filter(|s| !s.is_empty() && s.len() <= 200).map(str::to_owned)
}

async fn source_guard(req: Request, next: Next) -> Response {
    match peer(&req) {
        Some(a) if policy::allowed(a.ip()) => {}
        Some(a) => return refuse(StatusCode::FORBIDDEN, "source_refused", &policy::refusal(a.ip())),
        None => return refuse(StatusCode::FORBIDDEN, "source_refused", "unknown source address"),
    }
    if req.headers().contains_key(header::ORIGIN) {
        return refuse(StatusCode::FORBIDDEN, "forbidden_origin", "browsers may not call the sync listener");
    }
    next.run(req).await
}

async fn device_auth(State(m): State<Shared>, req: Request, next: Next) -> Response {
    let hub = match hub(&m) {
        Ok(h) => h.clone(),
        Err(r) => return r,
    };
    let from = peer(&req);
    let (parts, body) = req.into_parts();
    let Ok(bytes) = to_bytes(body, MAX_BODY).await else {
        return refuse(StatusCode::PAYLOAD_TOO_LARGE, "too_large", "request body too large");
    };
    let path_and_query = parts.uri.path_and_query().map(|p| p.as_str()).unwrap_or("/").to_owned();
    let login = tailscale_login(&parts.headers, from);
    let incoming = Incoming {
        method: parts.method.as_str(),
        path_and_query: &path_and_query,
        authorization: parts.headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()),
        body: &bytes,
        tailscale_login: login.as_deref(),
        source: from.map(|a| a.ip().to_string()),
    };
    match hub.verify(&incoming).await {
        Ok(device) => {
            let mut req = Request::from_parts(parts, Body::from(bytes));
            req.extensions_mut().insert(AuthedDevice(device));
            next.run(req).await
        }
        Err(e) => refuse(StatusCode::UNAUTHORIZED, e.code(), &e.to_string()),
    }
}

async fn pair(
    State(m): State<Shared>,
    ConnectInfo(from): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let hub = match hub(&m) {
        Ok(h) => h.clone(),
        Err(r) => return r,
    };
    let Ok(req) = serde_json::from_slice::<PairRequest>(&body) else {
        return refuse(StatusCode::BAD_REQUEST, "pair_invalid", "invalid pairing request");
    };
    match hub.accept_pair(req, tailscale_login(&headers, Some(from)), Some(from.ip().to_string())).await {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => {
            let status = if matches!(e, cm_sync::PairRefusal::Code(_)) { StatusCode::UNAUTHORIZED } else { StatusCode::BAD_REQUEST };
            refuse(status, e.code(), &e.to_string())
        }
    }
}

async fn hello(State(m): State<Shared>, Extension(AuthedDevice(d)): Extension<AuthedDevice>) -> Response {
    match hub(&m) {
        Ok(h) => Json(h.hello_for(&d)).into_response(),
        Err(r) => r,
    }
}

async fn not_found() -> Response {
    refuse(StatusCode::NOT_FOUND, "not_found", "not a sync route")
}

/// Routes that require a device signature. Phase 17: merge the sync-engine routes here.
fn signed_routes() -> Router<Shared> {
    Router::new().route("/sync/v1/hello", get(hello).post(hello)).merge(super::sync_peer::routes())
}

/// The whole sync listener app (state applied). Serve it with connect info:
/// `axum::serve(l, router(m).into_make_service_with_connect_info::<SocketAddr>())`.
pub fn router(m: Shared) -> Router {
    let signed = signed_routes().layer(middleware::from_fn_with_state(m.clone(), device_auth));
    Router::new()
        .route("/sync/v1/pair", post(pair))
        .merge(signed)
        .fallback(not_found)
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .layer(middleware::from_fn(source_guard))
        .with_state(m)
}

/// Bind the sync listener on `addr` and serve it in the background.
pub async fn bind(m: Shared, addr: SocketAddr) -> Result<SocketAddr> {
    let listener = tokio::net::TcpListener::bind(addr).await.with_context(|| format!("cannot listen on {addr} for device sync"))?;
    let local = listener.local_addr()?;
    if let Some(h) = m.device_hub() {
        h.add_listener(local);
    }
    let app = router(m).into_make_service_with_connect_info::<SocketAddr>();
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("device sync listener {local} stopped: {e}");
        }
    });
    Ok(local)
}

/// Start the sync listener(s) for this process: `127.0.0.1:<sync_port>`, plus the tailnet-IP
/// fallback when the user chose it earlier and Tailscale is up. Failures are printed and leave
/// device sync off; the app is unaffected.
pub async fn start(m: Shared) {
    let Some(hub) = m.device_hub().cloned() else { return };
    let port = hub.config().sync_port;
    match bind(m.clone(), SocketAddr::from(([127, 0, 0, 1], port))).await {
        Ok(a) => println!("Device sync listener: http://{a} (reachable from your tailnet only after \"Make reachable\")"),
        Err(e) => {
            eprintln!("Device sync disabled: {e:#}");
            return;
        }
    }
    if hub.tailnet_fallback_chosen().await {
        match hub.tailnet_bind_addr().await {
            Ok(addr) => match bind(m, addr).await {
                Ok(a) => println!("Device sync also on http://{a} (plain HTTP inside Tailscale's WireGuard)"),
                Err(e) => eprintln!("Device sync tailnet listener not started: {e:#}"),
            },
            Err(e) => eprintln!("Device sync tailnet listener not started: {e:#}"),
        }
    }
}
