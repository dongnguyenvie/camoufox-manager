//! Profile routes: overview, create/delete, start/stop, reveal data folder.
//! Phase 4 adds edit routes here.

use super::{ApiResult, Shared, ok};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use cm_core::NewProfile;
use serde_json::json;

async fn overview(State(m): State<Shared>) -> ApiResult {
    ok(m.overview().await)
}

async fn start(State(m): State<Shared>, Path(id): Path<String>) -> ApiResult {
    ok(m.start(&id).await?)
}

async fn stop(State(m): State<Shared>, Path(id): Path<String>) -> ApiResult {
    m.stop(&id).await?;
    ok(json!({ "ok": true }))
}

async fn create_profile(State(m): State<Shared>, Json(b): Json<NewProfile>) -> ApiResult {
    ok(m.create_profile(b).await?)
}

async fn delete_profile(State(m): State<Shared>, Path(id): Path<String>) -> ApiResult {
    m.delete_profile(&id).await?;
    ok(json!({ "ok": true }))
}

async fn reveal(State(m): State<Shared>, Path(id): Path<String>) -> ApiResult {
    std::process::Command::new("open").arg(m.profile_dir(&id).await?).spawn()?;
    ok(json!({ "ok": true }))
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/overview", get(overview))
        .route("/api/profiles", post(create_profile))
        .route("/api/profiles/{id}", delete(delete_profile))
        .route("/api/profiles/{id}/start", post(start))
        .route("/api/profiles/{id}/stop", post(stop))
        .route("/api/profiles/{id}/reveal", post(reveal))
}
