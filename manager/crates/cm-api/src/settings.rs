//! Settings routes (phases 1, 12).
//!
//! | route | |
//! |---|---|
//! | `GET /api/settings` | language, headless default, engine status + versions, data dir, ports |
//! | `PUT /api/settings` | partial update `{language?, headless?, requireApiKey?}` |
//! | `POST /api/settings/reveal-data` | open the data folder in Finder |
//! | `POST /api/engine/install` | start Camoufox install / update (409 while one runs) |
//! | `GET /api/engine/install?from=N` | log lines from position N + running/ok/error (poll) |
//! | `GET/POST /api/api-keys`, `DELETE /api/api-keys/{id}` | named keys; the secret is only in the POST answer (UI only, see `auth`) |

use super::{ApiResult, Shared, ok};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
    routing::{delete, get, post},
};
use cm_service::settings::SettingsPatch;
use serde::Deserialize;
use serde_json::json;

fn port_of(h: &HeaderMap) -> Option<u16> {
    h.get(header::HOST)?.to_str().ok()?.rsplit_once(':')?.1.parse().ok()
}

async fn get_settings(State(m): State<Shared>, headers: HeaderMap) -> ApiResult {
    let mut v = m.settings().await;
    v.ui_port = port_of(&headers);
    ok(v)
}

async fn put_settings(State(m): State<Shared>, headers: HeaderMap, Json(b): Json<SettingsPatch>) -> ApiResult {
    let mut v = m.update_settings(b).await?;
    v.ui_port = port_of(&headers);
    ok(v)
}

async fn reveal_data(State(m): State<Shared>) -> ApiResult {
    std::process::Command::new("open").arg(&m.config().data_dir).spawn()?;
    ok(json!({ "ok": true }))
}

async fn install(State(m): State<Shared>) -> ApiResult {
    m.start_engine_install()?;
    ok(json!({ "ok": true }))
}

#[derive(Deserialize)]
struct LogQuery {
    #[serde(default)]
    from: usize,
}

async fn install_log(State(m): State<Shared>, Query(q): Query<LogQuery>) -> ApiResult {
    ok(m.engine_install_log(q.from))
}

async fn list_keys(State(m): State<Shared>) -> ApiResult {
    ok(json!({ "keys": m.list_api_keys().await? }))
}

#[derive(Deserialize)]
struct NewKey {
    name: String,
}

async fn create_key(State(m): State<Shared>, Json(b): Json<NewKey>) -> ApiResult {
    ok(m.create_api_key(&b.name).await?)
}

async fn revoke_key(State(m): State<Shared>, Path(id): Path<String>) -> ApiResult {
    m.revoke_api_key(&id).await?;
    ok(json!({ "ok": true }))
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/settings", get(get_settings).put(put_settings))
        .route("/api/settings/reveal-data", post(reveal_data))
        .route("/api/engine/install", get(install_log).post(install))
        .route("/api/api-keys", get(list_keys).post(create_key))
        .route("/api/api-keys/{id}", delete(revoke_key))
}
