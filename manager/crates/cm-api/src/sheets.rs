//! Excel / CSV bulk import/export routes (phase 11).
//!
//! - `GET  /api/sheets/export?format=xlsx|csv&ids=a,b`  download (all profiles, or only `ids`)
//! - `POST /api/sheets/import/preview?format=&createMissing=`  raw file body -> dry-run diff + `token`
//! - `POST /api/sheets/import/apply`  `{ token, createMissing? }` -> applies the file kept under the token

use super::{ApiError, ApiResult, Shared, ok};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Query, State},
    http::{HeaderValue, header},
    response::Response,
    routing::{get, post},
};
use cm_core::sheets::SheetFormat;
use serde::Deserialize;

/// Largest sheet accepted (20k rows of text fit well inside).
const MAX_UPLOAD: usize = 32 * 1024 * 1024;

#[derive(Deserialize)]
struct ExportQuery {
    #[serde(default = "xlsx")]
    format: SheetFormat,
    /// Comma separated profile ids; absent = all.
    ids: Option<String>,
}

fn xlsx() -> SheetFormat {
    SheetFormat::Xlsx
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewQuery {
    format: Option<SheetFormat>,
    #[serde(default)]
    create_missing: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Apply {
    token: String,
    #[serde(default)]
    create_missing: bool,
}

async fn export(State(m): State<Shared>, Query(q): Query<ExportQuery>) -> Result<Response, ApiError> {
    let ids: Option<Vec<String>> = q.ids.map(|s| s.split(',').map(|i| i.trim().to_string()).filter(|i| !i.is_empty()).collect());
    let f = m.export_sheet(q.format, ids.as_deref()).await?;
    let mut r = Response::new(f.bytes.into());
    let h = r.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(f.content_type));
    // The name is generated from a date: safe to quote.
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{}\"", f.file_name)) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(r)
}

async fn preview(State(m): State<Shared>, Query(q): Query<PreviewQuery>, body: Bytes) -> ApiResult {
    ok(m.preview_sheet(body.to_vec(), q.format, q.create_missing).await?)
}

async fn apply(State(m): State<Shared>, Json(b): Json<Apply>) -> ApiResult {
    ok(m.apply_sheet(&b.token, b.create_missing).await?)
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/sheets/export", get(export))
        .route("/api/sheets/import/preview", post(preview).layer(DefaultBodyLimit::max(MAX_UPLOAD)))
        .route("/api/sheets/import/apply", post(apply))
}
