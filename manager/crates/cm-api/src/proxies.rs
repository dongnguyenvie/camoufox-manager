//! Proxy routes: library list, check, assign to a profile. Phase 2 adds CRUD/import here.

use super::{ApiResult, Shared, ok};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use cm_core::ProxyId;
use serde::Deserialize;
use serde_json::json;

async fn proxies(State(m): State<Shared>) -> ApiResult {
    ok(json!({ "proxies": m.proxies().await? }))
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

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/proxies", get(proxies))
        .route("/api/proxies/{id}/check", post(check_proxy))
        .route("/api/profiles/{id}/assign", post(assign))
}
