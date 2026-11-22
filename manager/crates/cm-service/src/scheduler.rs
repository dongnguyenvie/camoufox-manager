//! Scheduling (phase 10): warming launches and browser refresh. Never rotates hardware (D4).
//!
//! - A [`Schedule`] targets one profile or a tag / group and fires by its recurrence (daily at a
//!   time, chosen weekdays, every N hours) evaluated in the schedule's own IANA timezone.
//! - State lives in SQLite (`next_run_at`, run history), so the loop survives an app restart:
//!   the next tick finds the schedules that fell due while the app was down, runs them when they
//!   are inside the grace window and records `skipped` otherwise. Slots are never replayed.
//! - [`Manager::scheduler_tick`] does one pass against the injected `Clock`, so tests drive it
//!   with a fake clock; [`Manager::spawn_scheduler`] calls it every few seconds.
//! - One run per profile: preconditions (profile closed, proxy check passes; waits up to the
//!   timeout) -> lease holder `scheduler` -> start -> browse -> stop (stop releases the lease).
//!   A busy profile gives `skipped`, a failing proxy check or a launch/browse error `failed`.
//! - Concurrency: `max_concurrent` profiles per firing, plus a global cap on scheduled runs.

use crate::Manager;
use anyhow::Result;
use chrono::{Datelike, Duration as ChronoDuration, LocalResult, NaiveTime, TimeZone};
use chrono_tz::Tz;
use cm_core::{CmError, Preconditions, ProfileId, Recurrence, Schedule, ScheduleAction, ScheduleRun, ScheduleRunOutcome, ScheduleTarget};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::Semaphore, task::JoinHandle};

/// Lease holder name of scheduled runs.
pub const HOLDER: &str = "scheduler";
/// Scheduled profile runs allowed at once, across all schedules.
pub const GLOBAL_MAX_RUNS: usize = 4;
/// How often the loop looks for due schedules.
const TICK_SECS: u64 = 10;

/// Runtime state of the scheduler (kept in [`Manager`]).
pub(crate) struct SchedState {
    /// Schedules with a firing in progress (a firing never overlaps itself).
    active: Mutex<HashSet<i64>>,
    global: Arc<Semaphore>,
}

impl Default for SchedState {
    fn default() -> Self {
        Self { active: Mutex::default(), global: Arc::new(Semaphore::new(GLOBAL_MAX_RUNS)) }
    }
}

// ---------------------------------------------------------------------------------------------
// Recurrence math
// ---------------------------------------------------------------------------------------------

fn parse_at(at: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(at.trim(), "%H:%M").map_err(|_| CmError::Invalid(format!("time must be HH:MM, got {at:?}")).into())
}

pub fn parse_tz(name: &str) -> Result<Tz> {
    name.trim()
        .parse::<Tz>()
        .map_err(|_| CmError::Invalid(format!("unknown timezone {name:?} (use an IANA name such as Europe/Berlin)")).into())
}

/// Smallest slot of `rec` strictly after `after` (unix seconds). `anchor` only matters for
/// `EveryHours`, which fires at `anchor + k * hours`.
pub fn next_after(rec: &Recurrence, tz: Tz, anchor: u64, after: u64) -> Option<u64> {
    match rec {
        Recurrence::EveryHours { hours } => {
            let step = u64::from((*hours).max(1)) * 3600;
            Some(if after < anchor { anchor } else { anchor + ((after - anchor) / step + 1) * step })
        }
        Recurrence::Daily { at } => next_local(at, None, tz, after),
        Recurrence::Weekdays { at, days } => next_local(at, Some(days), tz, after),
    }
}

fn next_local(at: &str, days: Option<&Vec<u8>>, tz: Tz, after: u64) -> Option<u64> {
    let time = parse_at(at).ok()?;
    let start = tz.timestamp_opt(i64::try_from(after).ok()?, 0).single()?.date_naive();
    for d in 0..=8 {
        let date = start + ChronoDuration::days(d);
        if days.is_some_and(|ds| !ds.contains(&(date.weekday().num_days_from_monday() as u8))) {
            continue;
        }
        let naive = date.and_time(time);
        let local = match tz.from_local_datetime(&naive) {
            LocalResult::Single(x) | LocalResult::Ambiguous(x, _) => x,
            // The wall time does not exist (spring-forward gap): fire an hour later.
            LocalResult::None => match tz.from_local_datetime(&(naive + ChronoDuration::hours(1))) {
                LocalResult::Single(x) | LocalResult::Ambiguous(x, _) => x,
                LocalResult::None => continue,
            },
        };
        let ts = u64::try_from(local.timestamp()).ok()?;
        if ts > after {
            return Some(ts);
        }
    }
    None
}

