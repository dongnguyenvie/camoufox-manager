//! Profile routes: overview, create/delete, start/stop, reveal data folder.
//! Phase 4: `GET/PUT /api/profiles/{id}` (edit settings), `POST /api/profiles/{id}/regenerate-machine`.

use super::{ApiResult, Shared, ok};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use cm_core::NewProfile;
use cm_service::profiles::ProfileEdit;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, Default)]
struct RegenerateBody {
    /// New OS (`win`/`mac`/`lin`); default: keep the profile's OS.
    #[serde(default)]
    os: Option<String>,
}

async fn get_profile(State(m): State<Shared>, Path(id): Path<String>) -> ApiResult {
    ok(m.profile_view(&id).await?)
}

async fn update_profile(State(m): State<Shared>, Path(id): Path<String>, Json(b): Json<ProfileEdit>) -> ApiResult {
    ok(m.update_profile(&id, b).await?)
}

async fn regenerate_machine(State(m): State<Shared>, Path(id): Path<String>, body: Option<Json<RegenerateBody>>) -> ApiResult {
    let os = body.and_then(|Json(b)| b.os);
    ok(m.regenerate_machine(&id, os.as_deref()).await?)
}

async fn overview(State(m): State<Shared>) -> ApiResult {
    ok(m.overview().await)
}

async fn start(State(m): State<Shared>, Path(id): Path<String>, headers: HeaderMap) -> ApiResult {
    // Phase 8: the lease holder identifies itself; anyone else gets 409 while the profile is leased.
    let holder = headers.get("x-lease-holder").and_then(|v| v.to_str().ok());
    ok(m.start_as(&id, holder).await?)
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
        .route("/api/profiles/{id}", get(get_profile).put(update_profile).delete(delete_profile))
        .route("/api/profiles/{id}/regenerate-machine", post(regenerate_machine))
        .route("/api/profiles/{id}/start", post(start))
        .route("/api/profiles/{id}/stop", post(stop))
        .route("/api/profiles/{id}/reveal", post(reveal))
}
