//! Proxy library table (`0001_core.sql`; phase 2 extends via `0002_proxies.sql`).

use crate::Db;
use anyhow::Result;
use async_trait::async_trait;
use cm_core::{Clock, CmError, NewProxy, Proxy, ProxyCheck, ProxyId, ProxyRepo, ProxyScheme, SystemClock};
use rusqlite::{Connection, OptionalExtension, Row, params};

pub struct SqliteProxies {
    db: Db,
}

impl SqliteProxies {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

const COLS: &str = "id, name, scheme, host, port, username, password, created_at, last_check";

fn from_row(r: &Row) -> rusqlite::Result<Proxy> {
    let scheme: String = r.get(2)?;
    let check: Option<String> = r.get(8)?;
    Ok(Proxy {
        id: r.get(0)?,
        name: r.get(1)?,
        scheme: ProxyScheme::parse(&scheme).unwrap_or_default(),
        host: r.get(3)?,
        port: r.get(4)?,
        username: r.get(5)?,
        password: r.get(6)?,
        created_at: r.get(7)?,
        last_check: check.and_then(|s| serde_json::from_str(&s).ok()),
    })
}

/// Insert inside an existing connection/transaction (used by the legacy import too).
pub(crate) fn insert(c: &Connection, p: &NewProxy) -> Result<ProxyId> {
    c.execute(
        "INSERT INTO proxies (name, scheme, host, port, username, password, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![p.name, p.scheme.as_str(), p.host, p.port, p.username, p.password, SystemClock.now_iso()],
    )?;
    Ok(c.last_insert_rowid())
}

fn not_found(id: ProxyId) -> anyhow::Error {
    CmError::NotFound(format!("proxy #{id}")).into()
}

#[async_trait]
impl ProxyRepo for SqliteProxies {
    async fn list(&self) -> Result<Vec<Proxy>> {
        self.db.with(|c| {
            let mut st = c.prepare(&format!("SELECT {COLS} FROM proxies ORDER BY id"))?;
            let rows = st.query_map([], from_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
    }

    async fn get(&self, id: ProxyId) -> Result<Option<Proxy>> {
        self.db.with(|c| Ok(c.query_row(&format!("SELECT {COLS} FROM proxies WHERE id = ?1"), [id], from_row).optional()?))
    }

    async fn insert(&self, p: &NewProxy) -> Result<Proxy> {
        self.db.with(|c| {
            let id = insert(c, p)?;
            Ok(c.query_row(&format!("SELECT {COLS} FROM proxies WHERE id = ?1"), [id], from_row)?)
        })
    }

    async fn update(&self, id: ProxyId, p: &NewProxy) -> Result<()> {
        self.db.with(|c| {
            let n = c.execute(
                "UPDATE proxies SET name = ?2, scheme = ?3, host = ?4, port = ?5, username = ?6,
                 password = COALESCE(?7, password) WHERE id = ?1",
                params![id, p.name, p.scheme.as_str(), p.host, p.port, p.username, p.password],
            )?;
            if n == 0 { Err(not_found(id)) } else { Ok(()) }
        })
    }

    async fn delete(&self, id: ProxyId) -> Result<()> {
        self.db.with(|c| {
            // ON DELETE SET NULL unassigns it from profiles.
            let n = c.execute("DELETE FROM proxies WHERE id = ?1", [id])?;
            if n == 0 { Err(not_found(id)) } else { Ok(()) }
        })
    }

    async fn set_check(&self, id: ProxyId, check: &ProxyCheck) -> Result<()> {
        self.db.with(|c| {
            c.execute("UPDATE proxies SET last_check = ?2 WHERE id = ?1", params![id, serde_json::to_string(check)?])?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqliteProfiles;
    use cm_core::ProfileRepo;

    #[tokio::test]
    async fn crud_and_unassign_on_delete() {
        let db = Db::memory().unwrap();
        let repo = SqliteProxies::new(db.clone());
        let p = repo
            .insert(&NewProxy {
                name: "a".into(),
                host: "1.2.3.4".into(),
                port: 8080,
                username: Some("u".into()),
                password: Some("p".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(p.password.as_deref(), Some("p"));
        repo.update(p.id, &NewProxy { name: "b".into(), host: "1.2.3.4".into(), port: 81, ..Default::default() }).await.unwrap();
        let got = repo.get(p.id).await.unwrap().unwrap();
        assert_eq!((got.name.as_str(), got.port, got.password.as_deref()), ("b", 81, Some("p")));

        let profiles = SqliteProfiles::new(db);
        let mut prof = crate::profiles::tests_support::sample("bb22");
        prof.proxy_id = Some(p.id);
        profiles.insert(&prof).await.unwrap();
        repo.delete(p.id).await.unwrap();
        assert_eq!(profiles.get("bb22").await.unwrap().unwrap().proxy_id, None);
    }
}
