//! Notification routes (phase 19) and the "finished" hook for imports / exports.
//!
//! | route | |
//! |---|---|
//! | `GET /api/notifications?limit=N` | `{ items[], unread, systemNotify }`, newest first |
//! | `POST /api/notifications/read` | `{ ids? }`: mark those (or all when omitted) as read |
//! | `POST /api/notifications/clear` | hide all |
//! | `PUT /api/notifications/settings` | `{ systemNotify }`: also show a macOS banner (default off) |
//!
//! [`layer`] watches the import / export routes of other areas and records a "finished" notification
//! when the call succeeded, so those modules need no changes.

use super::{ApiResult, Shared, ok};
use axum::{
    Json, Router,
    body::Body,
    extract::{Query, Request, State},
    http::{Method, StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::{get, post, put},
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
struct ListQuery {
    limit: Option<usize>,
}

async fn list(State(m): State<Shared>, Query(q): Query<ListQuery>) -> ApiResult {
    ok(m.notifications(q.limit.unwrap_or(50)).await?)
}

#[derive(Deserialize)]
struct ReadBody {
    ids: Option<Vec<i64>>,
}

async fn read(State(m): State<Shared>, Json(b): Json<ReadBody>) -> ApiResult {
    m.mark_notifications_read(b.ids.as_deref()).await?;
    ok(m.notifications(50).await?)
}

async fn clear(State(m): State<Shared>) -> ApiResult {
    m.clear_notifications().await?;
    ok(m.notifications(50).await?)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsBody {
    system_notify: bool,
}

async fn settings(State(m): State<Shared>, Json(b): Json<SettingsBody>) -> ApiResult {
    m.set_system_notify(b.system_notify).await?;
    ok(m.notifications(50).await?)
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/notifications", get(list))
        .route("/api/notifications/read", post(read))
        .route("/api/notifications/clear", post(clear))
        .route("/api/notifications/settings", put(settings))
}

/// What a finished call is called, and which JSON field names it (`None` = take the file name
/// from `Content-Disposition`, for downloads that must not be buffered).
fn classify(method: &Method, path: &str) -> Option<(&'static str, Option<&'static str>)> {
    match (method.as_str(), path) {
        ("POST", p) if p.starts_with("/api/transfer/export/") => Some(("transfer.export_finished", Some("fileName"))),
        ("POST", "/api/transfer/import" | "/api/transfer/import-path") => Some(("transfer.import_finished", Some("profile.name"))),
        ("GET", "/api/sheets/export") => Some(("sheets.export_finished", None)),
        ("POST", "/api/sheets/import/apply") => Some(("sheets.import_finished", Some("summary"))),
        _ => None,
    }
}

fn pick(v: &Value, field: &str) -> String {
    let mut cur = v;
    for part in field.split('.') {
        cur = &cur[part];
    }
    cur.as_str().unwrap_or_default().to_string()
}

fn disposition_name(res: &Response) -> String {
    res.headers()
        .get(header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split("filename=").nth(1))
        .map(|n| n.trim().trim_matches('"').to_string())
        .unwrap_or_default()
}

async fn hook(State(m): State<Shared>, req: Request, next: Next) -> Response {
    let Some((kind, field)) = classify(req.method(), req.uri().path()) else { return next.run(req).await };
    let res = next.run(req).await;
    if !res.status().is_success() {
        return res;
    }
    match field {
        None => {
            let name = disposition_name(&res);
            m.notify(kind, json!({ "name": name }), None).await;
            res
        }
        Some(field) => {
            let (parts, body) = res.into_parts();
            // Small JSON answers only; anything bigger is passed on without a notification.
            let Ok(bytes) = axum::body::to_bytes(body, 1 << 20).await else {
                return Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::empty()).unwrap_or_default();
            };
            if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
                let name = if field == "summary" { format!("{} created, {} updated", v["created"], v["updated"]) } else { pick(&v, field) };
                m.notify(kind, json!({ "name": name }), None).await;
            }
            Response::from_parts(parts, Body::from(bytes))
        }
    }
}

/// Wrap the `/api` router so successful imports and exports leave a notification.
pub fn layer(api: Router<Shared>, m: Shared) -> Router<Shared> {
    api.layer(middleware::from_fn_with_state(m, hook))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_import_and_export_calls_are_watched() {
        let post = Method::POST;
        assert!(classify(&post, "/api/transfer/export/abc").is_some());
        assert!(classify(&post, "/api/transfer/import").is_some());
        assert!(classify(&Method::GET, "/api/sheets/export").is_some());
        assert!(classify(&post, "/api/sheets/import/apply").is_some());
        assert!(classify(&Method::GET, "/api/transfer/exports/file.zip").is_none(), "downloading is not exporting");
        assert!(classify(&post, "/api/sheets/import/preview").is_none(), "a dry run is not an import");
        assert!(classify(&post, "/api/transfer/clone/x").is_none());
    }

    #[test]
    fn picks_nested_fields() {
        assert_eq!(pick(&json!({ "profile": { "name": "Shop" } }), "profile.name"), "Shop");
        assert_eq!(pick(&json!({}), "profile.name"), "");
    }
}
