//! Profile transfer: zip export/import, clone, bulk create (phase 6) — stub: every route answers 501 until the phase replaces this file's contents.

use super::{ApiError, Shared, not_implemented};
use axum::{Router, routing::any};

async fn stub() -> ApiError {
    not_implemented("transfer")
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/transfer", any(stub)).route("/api/transfer/{*rest}", any(stub))
}
