//! Scheduler tests with a fake clock, fake engine and in-memory SQLite.

use super::*;
use crate::{Config, Deps, tests::FakeChecker};
use async_trait::async_trait;
use cm_core::{
    Clock, Engine, EngineStatus, LaunchOptions, MachineSummary, NewProfile, NewProxy, Os, Preconditions, Profile, ScheduleRunOutcome as O,
    Session,
};
use cm_store::*;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

/// 2024-01-01 00:00:00 UTC (a Monday).
const T0: u64 = 1_704_067_200;

struct FakeClock(std::sync::atomic::AtomicU64);
impl FakeClock {
    fn set(&self, v: u64) {
        self.0.store(v, Ordering::SeqCst);
    }
}
impl Clock for FakeClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Default)]
struct TestEngine {
    browsed: Mutex<Vec<(String, Vec<String>, u32)>>,
    inflight: AtomicUsize,
    max_inflight: AtomicUsize,
    launched: AtomicUsize,
    fail_browse: bool,
}

#[async_trait]
impl Engine for TestEngine {
    fn name(&self) -> &'static str {
        "Fake"
    }
    fn ws_protocol(&self) -> &'static str {
        "Playwright"
    }
    async fn status(&self) -> EngineStatus {
        EngineStatus { ready: true, hint: None, version: Some("135.0".into()) }
    }
    async fn generate(&self, os: Os) -> Result<Value> {
        Ok(json!({ "os": os.code() }))
    }
    fn summarize(&self, _m: &Value) -> MachineSummary {
        MachineSummary::default()
    }
    async fn launch(&self, p: &cm_core::Profile, _o: &LaunchOptions) -> Result<Session> {
        self.launched.fetch_add(1, Ordering::SeqCst);
        Ok(Session { ws_url: format!("ws://127.0.0.1:1/{}", p.id) })
    }
    async fn stop(&self, _id: &str) -> Result<()> {
        Ok(())
    }
    async fn alive(&self, _id: &str) -> Option<bool> {
        Some(true)
    }
    fn data_dir(&self, id: &str) -> PathBuf {
        std::env::temp_dir().join("cm-fake-sched").join(id)
    }
    async fn delete_data(&self, _id: &str) -> Result<()> {
        Ok(())
    }
    async fn browse(&self, ws: &str, urls: &[String], dwell: u32) -> Result<()> {
        let now = self.inflight.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_inflight.fetch_max(now, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(40)).await;
        self.inflight.fetch_sub(1, Ordering::SeqCst);
        self.browsed.lock().unwrap().push((ws.to_string(), urls.to_vec(), dwell));
        if self.fail_browse { Err(anyhow::anyhow!("page crashed")) } else { Ok(()) }
    }
}

struct Rig {
    m: Arc<Manager>,
    engine: Arc<TestEngine>,
    clock: Arc<FakeClock>,
}

fn rig(proxy_ok: bool, fail_browse: bool) -> Rig {
    let db = Db::memory().unwrap();
    let engine = Arc::new(TestEngine { fail_browse, ..Default::default() });
    let clock = Arc::new(FakeClock(std::sync::atomic::AtomicU64::new(T0)));
    let deps = Deps {
        engine: engine.clone(),
        profiles: Arc::new(SqliteProfiles::new(db.clone())),
        proxies: Arc::new(SqliteProxies::new(db.clone())),
        checker: Arc::new(FakeChecker::new(proxy_ok)),
        tags: Arc::new(SqliteTags::new(db.clone())),
        groups: Arc::new(SqliteGroups::new(db.clone())),
        views: Arc::new(SqliteSavedViews::new(db.clone())),
        templates: Arc::new(SqliteTemplates::new(db.clone())),
        extensions: Arc::new(SqliteExtensions::new(db.clone())),
        schedules: Arc::new(SqliteSchedules::new(db.clone())),
        settings: Arc::new(SqliteSettings::new(db)),
        clock: clock.clone(),
    };
    let m = Arc::new(Manager::new(deps, Config { proxy_base_port: 0, data_dir: PathBuf::from("/tmp"), db_path: None }));
    Rig { m, engine, clock }
}

fn warm(target: ScheduleTarget) -> ScheduleInput {
    ScheduleInput {
        name: "warm".into(),
        target,
        action: ScheduleAction::Warm { urls: vec!["https://a.test".into(), "https://b.test".into()], dwell_secs: 10 },
        recurrence: Recurrence::Daily { at: "09:00".into() },
        timezone: "UTC".into(),
        enabled: true,
        grace_minutes: 10,
        preconditions: Preconditions { profile_closed: true, proxy_check: true, timeout_secs: 0 },
        max_concurrent: 1,
    }
}

