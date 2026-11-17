//! Cookies of a closed profile (phase 5).
//!
//! - `GET  /api/cookies/{id}/export?format=json|netscape` → the file (download); 409 while running
//! - `POST /api/cookies/{id}/import` `{ content, format?, mode? }` → import report; 409 while running

use super::{ApiError, ApiResult, Shared, ok};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use cm_core::cookies::CookieFormat;
use cm_service::cookies::CookieImport;
use serde::Deserialize;

#[derive(Deserialize)]
struct ExportQuery {
    #[serde(default = "json")]
    format: CookieFormat,
}

fn json() -> CookieFormat {
    CookieFormat::Json
}

async fn export(State(m): State<Shared>, Path(id): Path<String>, Query(q): Query<ExportQuery>) -> Result<Response, ApiError> {
    let e = m.export_cookies(&id, q.format).await?;
    let (ext, ct) = match e.format {
        CookieFormat::Json => ("json", "application/json; charset=utf-8"),
        CookieFormat::Netscape => ("txt", "text/plain; charset=utf-8"),
    };
    let mut r = e.body.into_response();
    let h = r.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(ct));
    h.insert(header::CONTENT_DISPOSITION, HeaderValue::from_str(&format!("attachment; filename=\"cookies-{id}.{ext}\""))?);
    h.insert("x-cookie-count", HeaderValue::from(e.count));
    Ok(r)
}

async fn import(State(m): State<Shared>, Path(id): Path<String>, Json(b): Json<CookieImport>) -> ApiResult {
    ok(m.import_cookies(&id, b).await?)
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/cookies/{id}/export", get(export)).route("/api/cookies/{id}/import", post(import))
}
