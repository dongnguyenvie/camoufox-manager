//! Organization routes (phase 7).
//!
//! - `GET /api/org` — tags, groups, saved views and the profile assignments in one call
//! - `GET|POST /api/tags`, `DELETE /api/tags/{id}`
//! - `GET|POST /api/groups`, `PUT|DELETE /api/groups/{id}`
//! - `GET|POST /api/views` (POST with an existing name replaces it), `DELETE /api/views/{id}`
//! - `POST /api/bulk` — `{ action, profileIds, proxyId?, tagId?, groupId? }`, per-profile results

use super::{ApiResult, Shared, ok};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use cm_service::org::BulkRequest;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
struct TagBody {
    name: String,
    #[serde(default)]
    color: Option<String>,
}

#[derive(Deserialize)]
struct GroupBody {
    name: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize)]
struct ViewBody {
    name: String,
    #[serde(default)]
    query: Value,
}

async fn org(State(m): State<Shared>) -> ApiResult {
    ok(m.org_overview().await?)
}

async fn list_tags(State(m): State<Shared>) -> ApiResult {
    ok(m.deps().tags.list().await?)
}
async fn create_tag(State(m): State<Shared>, Json(b): Json<TagBody>) -> ApiResult {
    ok(m.create_tag(&b.name, b.color.as_deref()).await?)
}
async fn delete_tag(State(m): State<Shared>, Path(id): Path<i64>) -> ApiResult {
    m.delete_tag(id).await?;
    ok(json!({ "ok": true }))
}

async fn list_groups(State(m): State<Shared>) -> ApiResult {
    ok(m.deps().groups.list().await?)
}
async fn create_group(State(m): State<Shared>, Json(b): Json<GroupBody>) -> ApiResult {
    ok(m.create_group(&b.name, b.description.as_deref()).await?)
}
async fn rename_group(State(m): State<Shared>, Path(id): Path<i64>, Json(b): Json<GroupBody>) -> ApiResult {
    m.rename_group(id, &b.name).await?;
    ok(json!({ "ok": true }))
}
async fn delete_group(State(m): State<Shared>, Path(id): Path<i64>) -> ApiResult {
    m.delete_group(id).await?;
    ok(json!({ "ok": true }))
}

async fn list_views(State(m): State<Shared>) -> ApiResult {
    ok(m.deps().views.list().await?)
}
async fn save_view(State(m): State<Shared>, Json(b): Json<ViewBody>) -> ApiResult {
    ok(m.save_view(&b.name, &b.query).await?)
}
async fn delete_view(State(m): State<Shared>, Path(id): Path<i64>) -> ApiResult {
    m.delete_view(id).await?;
    ok(json!({ "ok": true }))
}

async fn bulk(State(m): State<Shared>, Json(b): Json<BulkRequest>) -> ApiResult {
    ok(m.bulk(b).await?)
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/org", get(org))
        .route("/api/tags", get(list_tags).post(create_tag))
        .route("/api/tags/{id}", axum::routing::delete(delete_tag))
        .route("/api/groups", get(list_groups).post(create_group))
        .route("/api/groups/{id}", axum::routing::put(rename_group).delete(delete_group))
        .route("/api/views", get(list_views).post(save_view))
        .route("/api/views/{id}", axum::routing::delete(delete_view))
        .route("/api/bulk", post(bulk))
}