async fn profile(m: &Manager, name: &str) -> Profile {
    let v = m.create_profile(NewProfile { name: name.into(), os: "mac".into() }).await.unwrap();
    m.profile(&v.id).await.unwrap()
}

async fn tick(r: &Rig) {
    for t in r.m.scheduler_tick().await.unwrap() {
        t.await.unwrap();
    }
}

async fn runs(r: &Rig, id: i64) -> Vec<ScheduleRun> {
    r.m.schedule_runs(Some(id), 50).await.unwrap()
}

const DUE: u64 = T0 + 9 * 3600;

#[test]
fn next_slots_follow_the_timezone() {
    let berlin = parse_tz("Europe/Berlin").unwrap();
    let daily = Recurrence::Daily { at: "09:00".into() };
    // 09:00 CET (winter) is 08:00 UTC.
    assert_eq!(next_after(&daily, berlin, 0, T0), Some(T0 + 8 * 3600));
    assert_eq!(next_after(&daily, parse_tz("UTC").unwrap(), 0, T0), Some(DUE));
    // Strictly after: asking at the slot itself gives tomorrow.
    assert_eq!(next_after(&daily, parse_tz("UTC").unwrap(), 0, DUE), Some(DUE + 86_400));
    // Summer: Berlin is UTC+2 (2024-07-01 00:00 UTC = 1719792000).
    assert_eq!(next_after(&daily, berlin, 0, 1_719_792_000), Some(1_719_792_000 + 7 * 3600));
    // Weekdays Wed (2) and Fri (4) from Monday 2024-01-01.
    let wk = Recurrence::Weekdays { at: "09:00".into(), days: vec![2, 4] };
    let utc = parse_tz("UTC").unwrap();
    assert_eq!(next_after(&wk, utc, 0, T0), Some(DUE + 2 * 86_400));
    assert_eq!(next_after(&wk, utc, 0, DUE + 2 * 86_400), Some(DUE + 4 * 86_400));
    // Every 6 hours from the anchor.
    let ev = Recurrence::EveryHours { hours: 6 };
    assert_eq!(next_after(&ev, utc, T0 + 100, T0 + 100), Some(T0 + 100 + 6 * 3600));
    assert_eq!(next_after(&ev, utc, T0, T0 + 13 * 3600), Some(T0 + 18 * 3600));
}

#[tokio::test]
async fn due_run_within_grace_launches_with_scheduler_lease_and_closes() {
    let r = rig(true, false);
    let p = profile(&r.m, "a").await;
    let s = r.m.create_schedule(warm(ScheduleTarget::Profile { id: p.id.clone() })).await.unwrap();
    assert_eq!(s.next_run_at, Some(DUE));

    tick(&r).await; // not due
    assert!(runs(&r, s.id).await.is_empty());

    r.clock.set(DUE + 120); // 2 minutes late, grace is 10
    tick(&r).await;
    let h = runs(&r, s.id).await;
    assert_eq!(h.len(), 1);
    assert_eq!((h[0].outcome, h[0].profile_id.as_deref(), h[0].trigger.as_str()), (O::Ok, Some(p.id.as_str()), "schedule"));
    assert_eq!(h[0].scheduled_for, DUE);
    assert!(h[0].finished_at.is_some());
    let b = r.engine.browsed.lock().unwrap().clone();
    assert_eq!(b.len(), 1);
    assert_eq!((b[0].1.len(), b[0].2), (2, 10));
    assert!(r.m.running.lock().await.is_empty(), "profile closed afterwards");
    assert!(r.m.lease_of(&p.id).is_none(), "lease released");
    assert_eq!(r.m.schedule(s.id).await.unwrap().next_run_at, Some(DUE + 86_400), "advanced to tomorrow");

    tick(&r).await; // the slot is consumed: nothing runs twice
    assert_eq!(runs(&r, s.id).await.len(), 1);
}

#[tokio::test]
async fn lease_holder_is_scheduler_while_running() {
    let r = rig(true, false);
    let p = profile(&r.m, "a").await;
    r.m.lease_acquire(&p.id, "agent-7", 86_400).await.unwrap();
    let s = r.m.create_schedule(warm(ScheduleTarget::Profile { id: p.id.clone() })).await.unwrap();
    r.clock.set(DUE);
    tick(&r).await;
    let h = runs(&r, s.id).await;
    assert_eq!(h[0].outcome, O::Skipped, "leased by an agent -> busy");
    assert!(r.engine.browsed.lock().unwrap().is_empty());
    r.m.lease_release(&p.id, None).await.unwrap();
    r.m.run_schedule_now(s.id).await.unwrap().await.unwrap();
    assert_eq!(runs(&r, s.id).await[0].outcome, O::Ok);
    assert!(r.engine.browsed.lock().unwrap()[0].0.ends_with(&p.id));
}

