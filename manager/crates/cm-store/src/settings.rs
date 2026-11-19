//! Key/value settings (`0001_core.sql`) and named API keys (`0007_settings.sql`).

use crate::Db;
use anyhow::Result;
use async_trait::async_trait;
use cm_core::{ApiKeyInfo, ApiKeyRecord, ApiKeyRepo, SettingsRepo};
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

/// Named API keys (phase 12).
pub struct SqliteApiKeys {
    db: Db,
}

impl SqliteApiKeys {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

fn key_info(r: &rusqlite::Row) -> rusqlite::Result<ApiKeyInfo> {
    Ok(ApiKeyInfo { id: r.get(0)?, name: r.get(1)?, created_at: r.get(2)?, last_used_at: r.get(3)?, revoked_at: r.get(4)? })
}

#[async_trait]
impl ApiKeyRepo for SqliteApiKeys {
    async fn insert(&self, rec: &ApiKeyRecord) -> Result<()> {
        self.db.with(|c| {
            c.execute(
                "INSERT INTO api_keys (id, name, salt, hash, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![rec.info.id, rec.info.name, rec.salt, rec.hash, rec.info.created_at],
            )?;
            Ok(())
        })
    }

    async fn list(&self) -> Result<Vec<ApiKeyInfo>> {
        self.db.with(|c| {
            let mut st =
                c.prepare("SELECT id, name, created_at, last_used_at, revoked_at FROM api_keys ORDER BY created_at DESC, rowid DESC")?;
            let rows = st.query_map([], key_info)?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
    }

    async fn get(&self, id: &str) -> Result<Option<ApiKeyRecord>> {
        self.db.with(|c| {
            Ok(c.query_row("SELECT id, name, created_at, last_used_at, revoked_at, salt, hash FROM api_keys WHERE id = ?1", [id], |r| {
                Ok(ApiKeyRecord { info: key_info(r)?, salt: r.get(5)?, hash: r.get(6)? })
            })
            .optional()?)
        })
    }

    async fn touch(&self, id: &str, at: &str) -> Result<()> {
        self.db.with(|c| {
            c.execute("UPDATE api_keys SET last_used_at = ?2 WHERE id = ?1", params![id, at])?;
            Ok(())
        })
    }

    async fn revoke(&self, id: &str, at: &str) -> Result<bool> {
        self.db.with(|c| Ok(c.execute("UPDATE api_keys SET revoked_at = ?2 WHERE id = ?1 AND revoked_at IS NULL", params![id, at])? > 0))
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

    #[tokio::test]
    async fn api_keys_lifecycle() {
        let k = SqliteApiKeys::new(Db::memory().unwrap());
        let rec = ApiKeyRecord {
            info: ApiKeyInfo {
                id: "ab12".into(),
                name: "agent".into(),
                created_at: "2026-01-01T00:00:00Z".into(),
                last_used_at: None,
                revoked_at: None,
            },
            salt: "s".into(),
            hash: "h".into(),
        };
        k.insert(&rec).await.unwrap();
        assert_eq!(k.get("ab12").await.unwrap(), Some(rec));
        k.touch("ab12", "2026-01-02T00:00:00Z").await.unwrap();
        assert_eq!(k.list().await.unwrap()[0].last_used_at.as_deref(), Some("2026-01-02T00:00:00Z"));
        assert!(k.revoke("ab12", "2026-01-03T00:00:00Z").await.unwrap());
        assert!(!k.revoke("ab12", "2026-01-04T00:00:00Z").await.unwrap());
        assert!(k.get("nope").await.unwrap().is_none());
    }
}
