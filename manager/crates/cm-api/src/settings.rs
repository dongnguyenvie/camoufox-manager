//! Settings routes: `GET /api/settings`, `PUT /api/settings` (partial). Phase 12 extends.

use super::{ApiResult, Shared, ok};
use axum::{Json, Router, extract::State, routing::get};
use cm_service::settings::SettingsPatch;

async fn get_settings(State(m): State<Shared>) -> ApiResult {
    ok(m.settings().await)
}

async fn put_settings(State(m): State<Shared>, Json(b): Json<SettingsPatch>) -> ApiResult {
    ok(m.update_settings(b).await?)
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/settings", get(get_settings).put(put_settings))
}
