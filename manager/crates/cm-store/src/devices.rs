//! Paired devices (`0010_devices.sql`, phase 16).

use crate::Db;
use anyhow::Result;
use async_trait::async_trait;
use cm_core::{Device, DeviceRepo};
use rusqlite::{OptionalExtension, params};

pub struct SqliteDevices {
    db: Db,
}

impl SqliteDevices {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

const COLS: &str = "id, name, public_key, url, tailscale_login, paired_at, last_seen_at, last_seen_addr, revoked_at";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Device> {
    Ok(Device {
        id: r.get(0)?,
        name: r.get(1)?,
        public_key: r.get(2)?,
        url: r.get(3)?,
        tailscale_login: r.get(4)?,
        paired_at: r.get(5)?,
        last_seen_at: r.get(6)?,
        last_seen_addr: r.get(7)?,
        revoked_at: r.get(8)?,
    })
}

#[async_trait]
impl DeviceRepo for SqliteDevices {
    async fn upsert(&self, d: &Device) -> Result<()> {
        self.db.with(|c| {
            c.execute(
                &format!(
                    "INSERT INTO devices ({COLS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                     ON CONFLICT(id) DO UPDATE SET name = excluded.name, public_key = excluded.public_key, url = excluded.url,
                       tailscale_login = excluded.tailscale_login, paired_at = excluded.paired_at,
                       last_seen_at = excluded.last_seen_at, last_seen_addr = excluded.last_seen_addr, revoked_at = excluded.revoked_at"
                ),
                params![d.id, d.name, d.public_key, d.url, d.tailscale_login, d.paired_at, d.last_seen_at, d.last_seen_addr, d.revoked_at],
            )?;
            Ok(())
        })
    }

    async fn get(&self, id: &str) -> Result<Option<Device>> {
        self.db.with(|c| Ok(c.query_row(&format!("SELECT {COLS} FROM devices WHERE id = ?1"), [id], row).optional()?))
    }

    async fn list(&self) -> Result<Vec<Device>> {
        self.db.with(|c| {
            let mut st = c.prepare(&format!("SELECT {COLS} FROM devices ORDER BY paired_at DESC, rowid DESC"))?;
            Ok(st.query_map([], row)?.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    async fn touch(&self, id: &str, at: &str, addr: Option<&str>) -> Result<()> {
        self.db.with(|c| {
            c.execute(
                "UPDATE devices SET last_seen_at = ?2, last_seen_addr = COALESCE(?3, last_seen_addr) WHERE id = ?1",
                params![id, at, addr],
            )?;
            Ok(())
        })
    }

    async fn revoke(&self, id: &str, at: &str) -> Result<bool> {
        self.db.with(|c| Ok(c.execute("UPDATE devices SET revoked_at = ?2 WHERE id = ?1 AND revoked_at IS NULL", params![id, at])? > 0))
    }

    async fn delete(&self, id: &str) -> Result<bool> {
        self.db.with(|c| Ok(c.execute("DELETE FROM devices WHERE id = ?1", [id])? > 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(id: &str, at: &str) -> Device {
        Device {
            id: id.into(),
            name: "Other Mac".into(),
            public_key: format!("{id:0>64}"),
            url: Some("https://other.tail0000.ts.net:8443".into()),
            tailscale_login: None,
            paired_at: at.into(),
            last_seen_at: None,
            last_seen_addr: None,
            revoked_at: None,
        }
    }

    #[tokio::test]
    async fn lifecycle() {
        let r = SqliteDevices::new(Db::memory().unwrap());
        r.upsert(&dev("aa", "2026-01-01T00:00:00Z")).await.unwrap();
        r.upsert(&dev("bb", "2026-01-02T00:00:00Z")).await.unwrap();
        assert_eq!(r.list().await.unwrap().iter().map(|d| d.id.as_str()).collect::<Vec<_>>(), ["bb", "aa"]);
        r.touch("aa", "2026-01-03T00:00:00Z", Some("100.100.1.3")).await.unwrap();
        r.touch("aa", "2026-01-04T00:00:00Z", None).await.unwrap();
        let a = r.get("aa").await.unwrap().unwrap();
        assert_eq!(a.last_seen_at.as_deref(), Some("2026-01-04T00:00:00Z"));
        assert_eq!(a.last_seen_addr.as_deref(), Some("100.100.1.3"));
        assert!(r.revoke("aa", "2026-01-05T00:00:00Z").await.unwrap());
        assert!(!r.revoke("aa", "2026-01-05T00:00:00Z").await.unwrap());
        // Re-pairing replaces the row and clears the revocation.
        r.upsert(&dev("aa", "2026-01-06T00:00:00Z")).await.unwrap();
        assert!(r.get("aa").await.unwrap().unwrap().revoked_at.is_none());
        assert!(r.delete("bb").await.unwrap());
        assert!(r.get("bb").await.unwrap().is_none());
    }
}