#[tokio::test]
async fn missed_beyond_grace_is_skipped_not_replayed() {
    let r = rig(true, false);
    let p = profile(&r.m, "a").await;
    let s = r.m.create_schedule(warm(ScheduleTarget::Profile { id: p.id.clone() })).await.unwrap();
    r.clock.set(DUE + 3 * 3600); // app was down for 3 hours; grace is 10 minutes
    tick(&r).await;
    let h = runs(&r, s.id).await;
    assert_eq!(h.len(), 1);
    assert_eq!(h[0].outcome, O::Skipped);
    assert!(h[0].message.as_deref().unwrap().contains("missed by 180 min"));
    assert!(r.engine.browsed.lock().unwrap().is_empty());
    assert_eq!(r.engine.launched.load(Ordering::SeqCst), 0);
    assert_eq!(r.m.schedule(s.id).await.unwrap().next_run_at, Some(DUE + 86_400));
}

#[tokio::test]
async fn disabled_schedule_never_fires_and_enable_recomputes() {
    let r = rig(true, false);
    let p = profile(&r.m, "a").await;
    let s = r.m.create_schedule(warm(ScheduleTarget::Profile { id: p.id.clone() })).await.unwrap();
    let off = r.m.set_schedule_enabled(s.id, false).await.unwrap();
    assert_eq!(off.next_run_at, None);
    r.clock.set(DUE + 1);
    tick(&r).await;
    assert!(runs(&r, s.id).await.is_empty());
    let on = r.m.set_schedule_enabled(s.id, true).await.unwrap();
    assert_eq!(on.next_run_at, Some(DUE + 86_400), "from now, not from the past slot");
}

#[tokio::test]
async fn precondition_profile_open_skips_run() {
    let r = rig(true, false);
    let p = profile(&r.m, "a").await;
    r.m.start(&p.id).await.unwrap(); // the user has it open
    let s = r.m.create_schedule(warm(ScheduleTarget::Profile { id: p.id.clone() })).await.unwrap();
    r.m.run_schedule_now(s.id).await.unwrap().await.unwrap();
    let h = runs(&r, s.id).await;
    assert_eq!(h[0].outcome, O::Skipped);
    assert!(h[0].message.as_deref().unwrap().contains("in use"));
    assert!(r.engine.browsed.lock().unwrap().is_empty());
    assert!(r.m.running.lock().await.contains_key(&p.id), "the user's browser is left alone");
}

