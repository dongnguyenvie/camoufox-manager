//! Profile routes: overview, create/delete, start/stop, reveal data folder.
//! CM-3.1 trash: `GET /api/profiles/trash`, `POST /api/profiles/{id}/restore`, `DELETE /api/profiles/{id}/purge`,
//! `POST /api/profiles/trash/empty` (DELETE on a profile moves it to the trash).
//! Phase 4: `GET/PUT /api/profiles/{id}` (edit settings), `POST /api/profiles/{id}/regenerate-machine`.

use super::{ApiResult, Shared, ok};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::HeaderMap,
    routing::{delete, get, post},
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

async fn trash_list(State(m): State<Shared>) -> ApiResult {
    ok(m.trashed_profiles().await?)
}

async fn restore_profile(State(m): State<Shared>, Path(id): Path<String>) -> ApiResult {
    m.restore_profile(&id).await?;
    ok(json!({ "ok": true }))
}

async fn purge_profile(State(m): State<Shared>, Path(id): Path<String>) -> ApiResult {
    m.purge_profile(&id).await?;
    ok(json!({ "ok": true }))
}

async fn empty_trash(State(m): State<Shared>) -> ApiResult {
    ok(json!({ "purged": m.empty_trash().await? }))
}

/// CM-3.4: parse a Netscape `bookmarks.html` body into a bookmark list (nothing is stored).
async fn import_bookmarks(body: Bytes) -> ApiResult {
    let html = String::from_utf8_lossy(&body);
    ok(json!({ "bookmarks": cm_service::bookmarks::validate(cm_service::bookmarks::parse_netscape(&html))? }))
}

async fn reveal(State(m): State<Shared>, Path(id): Path<String>) -> ApiResult {
    std::process::Command::new(super::opener()).arg(m.profile_dir(&id).await?).spawn()?;
    ok(json!({ "ok": true }))
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/overview", get(overview))
        .route("/api/profiles", post(create_profile))
        .route("/api/profiles/trash", get(trash_list))
        .route("/api/profiles/trash/empty", post(empty_trash))
        .route("/api/profiles/{id}", get(get_profile).put(update_profile).delete(delete_profile))
        .route("/api/profiles/{id}/regenerate-machine", post(regenerate_machine))
        .route("/api/profiles/{id}/start", post(start))
        .route("/api/profiles/{id}/stop", post(stop))
        .route("/api/profiles/{id}/restore", post(restore_profile))
        .route("/api/profiles/{id}/purge", delete(purge_profile))
        .route("/api/profiles/{id}/reveal", post(reveal))
        .route("/api/bookmarks/import", post(import_bookmarks).layer(DefaultBodyLimit::max(16 << 20)))
}
