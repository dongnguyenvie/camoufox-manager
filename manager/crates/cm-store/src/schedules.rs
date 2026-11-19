//! Schedule store (tables in `0006_schedules.sql`): schedules and the durable run history.

use crate::Db;
use anyhow::{Context, Result};
use async_trait::async_trait;
use cm_core::{CmError, Schedule, ScheduleRepo, ScheduleRun, ScheduleRunOutcome, ScheduleTarget};
use rusqlite::{Row, params};

pub struct SqliteSchedules {
    db: Db,
}

impl SqliteSchedules {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

const COLS: &str = "id, name, target_kind, target_id, action_json, recurrence_json, timezone, enabled, grace_minutes, \
                    precheck_json, max_concurrent, next_run_at, anchor";

fn target_parts(t: &ScheduleTarget) -> (&'static str, String) {
    match t {
        ScheduleTarget::Profile { id } => ("profile", id.clone()),
        ScheduleTarget::Tag { id } => ("tag", id.to_string()),
        ScheduleTarget::Group { id } => ("group", id.to_string()),
    }
}

fn target_from(kind: &str, id: &str) -> Result<ScheduleTarget> {
    Ok(match kind {
        "profile" => ScheduleTarget::Profile { id: id.to_string() },
        "tag" => ScheduleTarget::Tag { id: id.parse().context("bad tag id")? },
        "group" => ScheduleTarget::Group { id: id.parse().context("bad group id")? },
        other => anyhow::bail!("unknown schedule target {other}"),
    })
}

fn schedule_row(r: &Row) -> rusqlite::Result<Result<Schedule>> {
    let kind: String = r.get(2)?;
    let tid: String = r.get(3)?;
    let (action, rec, pre): (String, String, String) = (r.get(4)?, r.get(5)?, r.get(9)?);
    let s = Schedule {
        id: r.get(0)?,
        name: r.get(1)?,
        target: ScheduleTarget::Profile { id: String::new() },
        action: serde_json::from_str(&action).unwrap_or(cm_core::ScheduleAction::RefreshBrowser),
        recurrence: serde_json::from_str(&rec).unwrap_or(cm_core::Recurrence::Daily { at: "00:00".into() }),
        timezone: r.get(6)?,
        enabled: r.get::<_, i64>(7)? != 0,
        grace_minutes: r.get(8)?,
        preconditions: serde_json::from_str(&pre).unwrap_or_default(),
        max_concurrent: r.get(10)?,
        next_run_at: r.get::<_, Option<i64>>(11)?.map(|v| v as u64),
        anchor: r.get::<_, i64>(12)? as u64,
    };
    Ok(target_from(&kind, &tid).map(|target| Schedule { target, ..s }))
}

fn run_row(r: &Row) -> rusqlite::Result<ScheduleRun> {
    Ok(ScheduleRun {
        id: r.get(0)?,
        schedule_id: r.get(1)?,
        profile_id: r.get(2)?,
        scheduled_for: r.get::<_, i64>(3)? as u64,
        started_at: r.get::<_, i64>(4)? as u64,
        finished_at: r.get::<_, Option<i64>>(5)?.map(|v| v as u64),
        outcome: ScheduleRunOutcome::parse(&r.get::<_, String>(6)?),
        message: r.get(7)?,
        trigger: r.get(8)?,
    })
}

const RUN_COLS: &str = "id, schedule_id, profile_id, scheduled_for, started_at, finished_at, outcome, message, trigger";

#[async_trait]
impl ScheduleRepo for SqliteSchedules {
    async fn list(&self) -> Result<Vec<Schedule>> {
        self.db.with(|c| {
            let mut st = c.prepare(&format!("SELECT {COLS} FROM schedules ORDER BY id"))?;
            let rows = st.query_map([], schedule_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
            rows.into_iter().collect()
        })
    }

    async fn get(&self, id: i64) -> Result<Option<Schedule>> {
        self.db.with(|c| {
            let mut st = c.prepare(&format!("SELECT {COLS} FROM schedules WHERE id = ?1"))?;
            let mut rows = st.query_map([id], schedule_row)?;
            rows.next().transpose()?.transpose()
        })
    }

    async fn save(&self, s: &Schedule) -> Result<Schedule> {
        let (kind, tid) = target_parts(&s.target);
        let (action, rec, pre) =
            (serde_json::to_string(&s.action)?, serde_json::to_string(&s.recurrence)?, serde_json::to_string(&s.preconditions)?);
        let next = s.next_run_at.map(|v| v as i64);
        let id = self.db.with(|c| {
            if s.id == 0 {
                c.execute(
                    "INSERT INTO schedules (name, target_kind, target_id, action_json, recurrence_json, timezone, enabled, \
                     grace_minutes, precheck_json, max_concurrent, next_run_at, anchor) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                    params![
                        s.name,
                        kind,
                        tid,
                        action,
                        rec,
                        s.timezone,
                        s.enabled,
                        s.grace_minutes,
                        pre,
                        s.max_concurrent,
                        next,
                        s.anchor as i64
                    ],
                )?;
                Ok(c.last_insert_rowid())
            } else {
                let n = c.execute(
                    "UPDATE schedules SET name=?2, target_kind=?3, target_id=?4, action_json=?5, recurrence_json=?6, timezone=?7, \
                     enabled=?8, grace_minutes=?9, precheck_json=?10, max_concurrent=?11, next_run_at=?12, anchor=?13 WHERE id=?1",
                    params![
                        s.id,
                        s.name,
                        kind,
                        tid,
                        action,
                        rec,
                        s.timezone,
                        s.enabled,
                        s.grace_minutes,
                        pre,
                        s.max_concurrent,
                        next,
                        s.anchor as i64
                    ],
                )?;
                if n == 0 {
                    return Err(CmError::NotFound(format!("schedule #{}", s.id)).into());
                }
                Ok(s.id)
            }
        })?;
        Ok(Schedule { id, ..s.clone() })
    }

