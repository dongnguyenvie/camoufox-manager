//! Scheduling (phase 10): warming launches and browser refresh. Never rotates hardware (D4).

use crate::{ProfileId, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ScheduleAction {
    /// Open `urls`, keep the browser `minutes`, then close.
    Launch { urls: Vec<String>, minutes: u32 },
    /// Move the browser version only (what a real machine does on update).
    RefreshBrowser,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ScheduleKind {
    /// Every N minutes.
    Interval { minutes: u32 },
    /// At `HH:MM` local time on weekdays `days` (0 = Monday).
    Daily { at: String, days: Vec<u8> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    pub id: i64,
    pub profile_id: ProfileId,
    pub action: ScheduleAction,
    pub kind: ScheduleKind,
    pub enabled: bool,
    /// Unix seconds.
    pub next_run_at: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleRunOutcome {
    Ok,
    /// e.g. profile already running.
    Skipped,
    Error,
    /// Fell due while the app was down; logged, not replayed.
    Missed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleRun {
    pub id: i64,
    pub schedule_id: i64,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub outcome: ScheduleRunOutcome,
    pub message: Option<String>,
}

#[async_trait]
pub trait ScheduleRepo: Send + Sync {
    async fn list(&self) -> Result<Vec<Schedule>>;
    /// Insert when `s.id == 0`, otherwise update.
    async fn save(&self, s: &Schedule) -> Result<Schedule>;
    async fn delete(&self, id: i64) -> Result<()>;
    async fn log_run(&self, run: &ScheduleRun) -> Result<()>;
    async fn runs(&self, schedule: i64, limit: usize) -> Result<Vec<ScheduleRun>>;
}
