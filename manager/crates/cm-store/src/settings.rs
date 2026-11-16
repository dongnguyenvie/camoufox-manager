//! Key/value settings (`0001_core.sql`; phase 12 may extend via `0007_settings.sql`).

use crate::Db;
use anyhow::Result;
use async_trait::async_trait;
use cm_core::SettingsRepo;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

pub struct SqliteSettings {
    db: Db,
}

impl SqliteSettings {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

pub(crate) fn get(c: &Connection, key: &str) -> Result<Option<Value>> {
    let v: Option<String> = c.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).optional()?;
    Ok(v.and_then(|s| serde_json::from_str(&s).ok()))
}

pub(crate) fn set(c: &Connection, key: &str, value: &Value) -> Result<()> {
    c.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, serde_json::to_string(value)?],
    )?;
    Ok(())
}

#[async_trait]
impl SettingsRepo for SqliteSettings {
    async fn get(&self, key: &str) -> Result<Option<Value>> {
        self.db.with(|c| get(c, key))
    }

    async fn set(&self, key: &str, value: &Value) -> Result<()> {
        self.db.with(|c| set(c, key, value))
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.db.with(|c| {
            c.execute("DELETE FROM settings WHERE key = ?1", [key])?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn roundtrip() {
        let s = SqliteSettings::new(Db::memory().unwrap());
        assert_eq!(s.get("ui.language").await.unwrap(), None);
        s.set("ui.language", &json!("vi")).await.unwrap();
        s.set("ui.language", &json!("en")).await.unwrap();
        assert_eq!(s.get("ui.language").await.unwrap(), Some(json!("en")));
        s.delete("ui.language").await.unwrap();
        assert_eq!(s.get("ui.language").await.unwrap(), None);
    }
}
