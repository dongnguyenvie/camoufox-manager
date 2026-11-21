//! Device-sync endpoints for the UI (phase 17). All of them are **UI only** (the app window):
//! copying profiles between machines moves login sessions and must be an explicit user action.
//!
//! | route | |
//! |---|---|
//! | `GET /api/sync/devices/{id}/plan?direction=pull\|push` | diff of the two machines: one row per source profile (status, size, allowed actions, blocked reason) |
//! | `POST /api/sync/devices/{id}/start` | `{direction, items:[{profileId, action}], includeProxy, includeProxyPassword, acknowledged}` → job |
//! | `GET /api/sync/jobs` | recent jobs, newest first |
//! | `GET /api/sync/jobs/{id}` | progress per profile (bytes, state) |
//! | `POST /api/sync/jobs/{id}/cancel` | stop after the current piece |
//!
//! `start` is refused unless `acknowledged` is true (the "same logins on both machines" warning).

// Handlers return an early `Response` as the error: it is built once per request, size is irrelevant.
#![allow(clippy::result_large_err)]

use super::{Shared, auth::is_ui_request, ok};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use cm_core::CmError;
use cm_service::sync::{StartRequest, SyncError};
use cm_sync::{Direction, PeerError};
use serde::Deserialize;
use serde_json::json;

type Res = Result<Response, Response>;

pub(crate) fn refuse(status: StatusCode, code: &str, message: &str) -> Response {
    (status, Json(json!({ "error": message, "code": code }))).into_response()
}

/// Any failure → `{error, code}` with a fitting status (shared with the peer routes).
pub(crate) fn fail(e: anyhow::Error) -> Response {
    if let Some(s) = e.chain().find_map(|c| c.downcast_ref::<SyncError>()) {
        let status = StatusCode::from_u16(s.status).unwrap_or(StatusCode::BAD_REQUEST);
        let mut body = json!({ "error": s.message, "code": s.code });
        if let Some(r) = s.received {
            body["received"] = json!(r);
        }
        return (status, Json(body)).into_response();
    }
    if let Some(p) = e.chain().find_map(|c| c.downcast_ref::<PeerError>()) {
        return refuse(StatusCode::BAD_GATEWAY, p.code(), &format!("{e:#}"));
    }
    let (status, code) = match e.chain().find_map(|c| c.downcast_ref::<CmError>()) {
        Some(CmError::NotFound(_)) => (StatusCode::NOT_FOUND, "not_found"),
        Some(CmError::Conflict(_)) => (StatusCode::CONFLICT, "conflict"),
        Some(CmError::NotImplemented(_)) => (StatusCode::NOT_IMPLEMENTED, "not_implemented"),
        _ => (StatusCode::BAD_REQUEST, "invalid"),
    };
    refuse(status, code, &format!("{e:#}"))
}

fn ui_only(h: &HeaderMap) -> Result<(), Response> {
    if is_ui_request(h) {
        Ok(())
    } else {
        Err(refuse(StatusCode::FORBIDDEN, "ui_only", "device sync can only be used from the app window"))
    }
}

fn json_ok<T: serde::Serialize>(v: T) -> Res {
    ok(v).map(IntoResponse::into_response).map_err(|e| fail(e.0))
}

#[derive(Deserialize)]
struct PlanQuery {
    direction: Direction,
}

async fn plan(State(m): State<Shared>, headers: HeaderMap, Path(id): Path<String>, Query(q): Query<PlanQuery>) -> Res {
    ui_only(&headers)?;
    json_ok(m.sync_plan(&id, q.direction).await.map_err(fail)?)
}

async fn start(State(m): State<Shared>, headers: HeaderMap, Path(id): Path<String>, Json(req): Json<StartRequest>) -> Res {
    ui_only(&headers)?;
    json_ok(m.sync_start(&id, req).await.map_err(fail)?)
}

async fn jobs(State(m): State<Shared>, headers: HeaderMap) -> Res {
    ui_only(&headers)?;
    json_ok(json!({ "jobs": m.sync_jobs() }))
}

async fn job(State(m): State<Shared>, headers: HeaderMap, Path(id): Path<String>) -> Res {
    ui_only(&headers)?;
    match m.sync_job(&id) {
        Some(j) => json_ok(j),
        None => Err(refuse(StatusCode::NOT_FOUND, "not_found", "no such sync job")),
    }
}

async fn cancel(State(m): State<Shared>, headers: HeaderMap, Path(id): Path<String>) -> Res {
    ui_only(&headers)?;
    if m.sync_job(&id).is_none() {
        return Err(refuse(StatusCode::NOT_FOUND, "not_found", "no such sync job"));
    }
    json_ok(json!({ "ok": true, "cancelling": m.sync_cancel(&id) }))
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/sync/devices/{id}/plan", get(plan))
        .route("/api/sync/devices/{id}/start", post(start))
        .route("/api/sync/jobs", get(jobs))
        .route("/api/sync/jobs/{id}", get(job))
        .route("/api/sync/jobs/{id}/cancel", post(cancel))
}
