//! Templates + Firefox extension library routes (phase 9).
//!
//! - `GET/POST /api/templates`, `GET/PUT/DELETE /api/templates/{id}`
//! - `POST /api/templates/{id}/create`           `{ count?, pattern?, name?, start? }` -> profiles made from it
//! - `POST /api/templates/from-profile/{id}`     `{ name }` save a profile's settings as a template
//! - `GET /api/extensions`, `POST /api/extensions` (raw `.xpi` body, ≤ 64 MiB), `DELETE /api/extensions/{id}`
//! - `POST /api/extensions/assign`               `{ profiles, extensions, mode: add|remove|set }`
//! - `GET/PUT /api/profiles/{id}/extensions`     `{ extensions: [ids] }` (applied at the next start)

use super::{ApiResult, Shared, ok};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    routing::{get, post},
};
use cm_core::Template;
use cm_service::templates::{AssignRequest, FromTemplate, MAX_XPI_BYTES};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct NameBody {
    name: String,
}

#[derive(Deserialize)]
struct IdsBody {
    extensions: Vec<i64>,
}

async fn list(State(m): State<Shared>) -> ApiResult {
    ok(m.list_templates().await?)
}

async fn get_one(State(m): State<Shared>, Path(id): Path<i64>) -> ApiResult {
    ok(m.template(id).await?)
}

async fn create(State(m): State<Shared>, Json(t): Json<Template>) -> ApiResult {
    ok(m.create_template(t).await?)
}

async fn update(State(m): State<Shared>, Path(id): Path<i64>, Json(t): Json<Template>) -> ApiResult {
    ok(m.update_template(id, t).await?)
}

async fn delete(State(m): State<Shared>, Path(id): Path<i64>) -> ApiResult {
    m.delete_template(id).await?;
    ok(json!({ "ok": true }))
}

async fn make_profiles(State(m): State<Shared>, Path(id): Path<i64>, Json(b): Json<FromTemplate>) -> ApiResult {
    ok(m.create_from_template(id, b).await?)
}

async fn from_profile(State(m): State<Shared>, Path(id): Path<String>, Json(b): Json<NameBody>) -> ApiResult {
    ok(m.template_from_profile(&id, &b.name).await?)
}

async fn list_extensions(State(m): State<Shared>) -> ApiResult {
    ok(m.list_extensions().await?)
}

async fn upload_extension(State(m): State<Shared>, body: Bytes) -> ApiResult {
    ok(m.add_extension(&body).await?)
}

async fn delete_extension(State(m): State<Shared>, Path(id): Path<i64>) -> ApiResult {
    m.delete_extension(id).await?;
    ok(json!({ "ok": true }))
}

async fn assign(State(m): State<Shared>, Json(b): Json<AssignRequest>) -> ApiResult {
    ok(json!({ "profiles": m.assign_extensions(b).await? }))
}

async fn profile_extensions(State(m): State<Shared>, Path(id): Path<String>) -> ApiResult {
    ok(json!({ "extensions": m.profile_extensions(&id).await? }))
}

async fn set_profile_extensions(State(m): State<Shared>, Path(id): Path<String>, Json(b): Json<IdsBody>) -> ApiResult {
    ok(json!({ "extensions": m.set_profile_extensions(&id, &b.extensions).await? }))
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/templates", get(list).post(create))
        .route("/api/templates/{id}", get(get_one).put(update).delete(delete))
        .route("/api/templates/{id}/create", post(make_profiles))
        .route("/api/templates/from-profile/{id}", post(from_profile))
        .route("/api/extensions", get(list_extensions).post(upload_extension).layer(DefaultBodyLimit::max(MAX_XPI_BYTES + 1024)))
        .route("/api/extensions/{id}", axum::routing::delete(delete_extension))
        .route("/api/extensions/assign", post(assign))
        .route("/api/profiles/{id}/extensions", get(profile_extensions).put(set_profile_extensions))
}
