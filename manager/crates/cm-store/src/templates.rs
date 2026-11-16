//! Templates + extension library store — phase 9 fills this (tables in `0005_templates.sql`).
//! Stub: every call returns `CmError::NotImplemented("templates")`.

use crate::Db;
use anyhow::Result;
use async_trait::async_trait;
use cm_core::{Extension, ExtensionRepo, Template, TemplateRepo, not_implemented};

const AREA: &str = "templates";

pub struct SqliteTemplates {
    #[allow(dead_code)]
    db: Db,
}
pub struct SqliteExtensions {
    #[allow(dead_code)]
    db: Db,
}

impl SqliteTemplates {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}
impl SqliteExtensions {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TemplateRepo for SqliteTemplates {
    async fn list(&self) -> Result<Vec<Template>> {
        Err(not_implemented(AREA))
    }
    async fn get(&self, _id: i64) -> Result<Option<Template>> {
        Err(not_implemented(AREA))
    }
    async fn save(&self, _t: &Template) -> Result<Template> {
        Err(not_implemented(AREA))
    }
    async fn delete(&self, _id: i64) -> Result<()> {
        Err(not_implemented(AREA))
    }
}

#[async_trait]
impl ExtensionRepo for SqliteExtensions {
    async fn list(&self) -> Result<Vec<Extension>> {
        Err(not_implemented(AREA))
    }
    async fn insert(&self, _e: &Extension) -> Result<Extension> {
        Err(not_implemented(AREA))
    }
    async fn delete(&self, _id: i64) -> Result<()> {
        Err(not_implemented(AREA))
    }
    async fn of_profile(&self, _profile: &str) -> Result<Vec<i64>> {
        Err(not_implemented(AREA))
    }
    async fn set_for_profile(&self, _profile: &str, _ids: &[i64]) -> Result<()> {
        Err(not_implemented(AREA))
    }
}
