//! API-key middleware — phase 12 implements it: when a key is set (`api.key` setting),
//! non-UI clients must send `X-API-Key`. Until then every request passes.

use super::Shared;
use axum::{Router, extract::Request, middleware::Next, response::Response};

async fn check(req: Request, next: Next) -> Response {
    next.run(req).await
}

/// Wrap the `/api` routes with the API-key check.
pub fn apply(api: Router<Shared>, _m: Shared) -> Router<Shared> {
    api.layer(axum::middleware::from_fn(check))
}
