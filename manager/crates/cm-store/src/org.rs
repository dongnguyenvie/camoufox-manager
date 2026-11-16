//! Organization store — phase 7 fills this (tables in `0004_org.sql`). Stub: every call
//! returns `CmError::NotImplemented("org")`.

use crate::Db;
use anyhow::Result;
use async_trait::async_trait;
use cm_core::{Group, GroupRepo, ProfileId, SavedView, SavedViewRepo, Tag, TagRepo, not_implemented};
use serde_json::Value;

const AREA: &str = "org";

pub struct SqliteTags {
    #[allow(dead_code)]
    db: Db,
}
pub struct SqliteGroups {
    #[allow(dead_code)]
    db: Db,
}
pub struct SqliteSavedViews {
    #[allow(dead_code)]
    db: Db,
}

impl SqliteTags {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}
impl SqliteGroups {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}
impl SqliteSavedViews {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TagRepo for SqliteTags {
    async fn list(&self) -> Result<Vec<Tag>> {
        Err(not_implemented(AREA))
    }
    async fn create(&self, _name: &str, _color: Option<&str>) -> Result<Tag> {
        Err(not_implemented(AREA))
    }
    async fn delete(&self, _id: i64) -> Result<()> {
        Err(not_implemented(AREA))
    }
    async fn of_profile(&self, _profile: &str) -> Result<Vec<i64>> {
        Err(not_implemented(AREA))
    }
    async fn all_assignments(&self) -> Result<Vec<(ProfileId, i64)>> {
        Err(not_implemented(AREA))
    }
    async fn add(&self, _profiles: &[ProfileId], _tag: i64) -> Result<()> {
        Err(not_implemented(AREA))
    }
    async fn remove(&self, _profiles: &[ProfileId], _tag: i64) -> Result<()> {
        Err(not_implemented(AREA))
    }
}

#[async_trait]
impl GroupRepo for SqliteGroups {
    async fn list(&self) -> Result<Vec<Group>> {
        Err(not_implemented(AREA))
    }
    async fn create(&self, _name: &str, _description: Option<&str>) -> Result<Group> {
        Err(not_implemented(AREA))
    }
    async fn rename(&self, _id: i64, _name: &str) -> Result<()> {
        Err(not_implemented(AREA))
    }
    async fn delete(&self, _id: i64) -> Result<()> {
        Err(not_implemented(AREA))
    }
    async fn all_assignments(&self) -> Result<Vec<(ProfileId, i64)>> {
        Err(not_implemented(AREA))
    }
    async fn set(&self, _profiles: &[ProfileId], _group: Option<i64>) -> Result<()> {
        Err(not_implemented(AREA))
    }
}

#[async_trait]
impl SavedViewRepo for SqliteSavedViews {
    async fn list(&self) -> Result<Vec<SavedView>> {
        Err(not_implemented(AREA))
    }
    async fn save(&self, _name: &str, _query: &Value) -> Result<SavedView> {
        Err(not_implemented(AREA))
    }
    async fn delete(&self, _id: i64) -> Result<()> {
        Err(not_implemented(AREA))
    }
}