    async fn delete(&self, id: i64) -> Result<()> {
        let n = self.db.with(|c| Ok(c.execute("DELETE FROM schedules WHERE id = ?1", [id])?))?;
        if n == 0 { Err(CmError::NotFound(format!("schedule #{id}")).into()) } else { Ok(()) }
    }

    async fn set_next_run(&self, id: i64, next: Option<u64>) -> Result<()> {
        self.db.with(|c| {
            c.execute("UPDATE schedules SET next_run_at = ?2 WHERE id = ?1", params![id, next.map(|v| v as i64)])?;
            Ok(())
        })
    }

    async fn start_run(&self, r: &ScheduleRun) -> Result<i64> {
        self.db.with(|c| {
            c.execute(
                "INSERT INTO schedule_runs (schedule_id, profile_id, scheduled_for, started_at, finished_at, outcome, message, trigger) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    r.schedule_id,
                    r.profile_id,
                    r.scheduled_for as i64,
                    r.started_at as i64,
                    r.finished_at.map(|v| v as i64),
                    r.outcome.code(),
                    r.message,
                    r.trigger
                ],
            )?;
            Ok(c.last_insert_rowid())
        })
    }

    async fn finish_run(&self, id: i64, finished_at: u64, outcome: ScheduleRunOutcome, message: Option<&str>) -> Result<()> {
        self.db.with(|c| {
            c.execute(
                "UPDATE schedule_runs SET finished_at=?2, outcome=?3, message=?4 WHERE id=?1",
                params![id, finished_at as i64, outcome.code(), message],
            )?;
            Ok(())
        })
    }

    async fn runs(&self, schedule: Option<i64>, limit: usize) -> Result<Vec<ScheduleRun>> {
        self.db.with(|c| {
            let mut st = c.prepare(&format!(
                "SELECT {RUN_COLS} FROM schedule_runs WHERE (?1 IS NULL OR schedule_id = ?1) ORDER BY id DESC LIMIT ?2"
            ))?;
            Ok(st.query_map(params![schedule, limit as i64], run_row)?.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    async fn fail_running(&self, finished_at: u64, message: &str) -> Result<usize> {
        self.db.with(|c| {
            Ok(c.execute(
                "UPDATE schedule_runs SET finished_at=?1, outcome='failed', message=?2 WHERE outcome='running'",
                params![finished_at as i64, message],
            )?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cm_core::{Preconditions, Recurrence, ScheduleAction};

    fn sample() -> Schedule {
        Schedule {
            id: 0,
            name: "warm".into(),
            target: ScheduleTarget::Tag { id: 3 },
            action: ScheduleAction::Warm { urls: vec!["https://a.test".into()], dwell_secs: 30 },
            recurrence: Recurrence::Weekdays { at: "09:30".into(), days: vec![0, 2] },
            timezone: "Europe/Berlin".into(),
            enabled: true,
            grace_minutes: 15,
            preconditions: Preconditions::default(),
            max_concurrent: 2,
            next_run_at: Some(1000),
            anchor: 5,
        }
    }

    #[tokio::test]
    async fn roundtrip_and_history() {
        let repo = SqliteSchedules::new(Db::memory().unwrap());
        let s = repo.save(&sample()).await.unwrap();
        assert!(s.id > 0);
        assert_eq!(repo.get(s.id).await.unwrap().unwrap(), s);
        let upd = Schedule { name: "x".into(), enabled: false, ..s.clone() };
        repo.save(&upd).await.unwrap();
        assert_eq!(repo.list().await.unwrap(), vec![upd]);

        let run = ScheduleRun {
            id: 0,
            schedule_id: s.id,
            profile_id: Some("p1".into()),
            scheduled_for: 10,
            started_at: 11,
            finished_at: None,
            outcome: ScheduleRunOutcome::Running,
            message: None,
            trigger: "manual".into(),
        };
        let rid = repo.start_run(&run).await.unwrap();
        assert_eq!(repo.fail_running(20, "restart").await.unwrap(), 1);
        let rows = repo.runs(Some(s.id), 10).await.unwrap();
        assert_eq!(rows[0].outcome, ScheduleRunOutcome::Failed);
        assert_eq!(rows[0].message.as_deref(), Some("restart"));
        repo.finish_run(rid, 30, ScheduleRunOutcome::Ok, None).await.unwrap();
        assert_eq!(repo.runs(None, 10).await.unwrap()[0].finished_at, Some(30));
        repo.delete(s.id).await.unwrap();
        assert!(repo.runs(None, 10).await.unwrap().is_empty(), "history goes with the schedule");
        assert!(repo.delete(s.id).await.is_err());
    }
}
