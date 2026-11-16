//! Scheduling (phase 10) — stub: every route answers 501 until the phase replaces this file's contents.

use super::{ApiError, Shared, not_implemented};
use axum::{Router, routing::any};

async fn stub() -> ApiError {
    not_implemented("schedules")
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/schedules", any(stub)).route("/api/schedules/{*rest}", any(stub))
}
