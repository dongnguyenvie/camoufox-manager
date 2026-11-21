//! Signed peer routes of the sync engine (phase 17), merged into `sync::signed_routes()`.
//! They run on the sync listener only: source policy + device signature already passed, the caller
//! is `Extension<AuthedDevice>`. Protocol and wire types: `cm_sync::apply`.
//!
//! Chunk bodies are raw bytes (≤ 8 MiB, the listener buffers up to 16 MiB). A downloaded chunk
//! carries its SHA-256 in `x-chunk-sha256`; an uploaded one in the `?sha=` query (which is part
//! of the signed path).

// Handlers return an early `Response` as the error: it is built once per request, size is irrelevant.
#![allow(clippy::result_large_err)]

use super::{
    Shared,
    sync::AuthedDevice,
    sync_ui::{fail, refuse},
};
use axum::{
    Extension, Json, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use cm_sync::{BeginRequest, DoneRequest, PrepareRequest, transfer::CHUNK_SHA_HEADER};
use serde::Deserialize;
use serde_json::json;

type Res = Result<Response, Response>;

fn json_ok<T: serde::Serialize>(v: T) -> Res {
    Ok(Json(v).into_response())
}

fn bad_json<T>(b: &[u8]) -> Result<T, Response>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_slice(b).map_err(|e| refuse(StatusCode::BAD_REQUEST, "invalid", &format!("invalid request body: {e}")))
}

async fn profiles(State(m): State<Shared>, Extension(AuthedDevice(d)): Extension<AuthedDevice>) -> Res {
    json_ok(m.sync_manifest(&d.id).await.map_err(fail)?)
}

async fn prepare(State(m): State<Shared>, Extension(AuthedDevice(d)): Extension<AuthedDevice>, body: Bytes) -> Res {
    let req: PrepareRequest = bad_json(&body)?;
    json_ok(m.sync_prepare(&d.id, req).await.map_err(fail)?)
}

async fn out_chunk(
    State(m): State<Shared>,
    Extension(AuthedDevice(d)): Extension<AuthedDevice>,
    Path((tid, n)): Path<(String, u64)>,
) -> Res {
    let (bytes, sha) = m.sync_chunk(&d.id, &tid, n).await.map_err(fail)?;
    Ok((
        [(header::CONTENT_TYPE, "application/octet-stream".to_owned()), (axum::http::HeaderName::from_static(CHUNK_SHA_HEADER), sha)],
        bytes,
    )
        .into_response())
}

async fn out_done(
    State(m): State<Shared>,
    Extension(AuthedDevice(d)): Extension<AuthedDevice>,
    Path(tid): Path<String>,
    body: Bytes,
) -> Res {
    let req: DoneRequest = bad_json(&body)?;
    m.sync_done(&d.id, &tid, req).await.map_err(fail)?;
    json_ok(json!({ "ok": true }))
}

async fn out_cancel(State(m): State<Shared>, Extension(AuthedDevice(d)): Extension<AuthedDevice>, Path(tid): Path<String>) -> Res {
    m.sync_cancel_out(&d.id, &tid).map_err(fail)?;
    json_ok(json!({ "ok": true }))
}

async fn begin(State(m): State<Shared>, Extension(AuthedDevice(d)): Extension<AuthedDevice>, body: Bytes) -> Res {
    let req: BeginRequest = bad_json(&body)?;
    json_ok(m.sync_begin(&d.id, req).await.map_err(fail)?)
}

async fn in_status(State(m): State<Shared>, Extension(AuthedDevice(d)): Extension<AuthedDevice>, Path(tid): Path<String>) -> Res {
    json_ok(m.sync_inbox_status(&d.id, &tid).await.map_err(fail)?)
}

#[derive(Deserialize)]
struct ShaQuery {
    sha: String,
}

async fn in_chunk(
    State(m): State<Shared>,
    Extension(AuthedDevice(d)): Extension<AuthedDevice>,
    Path((tid, n)): Path<(String, u64)>,
    Query(q): Query<ShaQuery>,
    body: Bytes,
) -> Res {
    json_ok(m.sync_put_chunk(&d.id, &tid, n, body.to_vec(), q.sha).await.map_err(fail)?)
}

async fn in_commit(State(m): State<Shared>, Extension(AuthedDevice(d)): Extension<AuthedDevice>, Path(tid): Path<String>) -> Res {
    json_ok(m.sync_commit(&d.id, &d.name, &tid).await.map_err(fail)?)
}

async fn in_cancel(State(m): State<Shared>, Extension(AuthedDevice(d)): Extension<AuthedDevice>, Path(tid): Path<String>) -> Res {
    m.sync_cancel_in(&d.id, &tid).map_err(fail)?;
    json_ok(json!({ "ok": true }))
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/sync/v1/profiles", get(profiles))
        .route("/sync/v1/transfer/prepare", post(prepare))
        .route("/sync/v1/transfer/{id}/chunk/{n}", get(out_chunk))
        .route("/sync/v1/transfer/{id}/done", post(out_done))
        .route("/sync/v1/transfer/{id}/cancel", post(out_cancel))
        .route("/sync/v1/inbox/begin", post(begin))
        .route("/sync/v1/inbox/{id}", get(in_status))
        .route("/sync/v1/inbox/{id}/chunk/{n}", axum::routing::put(in_chunk))
        .route("/sync/v1/inbox/{id}/commit", post(in_commit))
        .route("/sync/v1/inbox/{id}/cancel", post(in_cancel))
}
