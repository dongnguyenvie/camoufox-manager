//! Profiles table (`0001_core.sql`; phase 4 extends via `0003_profiles.sql`).

use crate::Db;
use anyhow::Result;
use async_trait::async_trait;
use cm_core::{CmError, Os, Profile, ProfileRepo, ProxyId, TrashedProfile};
use rusqlite::{Connection, OptionalExtension, Row, params};

pub struct SqliteProfiles {
    db: Db,
}

impl SqliteProfiles {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

const COLS: &str = "id, name, os, notes, created_at, updated_at, last_activity, proxy_id, machine, settings";

fn from_row(r: &Row) -> rusqlite::Result<Profile> {
    let os: String = r.get(2)?;
    let machine: String = r.get(8)?;
    let settings: String = r.get(9)?;
    Ok(Profile {
        id: r.get(0)?,
        name: r.get(1)?,
        os: Os::parse(&os).unwrap_or(Os::Win),
        notes: r.get(3)?,
        created_at: r.get(4)?,
        updated_at: r.get(5)?,
        last_activity: r.get(6)?,
        proxy_id: r.get(7)?,
        machine: serde_json::from_str(&machine).unwrap_or_default(),
        settings: serde_json::from_str(&settings).unwrap_or_default(),
    })
}

/// Insert inside an existing connection/transaction (used by the legacy import too).
pub(crate) fn insert(c: &Connection, p: &Profile) -> Result<()> {
    c.execute(
        &format!("INSERT INTO profiles ({COLS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"),
        params![
            p.id,
            p.name,
            p.os.code(),
            p.notes,
            p.created_at,
            p.updated_at,
            p.last_activity,
            p.proxy_id,
            serde_json::to_string(&p.machine)?,
            serde_json::to_string(&p.settings)?,
        ],
    )?;
    Ok(())
}

fn not_found(id: &str) -> anyhow::Error {
    CmError::NotFound(format!("profile {id}")).into()
}

#[async_trait]
impl ProfileRepo for SqliteProfiles {
    async fn list(&self) -> Result<Vec<Profile>> {
        self.db.with(|c| {
            let mut st = c.prepare(&format!("SELECT {COLS} FROM profiles WHERE deleted_at IS NULL ORDER BY created_at, name"))?;
            let rows = st.query_map([], from_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
    }

    async fn get(&self, id: &str) -> Result<Option<Profile>> {
        self.db.with(|c| {
            Ok(c.query_row(&format!("SELECT {COLS} FROM profiles WHERE id = ?1 AND deleted_at IS NULL"), [id], from_row).optional()?)
        })
    }

    async fn insert(&self, p: &Profile) -> Result<()> {
        self.db.with(|c| insert(c, p))
    }

    async fn update(&self, p: &Profile) -> Result<()> {
        self.db.with(|c| {
            let n = c.execute(
                "UPDATE profiles SET name = ?2, os = ?3, notes = ?4, updated_at = ?5, last_activity = ?6, proxy_id = ?7,
                 machine = ?8, settings = ?9 WHERE id = ?1",
                params![
                    p.id,
                    p.name,
                    p.os.code(),
                    p.notes,
                    p.updated_at,
                    p.last_activity,
                    p.proxy_id,
                    serde_json::to_string(&p.machine)?,
                    serde_json::to_string(&p.settings)?,
                ],
            )?;
            if n == 0 { Err(not_found(&p.id)) } else { Ok(()) }
        })
    }

    async fn delete(&self, id: &str) -> Result<()> {
        self.db.with(|c| {
            let n = c.execute("DELETE FROM profiles WHERE id = ?1", [id])?;
            if n == 0 { Err(not_found(id)) } else { Ok(()) }
        })
    }

    async fn trash(&self, id: &str, at: &str) -> Result<()> {
        self.db.with(|c| {
            let n = c.execute("UPDATE profiles SET deleted_at = ?2 WHERE id = ?1 AND deleted_at IS NULL", params![id, at])?;
            if n == 0 { Err(not_found(id)) } else { Ok(()) }
        })
    }

    async fn list_trashed(&self) -> Result<Vec<TrashedProfile>> {
        self.db.with(|c| {
            let mut st =
                c.prepare(&format!("SELECT {COLS}, deleted_at FROM profiles WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC, id"))?;
            let rows = st
                .query_map([], |r| Ok(TrashedProfile { profile: from_row(r)?, deleted_at: r.get(10)? }))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
    }

    async fn restore(&self, id: &str) -> Result<()> {
        self.db.with(|c| {
            let n = c.execute("UPDATE profiles SET deleted_at = NULL WHERE id = ?1 AND deleted_at IS NOT NULL", [id])?;
            if n == 0 { Err(not_found(id)) } else { Ok(()) }
        })
    }

    async fn purge(&self, id: &str) -> Result<()> {
        self.db.with(|c| {
            let n = c.execute("DELETE FROM profiles WHERE id = ?1 AND deleted_at IS NOT NULL", [id])?;
            if n == 0 { Err(not_found(id)) } else { Ok(()) }
        })
    }

    async fn set_proxy(&self, id: &str, proxy: Option<ProxyId>) -> Result<()> {
        self.db.with(|c| {
            let n = c.execute("UPDATE profiles SET proxy_id = ?2 WHERE id = ?1", params![id, proxy])?;
            if n == 0 { Err(not_found(id)) } else { Ok(()) }
        })
    }

    async fn touch(&self, id: &str, at: &str) -> Result<()> {
        self.db.with(|c| {
            c.execute("UPDATE profiles SET last_activity = ?2 WHERE id = ?1", params![id, at])?;
            Ok(())
        })
    }
}

#[cfg(test)]
pub(crate) mod tests_support {
    use cm_core::{Os, Profile, ProfileSettings};

    pub(crate) fn sample(id: &str) -> Profile {
        Profile {
            id: id.into(),
            name: "Shop 01".into(),
            os: Os::Mac,
            notes: None,
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
            last_activity: None,
            proxy_id: None,
            machine: serde_json::json!({"navigator.userAgent": "UA"}),
            settings: ProfileSettings::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{tests_support::sample, *};

    #[tokio::test]
    async fn settings_roundtrip_and_old_rows_read_with_defaults() {
        use cm_core::{CanvasMode, WebRtcMode};
        let repo = SqliteProfiles::new(Db::memory().unwrap());
        let mut p = sample("bb22");
        p.settings.timezone = Some("Europe/Berlin".into());
        p.settings.webrtc = WebRtcMode::None;
        p.settings.fonts = vec!["Foo".into()];
        p.settings.screen = Some("1600x900".into());
        p.settings.canvas = CanvasMode::Random;
        repo.insert(&p).await.unwrap();
        assert_eq!(repo.get("bb22").await.unwrap().unwrap().settings, p.settings);
        // A row written before phase 4 (only old fields) reads with the new defaults.
        repo.db.with(|c| Ok(c.execute("UPDATE profiles SET settings = '{\"locale\":\"fr\"}' WHERE id = 'bb22'", [])?)).unwrap();
        let s = repo.get("bb22").await.unwrap().unwrap().settings;
        assert_eq!((s.locale.as_deref(), s.canvas, s.fonts.len()), (Some("fr"), CanvasMode::Stable, 0));
    }

    #[tokio::test]
    async fn trash_hides_restore_and_purge() {
        let repo = SqliteProfiles::new(Db::memory().unwrap());
        repo.insert(&sample("aa11")).await.unwrap();
        repo.insert(&sample("bb22")).await.unwrap();
        repo.trash("aa11", "2024-03-01T00:00:00Z").await.unwrap();
        assert!(repo.trash("aa11", "2024-03-02T00:00:00Z").await.is_err(), "already trashed");
        assert!(repo.get("aa11").await.unwrap().is_none());
        assert_eq!(repo.list().await.unwrap().len(), 1);
        let t = repo.list_trashed().await.unwrap();
        assert_eq!((t.len(), t[0].profile.id.as_str(), t[0].deleted_at.as_str()), (1, "aa11", "2024-03-01T00:00:00Z"));
        assert!(repo.purge("bb22").await.is_err(), "purge only touches trashed rows");
        repo.restore("aa11").await.unwrap();
        assert!(repo.restore("aa11").await.is_err());
        assert_eq!(repo.list().await.unwrap().len(), 2);
        repo.trash("aa11", "2024-03-01T00:00:00Z").await.unwrap();
        repo.purge("aa11").await.unwrap();
        assert!(repo.list_trashed().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn crud() {
        let repo = SqliteProfiles::new(Db::memory().unwrap());
        let p = sample("aa11");
        repo.insert(&p).await.unwrap();
        assert_eq!(repo.get("aa11").await.unwrap(), Some(p.clone()));
        repo.touch("aa11", "2024-02-02T00:00:00Z").await.unwrap();
        assert_eq!(repo.list().await.unwrap()[0].last_activity.as_deref(), Some("2024-02-02T00:00:00Z"));
        assert!(repo.set_proxy("aa11", Some(99)).await.is_err(), "foreign key enforced");
        repo.delete("aa11").await.unwrap();
        assert!(repo.get("aa11").await.unwrap().is_none());
        assert!(repo.delete("aa11").await.is_err());
    }
}