// ---------------------------------------------------------------------------------------------
// Input / views
// ---------------------------------------------------------------------------------------------

fn yes() -> bool {
    true
}
fn default_grace() -> u32 {
    10
}
fn one() -> u32 {
    1
}

/// What the API accepts to create or replace a schedule.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleInput {
    pub name: String,
    pub target: ScheduleTarget,
    pub action: ScheduleAction,
    pub recurrence: Recurrence,
    pub timezone: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "default_grace")]
    pub grace_minutes: u32,
    #[serde(default)]
    pub preconditions: Preconditions,
    #[serde(default = "one")]
    pub max_concurrent: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleView {
    #[serde(flatten)]
    pub schedule: Schedule,
    /// Profile / tag / group name for display.
    pub target_label: String,
    pub last_run: Option<ScheduleRun>,
    /// A firing of this schedule is in progress right now.
    pub running: bool,
}

fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(CmError::Invalid(msg.into()).into())
}

fn validate(i: &ScheduleInput) -> Result<()> {
    if i.name.trim().is_empty() || i.name.chars().count() > 80 {
        return invalid("name must be 1-80 characters");
    }
    parse_tz(&i.timezone)?;
    match &i.recurrence {
        Recurrence::Daily { at } => {
            parse_at(at)?;
        }
        Recurrence::Weekdays { at, days } => {
            parse_at(at)?;
            if days.is_empty() || days.iter().any(|d| *d > 6) {
                return invalid("pick at least one weekday (0 = Monday ... 6 = Sunday)");
            }
        }
        Recurrence::EveryHours { hours } => {
            if !(1..=720).contains(hours) {
                return invalid("hours must be 1-720");
            }
        }
    }
    if let ScheduleAction::Warm { urls, dwell_secs } = &i.action {
        if urls.is_empty() || urls.len() > 20 {
            return invalid("a warming launch needs 1-20 URLs");
        }
        if let Some(u) =
            urls.iter().find(|u| !(u.starts_with("http://") || u.starts_with("https://")) || u.chars().any(char::is_whitespace))
        {
            return invalid(format!("not an http(s) URL: {u}"));
        }
        if !(5..=3600).contains(dwell_secs) {
            return invalid("dwell per URL must be 5 seconds to 60 minutes");
        }
    }
    if i.grace_minutes > 1440 {
        return invalid("grace window is at most 1440 minutes");
    }
    if !(1..=10).contains(&i.max_concurrent) {
        return invalid("max concurrent runs must be 1-10");
    }
    if i.preconditions.timeout_secs > 3600 {
        return invalid("precondition timeout is at most 3600 seconds");
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Manager: schedules CRUD
// ---------------------------------------------------------------------------------------------

impl Manager {
    async fn check_target(&self, t: &ScheduleTarget) -> Result<String> {
        match t {
            ScheduleTarget::Profile { id } => Ok(self.profile(id).await?.name),
            ScheduleTarget::Tag { id } => self
                .d
                .tags
                .list()
                .await?
                .into_iter()
                .find(|x| x.id == *id)
                .map(|x| x.name)
                .ok_or_else(|| CmError::NotFound(format!("tag #{id}")).into()),
            ScheduleTarget::Group { id } => self
                .d
                .groups
                .list()
                .await?
                .into_iter()
                .find(|x| x.id == *id)
                .map(|x| x.name)
                .ok_or_else(|| CmError::NotFound(format!("group #{id}")).into()),
        }
    }

    pub async fn schedules(&self) -> Result<Vec<ScheduleView>> {
        let mut out = Vec::new();
        let active = self.sched.active.lock().unwrap().clone();
        for s in self.d.schedules.list().await? {
            let label = self.check_target(&s.target).await.unwrap_or_else(|_| "(deleted)".into());
            let last_run = self.d.schedules.runs(Some(s.id), 1).await?.into_iter().next();
            out.push(ScheduleView { running: active.contains(&s.id), target_label: label, last_run, schedule: s });
        }
        Ok(out)
    }

    pub async fn schedule(&self, id: i64) -> Result<Schedule> {
        self.d.schedules.get(id).await?.ok_or_else(|| CmError::NotFound(format!("schedule #{id}")).into())
    }

    fn compute_next(&self, s: &Schedule, after: u64) -> Option<u64> {
        if !s.enabled {
            return None;
        }
        parse_tz(&s.timezone).ok().and_then(|tz| next_after(&s.recurrence, tz, s.anchor, after))
    }

    pub async fn create_schedule(&self, i: ScheduleInput) -> Result<Schedule> {
        validate(&i)?;
        self.check_target(&i.target).await?;
        let now = self.d.clock.now();
        let mut s = Schedule {
            id: 0,
            name: i.name.trim().to_string(),
            target: i.target,
            action: i.action,
            recurrence: i.recurrence,
            timezone: i.timezone.trim().to_string(),
            enabled: i.enabled,
            grace_minutes: i.grace_minutes,
            preconditions: i.preconditions,
            max_concurrent: i.max_concurrent,
            next_run_at: None,
            anchor: now,
        };
        s.next_run_at = self.compute_next(&s, now);
        self.d.schedules.save(&s).await
    }

    pub async fn update_schedule(&self, id: i64, i: ScheduleInput) -> Result<Schedule> {
        validate(&i)?;
        let old = self.schedule(id).await?;
        let same_recurrence = old.recurrence == i.recurrence;
        self.check_target(&i.target).await?;
        let now = self.d.clock.now();
        let mut s = Schedule {
            id,
            name: i.name.trim().to_string(),
            target: i.target,
            action: i.action,
            recurrence: i.recurrence,
            timezone: i.timezone.trim().to_string(),
            enabled: i.enabled,
            grace_minutes: i.grace_minutes,
            preconditions: i.preconditions,
            max_concurrent: i.max_concurrent,
            next_run_at: None,
            anchor: if same_recurrence { old.anchor } else { now },
        };
        s.next_run_at = self.compute_next(&s, now);
        self.d.schedules.save(&s).await
    }

    pub async fn set_schedule_enabled(&self, id: i64, enabled: bool) -> Result<Schedule> {
        let mut s = self.schedule(id).await?;
        s.enabled = enabled;
        s.next_run_at = self.compute_next(&s, self.d.clock.now());
        self.d.schedules.save(&s).await
    }

    pub async fn delete_schedule(&self, id: i64) -> Result<()> {
        self.d.schedules.delete(id).await
    }

    pub async fn schedule_runs(&self, schedule: Option<i64>, limit: usize) -> Result<Vec<ScheduleRun>> {
        if let Some(id) = schedule {
            self.schedule(id).await?;
        }
        self.d.schedules.runs(schedule, limit.clamp(1, 500)).await
    }

    // -----------------------------------------------------------------------------------------
    // The loop
    // -----------------------------------------------------------------------------------------

    /// Start the background loop. Runs left `running` by a previous process are closed as failed
    /// first; schedules without a next slot get one.
    pub fn spawn_scheduler(self: Arc<Self>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let now = self.d.clock.now();
            match self.d.schedules.fail_running(now, "interrupted: the app was closed during the run").await {
                Ok(n) if n > 0 => println!("scheduler: {n} interrupted run(s) marked failed"),
                Ok(_) => {}
                Err(e) => eprintln!("scheduler: {e:#}"),
            }
            // CM-3.1: purge expired trash at startup, then hourly.
            let mut last_purge: Option<std::time::Instant> = None;
            loop {
                let _ = self.scheduler_tick().await.map_err(|e| eprintln!("scheduler: {e:#}"));
                if last_purge.is_none_or(|t| t.elapsed() >= Duration::from_secs(3600)) {
                    last_purge = Some(std::time::Instant::now());
                    let _ = self.purge_expired_trash().await.map_err(|e| eprintln!("trash: {e:#}"));
                }
                tokio::time::sleep(Duration::from_secs(TICK_SECS)).await;
            }
        })
    }

    /// One pass: start every enabled schedule whose slot is due. Returns the spawned run tasks
    /// (the loop ignores them; tests await them).
    pub async fn scheduler_tick(self: &Arc<Self>) -> Result<Vec<JoinHandle<()>>> {
        let now = self.d.clock.now();
        let mut tasks = Vec::new();
        for s in self.d.schedules.list().await? {
            if !s.enabled {
                continue;
            }
            let Some(due) = s.next_run_at.or_else(|| self.compute_next(&s, now)) else { continue };
            if due > now {
                if s.next_run_at != Some(due) {
                    self.d.schedules.set_next_run(s.id, Some(due)).await?;
                }
                continue;
            }
            // Persist the next slot BEFORE running, so a crash or restart never repeats this one.
            let next = parse_tz(&s.timezone).ok().and_then(|tz| next_after(&s.recurrence, tz, s.anchor, now));
            self.d.schedules.set_next_run(s.id, next).await?;
            let late = now - due;
            if late > u64::from(s.grace_minutes) * 60 {
                let msg = format!("missed by {} min (grace window {} min)", late / 60, s.grace_minutes);
                self.log_run(&s, None, due, ScheduleRunOutcome::Skipped, &msg, "schedule").await?;
                continue;
            }
            if let Some(t) = self.fire(&s, due, "schedule").await? {
                tasks.push(t);
            }
        }
        Ok(tasks)
    }

    /// "Run now": fire the schedule immediately, whatever its time or grace. Returns at once;
    /// progress shows in the run history.
    pub async fn run_schedule_now(self: &Arc<Self>, id: i64) -> Result<JoinHandle<()>> {
        let s = self.schedule(id).await?;
        let now = self.d.clock.now();
        match self.fire(&s, now, "manual").await? {
            Some(t) => Ok(t),
            None => Err(CmError::Conflict("this schedule is already running".into()).into()),
        }
    }

    /// Spawn one firing unless the schedule is already running (then record `skipped`).
    async fn fire(self: &Arc<Self>, s: &Schedule, due: u64, trigger: &'static str) -> Result<Option<JoinHandle<()>>> {
        if !self.sched.active.lock().unwrap().insert(s.id) {
            if trigger == "schedule" {
                self.log_run(s, None, due, ScheduleRunOutcome::Skipped, "the previous firing is still running", trigger).await?;
            }
            return Ok(None);
        }
        let (m, s) = (self.clone(), s.clone());
        Ok(Some(tokio::spawn(async move {
            if let Err(e) = m.execute(&s, due, trigger).await {
                let _ = m.log_run(&s, None, due, ScheduleRunOutcome::Failed, &format!("{e:#}"), trigger).await;
            }
            m.sched.active.lock().unwrap().remove(&s.id);
        })))
    }

    async fn log_run(
        &self,
        s: &Schedule,
        profile: Option<&str>,
        due: u64,
        outcome: ScheduleRunOutcome,
        msg: &str,
        trigger: &str,
    ) -> Result<()> {
        let now = self.d.clock.now();
        let run = ScheduleRun {
            id: 0,
            schedule_id: s.id,
            profile_id: profile.map(str::to_string),
            scheduled_for: due,
            started_at: now,
            finished_at: Some(now),
            outcome,
            message: Some(msg.to_string()),
            trigger: trigger.to_string(),
        };
        self.d.schedules.start_run(&run).await?;
        self.notify_schedule_run(s, profile, outcome, msg).await;
        Ok(())
    }

    async fn targets(&self, t: &ScheduleTarget) -> Result<Vec<ProfileId>> {
        let mut ids: Vec<ProfileId> = match t {
            ScheduleTarget::Profile { id } => vec![self.profile(id).await?.id],
            ScheduleTarget::Tag { id } => {
                self.d.tags.all_assignments().await?.into_iter().filter(|(_, t)| t == id).map(|(p, _)| p).collect()
            }
            ScheduleTarget::Group { id } => {
                self.d.groups.all_assignments().await?.into_iter().filter(|(_, g)| g == id).map(|(p, _)| p).collect()
            }
        };
        ids.sort();
        ids.dedup();
        Ok(ids)
    }

    async fn execute(self: &Arc<Self>, s: &Schedule, due: u64, trigger: &str) -> Result<()> {
        if matches!(s.action, ScheduleAction::RefreshBrowser) {
            let st = self.d.engine.status().await;
            let (outcome, msg) = if st.ready {
                let v = st.version.map_or_else(|| "version unknown".to_string(), |v| format!("version {v}"));
                (ScheduleRunOutcome::Ok, format!("{} {v}; pinned hardware unchanged", self.d.engine.name()))
            } else {
                (ScheduleRunOutcome::Failed, st.hint.map_or_else(|| "engine not ready".into(), |h| h.detail))
            };
            return self.log_run(s, None, due, outcome, &msg, trigger).await;
        }
        let ids = self.targets(&s.target).await?;
        if ids.is_empty() {
            return self.log_run(s, None, due, ScheduleRunOutcome::Skipped, "the target has no profiles", trigger).await;
        }
        let per_firing = Arc::new(Semaphore::new(s.max_concurrent.max(1) as usize));
        let mut set = tokio::task::JoinSet::new();
        for id in ids {
            let (m, s, sem, trigger) = (self.clone(), s.clone(), per_firing.clone(), trigger.to_string());
            set.spawn(async move {
                let Ok(_a) = sem.acquire().await else { return };
                let Ok(_b) = m.sched.global.clone().acquire_owned().await else { return };
                m.profile_run(&s, &id, due, &trigger).await;
            });
        }
        while set.join_next().await.is_some() {}
        Ok(())
    }

    /// One profile of a firing: history row `running` -> work -> final outcome.
    async fn profile_run(&self, s: &Schedule, profile: &str, due: u64, trigger: &str) {
        let row = ScheduleRun {
            id: 0,
            schedule_id: s.id,
            profile_id: Some(profile.to_string()),
            scheduled_for: due,
            started_at: self.d.clock.now(),
            finished_at: None,
            outcome: ScheduleRunOutcome::Running,
            message: None,
            trigger: trigger.to_string(),
        };
        let Ok(id) = self.d.schedules.start_run(&row).await else { return };
        let (outcome, msg) = match self.run_one(s, profile).await {
            Ok(r) => r,
            Err(e) => (ScheduleRunOutcome::Failed, format!("{e:#}")),
        };
        let _ = self.d.schedules.finish_run(id, self.d.clock.now(), outcome, Some(&msg)).await;
        self.notify_schedule_run(s, Some(profile), outcome, &msg).await;
    }

    async fn is_busy(&self, profile: &str) -> bool {
        self.prune_dead().await;
        self.running.lock().await.contains_key(profile) || self.lease_of(profile).is_some()
    }

    async fn run_one(&self, s: &Schedule, profile: &str) -> Result<(ScheduleRunOutcome, String)> {
        use ScheduleRunOutcome::{Failed, Ok as Done, Skipped};
        let p = self.profile(profile).await?;
        let pre = &s.preconditions;
        // Precondition: profile closed (wait up to the timeout; without it, a busy profile is skipped).
        let wait = if pre.profile_closed { Duration::from_secs(u64::from(pre.timeout_secs)) } else { Duration::ZERO };
        let began = tokio::time::Instant::now();
        while self.is_busy(profile).await {
            if began.elapsed() >= wait {
                return Ok((Skipped, format!("profile is in use (still open after {}s)", wait.as_secs())));
            }
            tokio::time::sleep(Duration::from_secs(1).min(wait)).await;
        }
        // Precondition: the proxy answers through itself.
        if pre.proxy_check
            && let Some(px) = p.proxy_id
        {
            let limit = Duration::from_secs(u64::from(pre.timeout_secs.max(5)));
            match tokio::time::timeout(limit, self.check_proxy(px, true)).await {
                Err(_) => return Ok((Failed, format!("proxy check timed out after {}s", limit.as_secs()))),
                Ok(Err(e)) => return Ok((Failed, format!("proxy check failed: {e:#}"))),
                Ok(Ok(_)) => {}
            }
        }
        let ScheduleAction::Warm { urls, dwell_secs } = &s.action else { return Ok((Failed, "not a launch schedule".into())) };
        let total = u64::from(*dwell_secs) * urls.len() as u64;
        if let Err(e) = self.lease_acquire(profile, HOLDER, total + 600).await {
            return Ok((Skipped, format!("{e:#}")));
        }
        let started = match self.start_as(profile, Some(HOLDER)).await {
            Ok(r) => r,
            Err(e) => {
                let _ = self.lease_release(profile, Some(HOLDER)).await;
                return Ok((Failed, format!("start failed: {e:#}")));
            }
        };
        if started.already {
            // Opened by someone else between the check and the start: never drive or close it.
            let _ = self.lease_release(profile, Some(HOLDER)).await;
            return Ok((Skipped, "profile was opened by someone else".into()));
        }
        let browsed = tokio::time::timeout(Duration::from_secs(total + 120), self.d.engine.browse(&started.ws, urls, *dwell_secs)).await;
        let stopped = self.stop(profile).await; // also releases the lease
        let res = match browsed {
            Err(_) => Err(anyhow::anyhow!("browsing took longer than expected")),
            Ok(r) => r,
        };
        Ok(match (res, stopped) {
            (Err(e), _) => (Failed, format!("{e:#}")),
            (Ok(()), Err(e)) => (Failed, format!("browsed, but closing failed: {e:#}")),
            (Ok(()), Ok(())) => (Done, format!("{} URL(s), {}s each, then closed", urls.len(), dwell_secs)),
        })
    }
}

#[cfg(test)]
mod tests;
