//! Templates + Firefox extensions (phase 9) — stub: every route answers 501 until the phase replaces this file's contents.

use super::{ApiError, Shared, not_implemented};
use axum::{Router, routing::any};

async fn stub() -> ApiError {
    not_implemented("templates")
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/templates", any(stub))
        .route("/api/templates/{*rest}", any(stub))
        .route("/api/extensions", any(stub))
        .route("/api/extensions/{*rest}", any(stub))
}
