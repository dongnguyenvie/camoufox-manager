//! Devices routes for the UI (phase 16). Everything that reveals a pairing code or changes
//! Tailscale, the listener or the device list is **UI only** (same rule as API keys, see `auth`).
//!
//! | route | UI only | |
//! |---|---|---|
//! | `GET /api/devices/status[?fresh=1]` | | Tailscale + reachability + this device |
//! | `GET /api/devices/pairing` | yes | my `cmsync://` link + code + expiry (new code if none is live) |
//! | `POST /api/devices/pairing/regenerate` | yes | replace the code |
//! | `POST /api/devices/serve` `{command, confirm: true}` | yes | run the shown `tailscale serve` command |
//! | `POST /api/devices/tailnet-ip` `{confirm: true}` | yes | fallback: also listen on the 100.x IP (plain HTTP in WireGuard) |
//! | `POST /api/devices/pair` `{url}` | yes | pair with the device behind a pasted link |
//! | `GET /api/devices` | | paired devices |
//! | `DELETE /api/devices/{id}[?forget=1]` | yes | revoke (immediate) / remove the row |
//! | `POST /api/devices/{id}/hello` | | signed ping of that device |

// Handlers return an early `Response` as the error: it is built once per request, size is irrelevant.
#![allow(clippy::result_large_err)]

use super::{ApiError, Shared, auth::is_ui_request, ok};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use cm_sync::{DeviceHub, PeerError};
use cm_tailscale::Confirmed;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

type Res = Result<Response, Response>;

fn refuse(status: StatusCode, code: &str, message: &str) -> Response {
    (status, Json(json!({ "error": message, "code": code }))).into_response()
}

fn ui_only(h: &HeaderMap) -> Result<(), Response> {
    if is_ui_request(h) { Ok(()) } else { Err(refuse(StatusCode::FORBIDDEN, "ui_only", "devices can only be managed from the app window")) }
}

fn hub(m: &Shared) -> Result<Arc<DeviceHub>, Response> {
    m.device_hub().cloned().ok_or_else(|| refuse(StatusCode::SERVICE_UNAVAILABLE, "sync_unavailable", "device sync is not available"))
}

/// Peer failures → 502 with the plain advice and a code; anything else → the usual mapping.
fn fail(e: anyhow::Error) -> Response {
    if let Some(p) = e.chain().find_map(|c| c.downcast_ref::<PeerError>()) {
        return refuse(StatusCode::BAD_GATEWAY, p.code(), &format!("{e:#}"));
    }
    ApiError::from(e).into_response()
}

fn json_ok<T: serde::Serialize>(v: T) -> Res {
    ok(v).map(IntoResponse::into_response).map_err(IntoResponse::into_response)
}

#[derive(Deserialize)]
struct Fresh {
    #[serde(default)]
    fresh: Option<String>,
}

async fn status(State(m): State<Shared>, Query(q): Query<Fresh>) -> Res {
    let h = hub(&m)?;
    json_ok(h.status(q.fresh.is_some()).await)
}

async fn pairing(State(m): State<Shared>, headers: HeaderMap) -> Res {
    ui_only(&headers)?;
    json_ok(hub(&m)?.offer().await.map_err(fail)?)
}

async fn regenerate(State(m): State<Shared>, headers: HeaderMap) -> Res {
    ui_only(&headers)?;
    json_ok(hub(&m)?.regenerate().await.map_err(fail)?)
}

#[derive(Deserialize)]
struct ServeBody {
    command: String,
    #[serde(default)]
    confirm: bool,
}

async fn serve(State(m): State<Shared>, headers: HeaderMap, Json(b): Json<ServeBody>) -> Res {
    ui_only(&headers)?;
    if !b.confirm {
        return Err(refuse(StatusCode::BAD_REQUEST, "confirm_required", "confirm the command first"));
    }
    let out = hub(&m)?.confirm_serve(&b.command, Confirmed::by_user()).await.map_err(fail)?;
    json_ok(json!({ "ok": true, "output": out }))
}

#[derive(Deserialize)]
struct ConfirmBody {
    #[serde(default)]
    confirm: bool,
}

async fn tailnet_ip(State(m): State<Shared>, headers: HeaderMap, Json(b): Json<ConfirmBody>) -> Res {
    ui_only(&headers)?;
    if !b.confirm {
        return Err(refuse(StatusCode::BAD_REQUEST, "confirm_required", "confirm first"));
    }
    let h = hub(&m)?;
    let addr = h.tailnet_bind_addr().await.map_err(fail)?;
    if !h.listeners().contains(&addr) {
        crate::sync::bind(m.clone(), addr).await.map_err(fail)?;
    }
    h.set_tailnet_fallback(true).await.map_err(fail)?;
    json_ok(json!({ "ok": true, "address": addr.to_string() }))
}

#[derive(Deserialize)]
struct PairBody {
    url: String,
}

async fn pair(State(m): State<Shared>, headers: HeaderMap, Json(b): Json<PairBody>) -> Res {
    ui_only(&headers)?;
    json_ok(hub(&m)?.pair_with_url(&b.url).await.map_err(fail)?)
}

async fn list(State(m): State<Shared>) -> Res {
    json_ok(json!({ "devices": hub(&m)?.list().await.map_err(fail)? }))
}

#[derive(Deserialize)]
struct Forget {
    #[serde(default)]
    forget: Option<String>,
}

async fn revoke(State(m): State<Shared>, headers: HeaderMap, Path(id): Path<String>, Query(q): Query<Forget>) -> Res {
    ui_only(&headers)?;
    let h = hub(&m)?;
    let done = if q.forget.is_some() { h.forget(&id).await } else { h.revoke(&id).await }.map_err(fail)?;
    if !done && h.get(&id).await.map_err(fail)?.is_none() {
        return Err(refuse(StatusCode::NOT_FOUND, "not_found", "device not found"));
    }
    json_ok(json!({ "ok": true }))
}

async fn hello(State(m): State<Shared>, Path(id): Path<String>) -> Res {
    json_ok(hub(&m)?.hello(&id).await.map_err(fail)?)
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/devices", get(list))
        .route("/api/devices/status", get(status))
        .route("/api/devices/pairing", get(pairing))
        .route("/api/devices/pairing/regenerate", post(regenerate))
        .route("/api/devices/serve", post(serve))
        .route("/api/devices/tailnet-ip", post(tailnet_ip))
        .route("/api/devices/pair", post(pair))
        .route("/api/devices/{id}", delete(revoke))
        .route("/api/devices/{id}/hello", post(hello))
}
