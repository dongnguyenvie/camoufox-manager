//! Agent lease routes (phase 8):
//! `POST /api/profiles/{id}/lease {holder, ttlSecs}` · `POST …/lease/renew {holder, ttlSecs}` ·
//! `DELETE …/lease[?holder=name]` · `GET …/lease`. A start by a non-holder while leased is 409;
//! the holder sends `X-Lease-Holder: <name>` with `POST …/start`.

use super::{ApiResult, Shared, ok};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LeaseBody {
    holder: String,
    ttl_secs: u64,
}

#[derive(Deserialize)]
struct HolderQuery {
    holder: Option<String>,
}

async fn acquire(State(m): State<Shared>, Path(id): Path<String>, Json(b): Json<LeaseBody>) -> ApiResult {
    ok(m.lease_acquire(&id, &b.holder, b.ttl_secs).await?)
}

async fn renew(State(m): State<Shared>, Path(id): Path<String>, Json(b): Json<LeaseBody>) -> ApiResult {
    ok(m.lease_renew(&id, &b.holder, b.ttl_secs).await?)
}

async fn release(State(m): State<Shared>, Path(id): Path<String>, Query(q): Query<HolderQuery>) -> ApiResult {
    m.lease_release(&id, q.holder.as_deref()).await?;
    ok(json!({ "ok": true }))
}

async fn show(State(m): State<Shared>, Path(id): Path<String>) -> ApiResult {
    ok(json!({ "lease": m.lease_of(&id) }))
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/profiles/{id}/lease", get(show).post(acquire).delete(release))
        .route("/api/profiles/{id}/lease/renew", post(renew))
}
