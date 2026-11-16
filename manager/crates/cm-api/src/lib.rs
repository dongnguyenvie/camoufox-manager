//! Thin HTTP layer: parse the request, call [`Manager`], return JSON.
//!
//! One module per area, each exposing `routes()`; [`router`] merges them. Errors carrying a
//! [`CmError`] map to 404/400/409/501, anything else to 400 `{ "error": "…" }`.
//!
//! | module | routes | phase |
//! |---|---|---|
//! | `ui` | `/`, static assets | 1 |
//! | `profiles` | `/api/overview`, `/api/profiles…` | 1, 4 |
//! | `proxies` | `/api/proxies…`, `/api/profiles/{id}/assign` | 1, 2 |
//! | `cookies` | `/api/cookies…` | 5 |
//! | `transfer` | `/api/transfer…` | 6 |
//! | `org` | `/api/tags…`, `/api/groups…`, `/api/views…` | 7 |
//! | `templates` | `/api/templates…`, `/api/extensions…` | 9 |
//! | `schedules` | `/api/schedules…` | 10 |
//! | `sheets` | `/api/sheets…` | 11 |
//! | `settings` | `/api/settings` | 1, 12 |
//! | `auth` | API-key middleware (pass-through until phase 12) | 12 |

mod auth;
mod cookies;
mod org;
mod profiles;
mod proxies;
mod schedules;
mod settings;
mod sheets;
mod templates;
mod transfer;
mod ui;

use axum::{
    Json, Router,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use cm_core::CmError;
use cm_service::Manager;
use serde_json::{Value, json};
use std::sync::Arc;

type Shared = Arc<Manager>;

pub(crate) struct ApiError(anyhow::Error);

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(e: E) -> Self {
        Self(e.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let typed = self.0.chain().find_map(|c| c.downcast_ref::<CmError>());
        let status = match typed {
            Some(CmError::NotFound(_)) => StatusCode::NOT_FOUND,
            Some(CmError::Invalid(_)) => StatusCode::BAD_REQUEST,
            Some(CmError::Conflict(_)) => StatusCode::CONFLICT,
            Some(CmError::NotImplemented(_)) => StatusCode::NOT_IMPLEMENTED,
            None => StatusCode::BAD_REQUEST,
        };
        (status, Json(json!({ "error": format!("{:#}", self.0) }))).into_response()
    }
}

pub(crate) type ApiResult = Result<Json<Value>, ApiError>;

pub(crate) fn ok<T: serde::Serialize>(v: T) -> ApiResult {
    Ok(Json(serde_json::to_value(v)?))
}

/// Handler for areas whose phase has not landed: 501 `{ "error": "<area>: not implemented yet" }`.
pub(crate) fn not_implemented(area: &'static str) -> ApiError {
    ApiError(cm_core::not_implemented(area))
}

pub fn router(m: Shared) -> Router {
    let api = Router::new()
        .merge(profiles::routes())
        .merge(proxies::routes())
        .merge(cookies::routes())
        .merge(transfer::routes())
        .merge(org::routes())
        .merge(templates::routes())
        .merge(schedules::routes())
        .merge(sheets::routes())
        .merge(settings::routes());
    auth::apply(api, m.clone()).merge(ui::routes()).with_state(m)
}
