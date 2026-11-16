//! Excel/CSV bulk import/export (phase 11) — stub: every route answers 501 until the phase replaces this file's contents.

use super::{ApiError, Shared, not_implemented};
use axum::{Router, routing::any};

async fn stub() -> ApiError {
    not_implemented("sheets")
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/sheets", any(stub)).route("/api/sheets/{*rest}", any(stub))
}
