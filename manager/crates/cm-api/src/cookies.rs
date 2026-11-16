//! Cookies import/export (phase 5) — stub: every route answers 501 until the phase replaces this file's contents.

use super::{ApiError, Shared, not_implemented};
use axum::{Router, routing::any};

async fn stub() -> ApiError {
    not_implemented("cookies")
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/cookies", any(stub)).route("/api/cookies/{*rest}", any(stub))
}