#[tokio::test]
async fn precondition_proxy_check_failure_fails_run_without_launch() {
    let r = rig(false, false);
    let p = profile(&r.m, "a").await;
    let px =
        r.m.deps().proxies.insert(&NewProxy { name: "x".into(), host: "127.0.0.1".into(), port: 9, ..Default::default() }).await.unwrap();
    r.m.assign(&p.id, Some(px.id)).await.unwrap();
    let s = r.m.create_schedule(warm(ScheduleTarget::Profile { id: p.id.clone() })).await.unwrap();
    r.m.run_schedule_now(s.id).await.unwrap().await.unwrap();
    let h = runs(&r, s.id).await;
    assert_eq!(h[0].outcome, O::Failed);
    assert!(h[0].message.as_deref().unwrap().contains("proxy check failed"));
    assert_eq!(r.engine.launched.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn browse_error_is_failed_and_browser_still_closed() {
    let r = rig(true, true);
    let p = profile(&r.m, "a").await;
    let s = r.m.create_schedule(warm(ScheduleTarget::Profile { id: p.id.clone() })).await.unwrap();
    r.m.run_schedule_now(s.id).await.unwrap().await.unwrap();
    let h = runs(&r, s.id).await;
    assert_eq!((h[0].outcome, h[0].message.as_deref()), (O::Failed, Some("page crashed")));
    assert!(r.m.running.lock().await.is_empty());
    assert!(r.m.lease_of(&p.id).is_none());
}

#[tokio::test]
async fn group_target_respects_concurrency_limit() {
    let r = rig(true, false);
    let group = r.m.create_group("g", None).await.unwrap();
    let mut ids = Vec::new();
    for i in 0..5 {
        ids.push(profile(&r.m, &format!("p{i}")).await.id);
    }
    r.m.deps().groups.set(&ids, Some(group.id)).await.unwrap();
    let mut input = warm(ScheduleTarget::Group { id: group.id });
    input.max_concurrent = 2;
    let s = r.m.create_schedule(input).await.unwrap();
    r.m.run_schedule_now(s.id).await.unwrap().await.unwrap();
    let h = runs(&r, s.id).await;
    assert_eq!(h.len(), 5);
    assert!(h.iter().all(|x| x.outcome == O::Ok));
    assert_eq!(r.engine.max_inflight.load(Ordering::SeqCst), 2, "never more than max_concurrent at once");

    // The global cap holds across schedules too.
    r.engine.max_inflight.store(0, Ordering::SeqCst);
    let mut input = warm(ScheduleTarget::Group { id: group.id });
    input.max_concurrent = 10;
    let s2 = r.m.create_schedule(input).await.unwrap();
    r.m.run_schedule_now(s2.id).await.unwrap().await.unwrap();
    assert!(r.engine.max_inflight.load(Ordering::SeqCst) <= GLOBAL_MAX_RUNS);
}

#[tokio::test]
async fn refresh_browser_records_version_and_touches_nothing() {
    let r = rig(true, false);
    let p = profile(&r.m, "a").await;
    let mut input = warm(ScheduleTarget::Profile { id: p.id.clone() });
    input.action = ScheduleAction::RefreshBrowser;
    let s = r.m.create_schedule(input).await.unwrap();
    r.m.run_schedule_now(s.id).await.unwrap().await.unwrap();
    let h = runs(&r, s.id).await;
    assert_eq!(h.len(), 1);
    assert_eq!(h[0].outcome, O::Ok);
    assert!(h[0].message.as_deref().unwrap().contains("version 135.0"));
    assert_eq!(r.engine.launched.load(Ordering::SeqCst), 0);
    let after = r.m.profile(&p.id).await.unwrap();
    assert_eq!(after.machine, p.machine, "hardware never rotates");
}

#[tokio::test]
async fn restart_closes_interrupted_runs_and_missed_slots_follow_grace() {
    let r = rig(true, false);
    let p = profile(&r.m, "a").await;
    let s = r.m.create_schedule(warm(ScheduleTarget::Profile { id: p.id.clone() })).await.unwrap();
    let stale = ScheduleRun {
        id: 0,
        schedule_id: s.id,
        profile_id: Some(p.id.clone()),
        scheduled_for: T0,
        started_at: T0,
        finished_at: None,
        outcome: O::Running,
        message: None,
        trigger: "schedule".into(),
    };
    r.m.deps().schedules.start_run(&stale).await.unwrap();
    let h = r.m.clone().spawn_scheduler();
    tokio::time::sleep(Duration::from_millis(100)).await;
    h.abort();
    let rows = runs(&r, s.id).await;
    assert_eq!(rows[0].outcome, O::Failed);
    assert!(rows[0].message.as_deref().unwrap().contains("interrupted"));
}

#[tokio::test]
async fn validation_rejects_bad_input() {
    let r = rig(true, false);
    let p = profile(&r.m, "a").await;
    let t = ScheduleTarget::Profile { id: p.id.clone() };
    let bad = |f: fn(&mut ScheduleInput)| {
        let mut i = warm(t.clone());
        f(&mut i);
        i
    };
    for i in [
        bad(|i| i.name = " ".into()),
        bad(|i| i.timezone = "Mars/Base".into()),
        bad(|i| i.recurrence = Recurrence::Daily { at: "25:99".into() }),
        bad(|i| i.recurrence = Recurrence::Weekdays { at: "09:00".into(), days: vec![] }),
        bad(|i| i.recurrence = Recurrence::EveryHours { hours: 0 }),
        bad(|i| i.action = ScheduleAction::Warm { urls: vec!["file:///etc/passwd".into()], dwell_secs: 10 }),
        bad(|i| i.action = ScheduleAction::Warm { urls: vec![], dwell_secs: 10 }),
        bad(|i| i.action = ScheduleAction::Warm { urls: vec!["https://a.test".into()], dwell_secs: 1 }),
        bad(|i| i.max_concurrent = 0),
        bad(|i| i.target = ScheduleTarget::Profile { id: "nope".into() }),
        bad(|i| i.target = ScheduleTarget::Tag { id: 999 }),
    ] {
        assert!(r.m.create_schedule(i).await.is_err());
    }
}
