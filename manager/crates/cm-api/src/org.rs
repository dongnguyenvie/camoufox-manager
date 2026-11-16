//! Organization: tags, groups, saved views, bulk actions (phase 7) — stub: every route answers 501 until the phase replaces this file's contents.

use super::{ApiError, Shared, not_implemented};
use axum::{Router, routing::any};

async fn stub() -> ApiError {
    not_implemented("org")
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/tags", any(stub))
        .route("/api/tags/{*rest}", any(stub))
        .route("/api/groups", any(stub))
        .route("/api/groups/{*rest}", any(stub))
        .route("/api/views", any(stub))
        .route("/api/views/{*rest}", any(stub))
        .route("/api/bulk", any(stub))
        .route("/api/bulk/{*rest}", any(stub))
}
