//! Serves the embedded UI: `/` and every file in `cm_ui::ASSETS`.

use super::Shared;
use axum::{
    Router,
    extract::Path,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};

async fn index() -> Html<&'static str> {
    Html(cm_ui::INDEX_HTML)
}

async fn asset(Path(path): Path<String>) -> Response {
    match cm_ui::get(&path) {
        Some(a) => ([(header::CONTENT_TYPE, a.content_type), (header::CACHE_CONTROL, "no-cache")], a.body).into_response(),
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/", get(index)).route("/{*path}", get(asset))
}
