//! Scheduling (phase 10): warming launches and browser refresh. Never rotates hardware (D4).
//!
//! A [`Schedule`] targets one profile or every profile of a tag / group. Each firing writes one
//! [`ScheduleRun`] per profile into a durable history. Times are unix seconds; the recurrence is
//! evaluated in the schedule's IANA `timezone`.

use crate::{ProfileId, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", rename_all_fields = "camelCase", tag = "type")]
pub enum ScheduleTarget {
    Profile { id: ProfileId },
    Tag { id: i64 },
    Group { id: i64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", rename_all_fields = "camelCase", tag = "type")]
pub enum ScheduleAction {
    /// Warming launch: open each of `urls`, stay `dwell_secs` on each with light scrolling, then close.
    Warm { urls: Vec<String>, dwell_secs: u32 },
    /// Record the Camoufox version check. Moves nothing: the pinned hardware stays (D4).
    RefreshBrowser,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", rename_all_fields = "camelCase", tag = "type")]
pub enum Recurrence {
    /// Every day at `at` (`HH:MM`, schedule timezone).
    Daily { at: String },
    /// At `at` on `days` (0 = Monday ... 6 = Sunday).
    Weekdays { at: String, days: Vec<u8> },
    /// Every `hours` hours, counted from the schedule's `anchor`.
    EveryHours { hours: u32 },
}

/// Conditions checked before each profile run; failing ones end the run (see `cm-service` scheduler).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preconditions {
    /// The profile must not be running; waits up to `timeout_secs` for it to close.
    pub profile_closed: bool,
    /// The assigned proxy (if any) must pass a fresh check.
    pub proxy_check: bool,
    pub timeout_secs: u32,
}

impl Default for Preconditions {
    fn default() -> Self {
        Self { profile_closed: true, proxy_check: true, timeout_secs: 60 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    /// 0 on create.
    pub id: i64,
    pub name: String,
    pub target: ScheduleTarget,
    pub action: ScheduleAction,
    pub recurrence: Recurrence,
    /// IANA name, e.g. `Europe/Berlin`.
    pub timezone: String,
    pub enabled: bool,
    /// A run that is up to this late still starts; later ones are recorded as `skipped`.
    pub grace_minutes: u32,
    pub preconditions: Preconditions,
    /// Profiles of one firing that may run at the same time.
    pub max_concurrent: u32,
    /// Unix seconds of the next firing (`None` when disabled).
    pub next_run_at: Option<u64>,
    /// Anchor of `EveryHours` (unix seconds).
    pub anchor: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleRunOutcome {
    /// Still going (a run found in this state after a restart is closed as `failed`).
    Running,
    Ok,
    Failed,
    /// Did not run: missed past the grace window, profile busy, ...
    Skipped,
}

impl ScheduleRunOutcome {
    pub fn code(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Ok => "ok",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "ok" => Self::Ok,
            "failed" => Self::Failed,
            "skipped" => Self::Skipped,
            _ => Self::Running,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleRun {
    /// 0 before it is stored.
    pub id: i64,
    pub schedule_id: i64,
    /// `None` for runs that are not about one profile (missed firing, browser refresh).
    pub profile_id: Option<ProfileId>,
    /// The slot this run belongs to (unix seconds); equals `started_at` for "run now".
    pub scheduled_for: u64,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub outcome: ScheduleRunOutcome,
    pub message: Option<String>,
    /// `schedule` or `manual`.
    pub trigger: String,
}

#[async_trait]
pub trait ScheduleRepo: Send + Sync {
    async fn list(&self) -> Result<Vec<Schedule>>;
    async fn get(&self, id: i64) -> Result<Option<Schedule>>;
    /// Insert when `s.id == 0`, otherwise update. Returns the stored schedule.
    async fn save(&self, s: &Schedule) -> Result<Schedule>;
    /// Also removes its run history.
    async fn delete(&self, id: i64) -> Result<()>;
    async fn set_next_run(&self, id: i64, next: Option<u64>) -> Result<()>;
    /// Store a run (any outcome) and return its id.
    async fn start_run(&self, run: &ScheduleRun) -> Result<i64>;
    async fn finish_run(&self, id: i64, finished_at: u64, outcome: ScheduleRunOutcome, message: Option<&str>) -> Result<()>;
    /// Newest first. `schedule = None` = every schedule.
    async fn runs(&self, schedule: Option<i64>, limit: usize) -> Result<Vec<ScheduleRun>>;
    /// Close runs left `running` by a previous process as failed. Returns how many.
    async fn fail_running(&self, finished_at: u64, message: &str) -> Result<usize>;
}
