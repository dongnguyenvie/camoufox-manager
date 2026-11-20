//! In-app notifications (`0009_notifications.sql`, phase 19).

use crate::Db;
use anyhow::Result;
use async_trait::async_trait;
use cm_core::{NewNotification, Notification, NotificationRepo, notification::KEEP};
use rusqlite::params;

pub struct SqliteNotifications {
    db: Db,
}

impl SqliteNotifications {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl NotificationRepo for SqliteNotifications {
    async fn add(&self, n: &NewNotification, at: u64) -> Result<Option<i64>> {
        self.db.with(|c| {
            let changed = c.execute(
                "INSERT OR IGNORE INTO notifications (kind, level, title, body, params, dedupe, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![n.kind, n.level, n.title, n.body, serde_json::to_string(&n.params)?, n.dedupe, at as i64],
            )?;
            if changed == 0 {
                return Ok(None);
            }
            let id = c.last_insert_rowid();
            c.execute(
                "DELETE FROM notifications WHERE id NOT IN (SELECT id FROM notifications ORDER BY created_at DESC, id DESC LIMIT ?1)",
                [KEEP as i64],
            )?;
            Ok(Some(id))
        })
    }

    async fn list(&self, limit: usize) -> Result<Vec<Notification>> {
        self.db.with(|c| {
            let mut st = c.prepare(
                "SELECT id, kind, level, title, body, params, created_at, read FROM notifications
                 WHERE cleared = 0 ORDER BY created_at DESC, id DESC LIMIT ?1",
            )?;
            let rows = st.query_map([limit as i64], |r| {
                let params: String = r.get(5)?;
                Ok(Notification {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    level: r.get(2)?,
                    title: r.get(3)?,
                    body: r.get(4)?,
                    params: serde_json::from_str(&params).unwrap_or_default(),
                    created_at: r.get::<_, i64>(6)? as u64,
                    read: r.get::<_, i64>(7)? != 0,
                })
            })?;
            Ok(rows.collect::<std::result::Result<_, _>>()?)
        })
    }

    async fn unread(&self) -> Result<u64> {
        self.db.with(|c| {
            Ok(c.query_row("SELECT COUNT(*) FROM notifications WHERE read = 0 AND cleared = 0", [], |r| r.get::<_, i64>(0))? as u64)
        })
    }

    async fn mark_read(&self, ids: &[i64]) -> Result<()> {
        self.db.with(|c| {
            for id in ids {
                c.execute("UPDATE notifications SET read = 1 WHERE id = ?1", [id])?;
            }
            Ok(())
        })
    }

    async fn mark_all_read(&self) -> Result<()> {
        self.db.with(|c| {
            c.execute("UPDATE notifications SET read = 1", [])?;
            Ok(())
        })
    }

    async fn clear(&self) -> Result<()> {
        self.db.with(|c| {
            c.execute("UPDATE notifications SET cleared = 1", [])?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn n(kind: &str, dedupe: Option<&str>) -> NewNotification {
        NewNotification {
            kind: kind.into(),
            level: "error".into(),
            title: format!("t {kind}"),
            body: "b".into(),
            params: json!({ "profile": "x" }),
            dedupe: dedupe.map(Into::into),
        }
    }

    #[tokio::test]
    async fn add_list_read_clear_and_dedupe() {
        let s = SqliteNotifications::new(Db::memory().unwrap());
        let a = s.add(&n("profile.crashed", Some("crash:a:1")), 10).await.unwrap().unwrap();
        assert!(s.add(&n("profile.crashed", Some("crash:a:1")), 11).await.unwrap().is_none(), "deduped");
        let b = s.add(&n("proxy.check_failed", None), 20).await.unwrap().unwrap();
        s.add(&n("proxy.check_failed", None), 30).await.unwrap().unwrap();
        assert_eq!(s.unread().await.unwrap(), 3);
        let l = s.list(10).await.unwrap();
        assert_eq!(l.iter().map(|x| x.created_at).collect::<Vec<_>>(), [30, 20, 10]);
        assert_eq!(l[2].params["profile"], "x");
        s.mark_read(&[a, b]).await.unwrap();
        assert_eq!(s.unread().await.unwrap(), 1);
        s.mark_all_read().await.unwrap();
        assert_eq!(s.unread().await.unwrap(), 0);
        s.clear().await.unwrap();
        assert!(s.list(10).await.unwrap().is_empty());
        assert!(s.add(&n("profile.crashed", Some("crash:a:1")), 40).await.unwrap().is_none(), "cleared rows still dedupe");
    }

    #[tokio::test]
    async fn keeps_only_the_newest_rows() {
        let s = SqliteNotifications::new(Db::memory().unwrap());
        for i in 0..(KEEP as u64 + 15) {
            s.add(&n("x", None), i).await.unwrap();
        }
        let l = s.list(1000).await.unwrap();
        assert_eq!(l.len(), KEEP);
        assert_eq!(l.last().unwrap().created_at, 15);
    }
}
