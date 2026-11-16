//! Proxy routes: library CRUD, import, check, assign (one → many, list → profiles in order).
//! Responses never carry a password (only `hasPassword`).
//!
//! | route | |
//! |---|---|
//! | `GET /api/proxies` | library |
//! | `POST /api/proxies` | create `{name, scheme, host, port, username?, password?}` |
//! | `PUT /api/proxies/{id}` | replace; omit `password` to keep it, `clearPassword: true` to remove |
//! | `DELETE /api/proxies/{id}` | delete (unassigns from profiles) |
//! | `POST /api/proxies/import` | `{text}`; 400 `{rejected:[{line,code}]}` when any line is bad |
//! | `POST /api/proxies/{id}/check` | check through the relay |
//! | `POST /api/proxies/{id}/assign` | `{profileIds}` one proxy → many profiles |
//! | `POST /api/proxies/assign-sequence` | `{profileIds, proxyIds?, dryRun?}` |
//! | `POST /api/profiles/{id}/assign` | `{proxyId}` single profile |

use super::{ApiResult, Shared, ok};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use cm_core::{NewProxy, ProxyId, ProxyScheme};
use cm_service::proxies::SequenceRequest;
use serde::Deserialize;
use serde_json::json;

async fn proxies(State(m): State<Shared>) -> ApiResult {
    ok(json!({ "proxies": m.proxies().await? }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProxyBody {
    #[serde(default)]
    name: String,
    #[serde(default)]
    scheme: ProxyScheme,
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    #[serde(default)]
    clear_password: bool,
}

impl ProxyBody {
    fn split(self) -> (NewProxy, bool) {
        let p = NewProxy {
            name: self.name,
            scheme: self.scheme,
            host: self.host,
            port: self.port,
            username: self.username,
            password: self.password,
        };
        (p, self.clear_password)
    }
}

async fn create(State(m): State<Shared>, Json(b): Json<ProxyBody>) -> ApiResult {
    ok(m.create_proxy(b.split().0).await?)
}

async fn update(State(m): State<Shared>, Path(id): Path<ProxyId>, Json(b): Json<ProxyBody>) -> ApiResult {
    let (p, clear) = b.split();
    ok(m.update_proxy(id, p, clear).await?)
}

async fn delete(State(m): State<Shared>, Path(id): Path<ProxyId>) -> ApiResult {
    m.delete_proxy(id).await?;
    ok(json!({ "ok": true }))
}

#[derive(Deserialize)]
struct ImportBody {
    text: String,
}

async fn import(State(m): State<Shared>, Json(b): Json<ImportBody>) -> Result<Response, super::ApiError> {
    let r = m.import_proxies(&b.text).await?;
    let status = if r.rejected.is_empty() { StatusCode::OK } else { StatusCode::BAD_REQUEST };
    let mut body = serde_json::to_value(&r)?;
    if !r.rejected.is_empty() {
        body["error"] = json!(format!("{} line(s) are not valid proxies, nothing was imported", r.rejected.len()));
    }
    Ok((status, Json(body)).into_response())
}

async fn check_proxy(State(m): State<Shared>, Path(id): Path<ProxyId>) -> ApiResult {
    ok(m.check_proxy(id, true).await?)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssignBody {
    proxy_id: Option<ProxyId>,
}

async fn assign(State(m): State<Shared>, Path(id): Path<String>, Json(b): Json<AssignBody>) -> ApiResult {
    m.assign(&id, b.proxy_id).await?;
    ok(json!({ "ok": true }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssignManyBody {
    profile_ids: Vec<String>,
}

async fn assign_many(State(m): State<Shared>, Path(id): Path<ProxyId>, Json(b): Json<AssignManyBody>) -> ApiResult {
    ok(json!({ "results": m.assign_many(Some(id), &b.profile_ids).await? }))
}

async fn assign_sequence(State(m): State<Shared>, Json(b): Json<SequenceRequest>) -> ApiResult {
    ok(m.assign_sequence(b).await?)
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/proxies", get(proxies).post(create))
        .route("/api/proxies/import", post(import))
        .route("/api/proxies/assign-sequence", post(assign_sequence))
        .route("/api/proxies/{id}", axum::routing::put(update).delete(delete))
        .route("/api/proxies/{id}/check", post(check_proxy))
        .route("/api/proxies/{id}/assign", post(assign_many))
        .route("/api/profiles/{id}/assign", post(assign))
}
