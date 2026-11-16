//! Schedule store — phase 10 fills this (tables in `0006_schedules.sql`).
//! Stub: every call returns `CmError::NotImplemented("schedules")`.

use crate::Db;
use anyhow::Result;
use async_trait::async_trait;
use cm_core::{Schedule, ScheduleRepo, ScheduleRun, not_implemented};

const AREA: &str = "schedules";

pub struct SqliteSchedules {
    #[allow(dead_code)]
    db: Db,
}

impl SqliteSchedules {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ScheduleRepo for SqliteSchedules {
    async fn list(&self) -> Result<Vec<Schedule>> {
        Err(not_implemented(AREA))
    }
    async fn save(&self, _s: &Schedule) -> Result<Schedule> {
        Err(not_implemented(AREA))
    }
    async fn delete(&self, _id: i64) -> Result<()> {
        Err(not_implemented(AREA))
    }
    async fn log_run(&self, _run: &ScheduleRun) -> Result<()> {
        Err(not_implemented(AREA))
    }
    async fn runs(&self, _schedule: i64, _limit: usize) -> Result<Vec<ScheduleRun>> {
        Err(not_implemented(AREA))
    }
}
