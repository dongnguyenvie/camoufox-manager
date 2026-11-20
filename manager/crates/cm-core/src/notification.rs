//! In-app notifications (bell in the header, phase 19). Local only, persisted by [`NotificationRepo`].
//!
//! Each row carries a machine `kind` (`profile.crashed`, `schedule.failed`, ...) plus `params`, so the
//! UI can translate it, and an English `title`/`body` as fallback and for the macOS banner.

use crate::Result;
use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;

/// `true`: also show a macOS banner (`osascript display notification`). Default off.
pub const SYSTEM_NOTIFY: &str = "notify.system";

/// Rows kept; older ones are dropped on insert.
pub const KEEP: usize = 200;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: i64,
    pub kind: String,
    /// `info` | `warn` | `error`
    pub level: String,
    pub title: String,
    pub body: String,
    pub params: Value,
    /// Unix seconds.
    pub created_at: u64,
    pub read: bool,
}

#[derive(Clone, Debug, Default)]
pub struct NewNotification {
    pub kind: String,
    pub level: String,
    pub title: String,
    pub body: String,
    pub params: Value,
    /// Same key twice = one row (for example a crash is reported once).
    pub dedupe: Option<String>,
}

#[async_trait]
pub trait NotificationRepo: Send + Sync {
    /// `None` when `dedupe` matched an existing row.
    async fn add(&self, n: &NewNotification, at: u64) -> Result<Option<i64>>;
    /// Newest first.
    async fn list(&self, limit: usize) -> Result<Vec<Notification>>;
    async fn unread(&self) -> Result<u64>;
    async fn mark_read(&self, ids: &[i64]) -> Result<()>;
    async fn mark_all_read(&self) -> Result<()>;
    async fn clear(&self) -> Result<()>;
}
