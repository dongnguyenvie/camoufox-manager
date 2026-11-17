//! Organization store (tables in `0004_org.sql`): tags, groups, saved views.
//! Profile deletion cascades to assignments through foreign keys.

use crate::Db;
use anyhow::Result;
use async_trait::async_trait;
use cm_core::{CmError, Group, GroupRepo, ProfileId, SavedView, SavedViewRepo, Tag, TagRepo};
use rusqlite::{Connection, ErrorCode, params};
use serde_json::Value;

pub struct SqliteTags {
    db: Db,
}
pub struct SqliteGroups {
    db: Db,
}
pub struct SqliteSavedViews {
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

/// Map a UNIQUE violation to 409 and a missing foreign key to 404.
fn map_err(e: rusqlite::Error, what: &str) -> anyhow::Error {
    if let rusqlite::Error::SqliteFailure(f, _) = &e
        && f.code == ErrorCode::ConstraintViolation
    {
        return match f.extended_code {
            rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE => CmError::Conflict(format!("{what} name already exists")).into(),
            rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY => CmError::NotFound(format!("{what} or profile")).into(),
            _ => e.into(),
        };
    }
    e.into()
}

fn deleted(n: usize, what: &str, id: i64) -> Result<()> {
    if n == 0 { Err(CmError::NotFound(format!("{what} #{id}")).into()) } else { Ok(()) }
}

fn pairs(c: &Connection, sql: &str) -> Result<Vec<(ProfileId, i64)>> {
    let mut st = c.prepare(sql)?;
    Ok(st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)
}

#[async_trait]
impl TagRepo for SqliteTags {
    async fn list(&self) -> Result<Vec<Tag>> {
        self.db.with(|c| {
            let mut st = c.prepare("SELECT id, name, color FROM tags ORDER BY name")?;
            Ok(st.query_map([], |r| Ok(Tag { id: r.get(0)?, name: r.get(1)?, color: r.get(2)? }))?.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }
    async fn create(&self, name: &str, color: Option<&str>) -> Result<Tag> {
        self.db.with(|c| {
            c.execute("INSERT INTO tags (name, color) VALUES (?1, ?2)", params![name, color]).map_err(|e| map_err(e, "tag"))?;
            Ok(Tag { id: c.last_insert_rowid(), name: name.to_string(), color: color.map(String::from) })
        })
    }
    async fn delete(&self, id: i64) -> Result<()> {
        self.db.with(|c| deleted(c.execute("DELETE FROM tags WHERE id = ?1", [id])?, "tag", id))
    }
    async fn of_profile(&self, profile: &str) -> Result<Vec<i64>> {
        self.db.with(|c| {
            let mut st = c.prepare("SELECT tag_id FROM profile_tags WHERE profile_id = ?1 ORDER BY tag_id")?;
            Ok(st.query_map([profile], |r| r.get(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }
    async fn all_assignments(&self) -> Result<Vec<(ProfileId, i64)>> {
        self.db.with(|c| pairs(c, "SELECT profile_id, tag_id FROM profile_tags ORDER BY profile_id, tag_id"))
    }
    async fn add(&self, profiles: &[ProfileId], tag: i64) -> Result<()> {
        self.db.with(|c| {
            let tx = c.transaction()?;
            for p in profiles {
                tx.execute("INSERT OR IGNORE INTO profile_tags (profile_id, tag_id) VALUES (?1, ?2)", params![p, tag])
                    .map_err(|e| map_err(e, "tag"))?;
            }
            Ok(tx.commit()?)
        })
    }
    async fn remove(&self, profiles: &[ProfileId], tag: i64) -> Result<()> {
        self.db.with(|c| {
            let tx = c.transaction()?;
            for p in profiles {
                tx.execute("DELETE FROM profile_tags WHERE profile_id = ?1 AND tag_id = ?2", params![p, tag])?;
            }
            Ok(tx.commit()?)
        })
    }
}

#[async_trait]
impl GroupRepo for SqliteGroups {
    async fn list(&self) -> Result<Vec<Group>> {
        self.db.with(|c| {
            let mut st = c.prepare("SELECT id, name, description FROM groups ORDER BY name")?;
            Ok(st
                .query_map([], |r| Ok(Group { id: r.get(0)?, name: r.get(1)?, description: r.get(2)? }))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }
    async fn create(&self, name: &str, description: Option<&str>) -> Result<Group> {
        self.db.with(|c| {
            c.execute("INSERT INTO groups (name, description) VALUES (?1, ?2)", params![name, description])
                .map_err(|e| map_err(e, "group"))?;
            Ok(Group { id: c.last_insert_rowid(), name: name.to_string(), description: description.map(String::from) })
        })
    }
    async fn rename(&self, id: i64, name: &str) -> Result<()> {
        self.db.with(|c| {
            let n = c.execute("UPDATE groups SET name = ?2 WHERE id = ?1", params![id, name]).map_err(|e| map_err(e, "group"))?;
            deleted(n, "group", id)
        })
    }
    async fn delete(&self, id: i64) -> Result<()> {
        self.db.with(|c| deleted(c.execute("DELETE FROM groups WHERE id = ?1", [id])?, "group", id))
    }
    async fn all_assignments(&self) -> Result<Vec<(ProfileId, i64)>> {
        self.db.with(|c| pairs(c, "SELECT profile_id, group_id FROM profile_groups ORDER BY profile_id"))
    }
    async fn set(&self, profiles: &[ProfileId], group: Option<i64>) -> Result<()> {
        self.db.with(|c| {
            let tx = c.transaction()?;
            for p in profiles {
                match group {
                    Some(g) => tx
                        .execute(
                            "INSERT INTO profile_groups (profile_id, group_id) VALUES (?1, ?2)
                             ON CONFLICT(profile_id) DO UPDATE SET group_id = excluded.group_id",
                            params![p, g],
                        )
                        .map_err(|e| map_err(e, "group"))?,
                    None => tx.execute("DELETE FROM profile_groups WHERE profile_id = ?1", [p])?,
                };
            }
            Ok(tx.commit()?)
        })
    }
}

fn view_row(r: &rusqlite::Row) -> rusqlite::Result<SavedView> {
    let q: String = r.get(2)?;
    Ok(SavedView { id: r.get(0)?, name: r.get(1)?, query: serde_json::from_str(&q).unwrap_or(Value::Null) })
}

#[async_trait]
impl SavedViewRepo for SqliteSavedViews {
    async fn list(&self) -> Result<Vec<SavedView>> {
        self.db.with(|c| {
            let mut st = c.prepare("SELECT id, name, query FROM saved_views ORDER BY name")?;
            Ok(st.query_map([], view_row)?.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }
    /// Saving under an existing name replaces that view's query.
    async fn save(&self, name: &str, query: &Value) -> Result<SavedView> {
        self.db.with(|c| {
            let q = serde_json::to_string(query)?;
            c.execute(
                "INSERT INTO saved_views (name, query) VALUES (?1, ?2) ON CONFLICT(name) DO UPDATE SET query = excluded.query",
                params![name, q],
            )?;
            Ok(c.query_row("SELECT id, name, query FROM saved_views WHERE name = ?1", [name], view_row)?)
        })
    }
    async fn delete(&self, id: i64) -> Result<()> {
        self.db.with(|c| deleted(c.execute("DELETE FROM saved_views WHERE id = ?1", [id])?, "view", id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqliteProfiles;
    use cm_core::ProfileRepo;
    use serde_json::json;

    #[tokio::test]
    async fn tags_groups_views_and_cascade() {
        let db = Db::memory().unwrap();
        let profiles = SqliteProfiles::new(db.clone());
        profiles.insert(&crate::profiles::tests_support::sample("aa11")).await.unwrap();
        profiles.insert(&crate::profiles::tests_support::sample("bb22")).await.unwrap();
        let (tags, groups, views) = (SqliteTags::new(db.clone()), SqliteGroups::new(db.clone()), SqliteSavedViews::new(db));

        let t1 = tags.create("shop", Some("#ff0000")).await.unwrap();
        let t2 = tags.create("vip", None).await.unwrap();
        assert!(tags.create("SHOP", None).await.is_err(), "names are unique, case-insensitive");
        let both = vec!["aa11".to_string(), "bb22".to_string()];
        tags.add(&both, t1.id).await.unwrap();
        tags.add(&both, t1.id).await.unwrap(); // idempotent
        tags.add(&["aa11".to_string()], t2.id).await.unwrap();
        assert_eq!(tags.of_profile("aa11").await.unwrap(), vec![t1.id, t2.id]);
        tags.remove(&["aa11".to_string()], t1.id).await.unwrap();
        assert_eq!(tags.all_assignments().await.unwrap().len(), 2);

        let g1 = groups.create("A", None).await.unwrap();
        let g2 = groups.create("B", Some("d")).await.unwrap();
        groups.set(&both, Some(g1.id)).await.unwrap();
        groups.set(&["aa11".to_string()], Some(g2.id)).await.unwrap();
        assert_eq!(groups.all_assignments().await.unwrap(), vec![("aa11".into(), g2.id), ("bb22".into(), g1.id)]);
        groups.rename(g2.id, "C").await.unwrap();
        assert!(groups.set(&["aa11".to_string()], Some(999)).await.is_err());
        groups.set(&["aa11".to_string()], None).await.unwrap();
        groups.delete(g1.id).await.unwrap();
        assert!(groups.all_assignments().await.unwrap().is_empty(), "deleting a group ungroups its profiles");

        let v = views.save("v", &json!({ "q": "x" })).await.unwrap();
        let v2 = views.save("V", &json!({ "q": "y" })).await.unwrap();
        assert_eq!(v.id, v2.id);
        assert_eq!(views.list().await.unwrap()[0].query, json!({ "q": "y" }));
        views.delete(v.id).await.unwrap();
        assert!(views.delete(v.id).await.is_err());

        profiles.delete("bb22").await.unwrap();
        assert_eq!(tags.all_assignments().await.unwrap(), vec![("aa11".into(), t2.id)]);
        tags.delete(t2.id).await.unwrap();
        assert!(tags.all_assignments().await.unwrap().is_empty());
    }
}
