//! Doctor route (phase 19).
//!
//! | route | |
//! |---|---|
//! | `GET /api/doctor` | run every check: `{ ok, summary, checks[], markdown, ... }` (read-only, nothing is sent anywhere) |

use super::{ApiResult, Shared, ok};
use axum::{
    Router,
    extract::State,
    http::{HeaderMap, header},
    routing::get,
};
use cm_service::doctor::DoctorOptions;

/// `Host: 127.0.0.1:7878` -> 7878.
fn port_of(h: &HeaderMap) -> Option<u16> {
    h.get(header::HOST)?.to_str().ok()?.rsplit_once(':')?.1.parse().ok()
}

async fn run(State(m): State<Shared>, headers: HeaderMap) -> ApiResult {
    ok(m.doctor(DoctorOptions { ui_port: port_of(&headers) }).await?)
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/doctor", get(run))
}
