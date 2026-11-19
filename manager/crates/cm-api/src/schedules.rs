//! Scheduling routes (phase 10):
//! `GET /api/schedules` · `POST /api/schedules` · `PUT|DELETE /api/schedules/{id}` ·
//! `POST /api/schedules/{id}/enabled {enabled}` · `POST /api/schedules/{id}/run` (run now, returns at once) ·
//! `GET /api/schedules/{id}/runs?limit=` · `GET /api/schedules/runs?limit=` (every schedule).

use super::{ApiResult, Shared, ok};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use cm_service::scheduler::ScheduleInput;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct Limit {
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct Enabled {
    enabled: bool,
}

async fn list(State(m): State<Shared>) -> ApiResult {
    ok(m.schedules().await?)
}

async fn create(State(m): State<Shared>, Json(i): Json<ScheduleInput>) -> ApiResult {
    ok(m.create_schedule(i).await?)
}

async fn update(State(m): State<Shared>, Path(id): Path<i64>, Json(i): Json<ScheduleInput>) -> ApiResult {
    ok(m.update_schedule(id, i).await?)
}

async fn delete(State(m): State<Shared>, Path(id): Path<i64>) -> ApiResult {
    m.delete_schedule(id).await?;
    ok(json!({ "ok": true }))
}

async fn enabled(State(m): State<Shared>, Path(id): Path<i64>, Json(b): Json<Enabled>) -> ApiResult {
    ok(m.set_schedule_enabled(id, b.enabled).await?)
}

async fn run_now(State(m): State<Shared>, Path(id): Path<i64>) -> ApiResult {
    m.run_schedule_now(id).await?;
    ok(json!({ "started": true }))
}

async fn runs(State(m): State<Shared>, Path(id): Path<i64>, Query(q): Query<Limit>) -> ApiResult {
    ok(m.schedule_runs(Some(id), q.limit.unwrap_or(50)).await?)
}

async fn all_runs(State(m): State<Shared>, Query(q): Query<Limit>) -> ApiResult {
    ok(m.schedule_runs(None, q.limit.unwrap_or(100)).await?)
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/schedules", get(list).post(create))
        .route("/api/schedules/runs", get(all_runs))
        .route("/api/schedules/{id}", axum::routing::put(update).delete(delete))
        .route("/api/schedules/{id}/enabled", post(enabled))
        .route("/api/schedules/{id}/run", post(run_now))
        .route("/api/schedules/{id}/runs", get(runs))
}
