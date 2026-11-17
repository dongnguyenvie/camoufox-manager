//! Profile transfer routes (phase 6).
//!
//! - `POST /api/transfer/export/{id}`            closed profile -> zip in `<data dir>/exports`
//! - `GET  /api/transfer/exports/{file}`         download an export (streamed)
//! - `POST /api/transfer/import?mode=&name=`     raw zip body (streamed to a temp file) -> new profile
//! - `POST /api/transfer/import-path`            `{ path, mode?, name? }` zip already on this machine
//! - `POST /api/transfer/clone/{id}`             `{ name, newMachine? }`
//! - `POST /api/transfer/bulk`                   `{ pattern, count, os, start? }`

use super::{ApiError, ApiResult, Shared, ok};
use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{HeaderValue, header},
    response::Response,
    routing::{get, post},
};
use cm_core::{
    CmError,
    snapshot::MAX_TOTAL_BYTES,
    transfer::{BulkCreateRequest, CloneRequest, ImportIdMode},
};
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;

#[derive(Deserialize, Default)]
struct ImportQuery {
    #[serde(default)]
    mode: ImportIdMode,
    name: Option<String>,
}

#[derive(Deserialize)]
struct ImportPath {
    path: String,
    #[serde(default)]
    mode: ImportIdMode,
    name: Option<String>,
}

async fn export(State(m): State<Shared>, Path(id): Path<String>) -> ApiResult {
    ok(m.export_profile(&id).await?)
}

async fn download(State(m): State<Shared>, Path(file): Path<String>) -> Result<Response, ApiError> {
    let path = m.export_file(&file)?;
    let f = tokio::fs::File::open(&path).await.map_err(anyhow::Error::from)?;
    let len = f.metadata().await.map_err(anyhow::Error::from)?.len();
    let mut r = Response::new(Body::from_stream(ReaderStream::with_capacity(f, 256 * 1024)));
    let h = r.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/zip"));
    h.insert(header::CONTENT_LENGTH, HeaderValue::from(len));
    // `export_file` only accepts [A-Za-z0-9-_.] names, so the quoted value is safe.
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{file}\"")) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    Ok(r)
}

/// Stream the request body to a scratch file (never into memory), then import it.
async fn import(State(m): State<Shared>, Query(q): Query<ImportQuery>, req: Request) -> ApiResult {
    let tmp = m.upload_path()?;
    let res = async {
        let mut file = tokio::fs::File::create(&tmp).await?;
        let mut stream = req.into_body().into_data_stream();
        let mut total = 0u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| CmError::Invalid(format!("upload interrupted: {e}")))?;
            total += chunk.len() as u64;
            // Compressed size is bounded here; the uncompressed total is bounded by the manifest.
            if total > MAX_TOTAL_BYTES {
                return Err(CmError::Invalid("archive is larger than 2 GiB".into()).into());
            }
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        drop(file);
        m.import_snapshot(&tmp, q.mode, q.name).await
    }
    .await;
    let _ = tokio::fs::remove_file(&tmp).await;
    ok(res?)
}

async fn import_path(State(m): State<Shared>, Json(b): Json<ImportPath>) -> ApiResult {
    let p = std::path::PathBuf::from(b.path.trim());
    if !p.is_absolute() || !p.is_file() {
        return Err(CmError::Invalid("path must be an existing absolute file path".into()).into());
    }
    ok(m.import_snapshot(&p, b.mode, b.name).await?)
}

async fn clone_profile(State(m): State<Shared>, Path(id): Path<String>, Json(b): Json<CloneRequest>) -> ApiResult {
    ok(m.clone_profile(&id, b).await?)
}

async fn bulk(State(m): State<Shared>, Json(b): Json<BulkCreateRequest>) -> ApiResult {
    ok(m.bulk_create(b).await?)
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/transfer/export/{id}", post(export))
        .route("/api/transfer/exports/{file}", get(download))
        .route("/api/transfer/import", post(import).layer(DefaultBodyLimit::disable()))
        .route("/api/transfer/import-path", post(import_path))
        .route("/api/transfer/clone/{id}", post(clone_profile))
        .route("/api/transfer/bulk", post(bulk))
}
