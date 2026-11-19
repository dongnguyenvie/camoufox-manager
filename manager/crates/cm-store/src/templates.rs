//! Templates + extension library store (tables in `0005_templates.sql`).
//! Deleting an extension, tag, group or proxy cascades / nulls the references through foreign keys.

use crate::Db;
use anyhow::Result;
use async_trait::async_trait;
use cm_core::{CmError, Extension, ExtensionRepo, Os, Template, TemplateRepo};
use rusqlite::{Connection, ErrorCode, OptionalExtension, Row, params};

pub struct SqliteTemplates {
    db: Db,
}
pub struct SqliteExtensions {
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

/// UNIQUE violation -> 409, missing foreign key -> 404.
fn map_err(e: rusqlite::Error, what: &str) -> anyhow::Error {
    if let rusqlite::Error::SqliteFailure(f, _) = &e
        && f.code == ErrorCode::ConstraintViolation
    {
        return match f.extended_code {
            rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE => CmError::Conflict(format!("{what} already exists")).into(),
            rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY => {
                CmError::NotFound(format!("{what}: a referenced proxy, tag, group, extension or profile")).into()
            }
            _ => e.into(),
        };
    }
    e.into()
}

fn ids(c: &Connection, sql: &str, key: impl rusqlite::ToSql) -> Result<Vec<i64>> {
    let mut st = c.prepare(sql)?;
    Ok(st.query_map([key], |r| r.get(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn template_of(c: &Connection, r: &Row) -> Result<Template> {
    let id: i64 = r.get(0)?;
    let os: String = r.get(2)?;
    let settings: String = r.get(3)?;
    Ok(Template {
        id,
        name: r.get(1)?,
        os: Os::parse(&os).unwrap_or(Os::Win),
        settings: serde_json::from_str(&settings).unwrap_or_default(),
        proxy_id: r.get(4)?,
        group_id: r.get(5)?,
        extensions: ids(c, "SELECT extension_id FROM template_extensions WHERE template_id = ?1 ORDER BY extension_id", id)?,
        tags: ids(c, "SELECT tag_id FROM template_tags WHERE template_id = ?1 ORDER BY tag_id", id)?,
    })
}

const COLS: &str = "id, name, os, settings, proxy_id, group_id";

fn get(c: &Connection, id: i64) -> Result<Option<Template>> {
    let mut st = c.prepare(&format!("SELECT {COLS} FROM templates WHERE id = ?1"))?;
    let mut rows = st.query([id])?;
    match rows.next()? {
        Some(r) => Ok(Some(template_of(c, r)?)),
        None => Ok(None),
    }
}

#[async_trait]
impl TemplateRepo for SqliteTemplates {
    async fn list(&self) -> Result<Vec<Template>> {
        self.db.with(|c| {
            let mut st = c.prepare(&format!("SELECT {COLS} FROM templates ORDER BY name COLLATE NOCASE"))?;
            let mut rows = st.query([])?;
            let mut out = Vec::new();
            while let Some(r) = rows.next()? {
                out.push(template_of(c, r)?);
            }
            Ok(out)
        })
    }

    async fn get(&self, id: i64) -> Result<Option<Template>> {
        self.db.with(|c| get(c, id))
    }

    async fn save(&self, t: &Template) -> Result<Template> {
        self.db.with(|c| {
            let tx = c.transaction()?;
            let settings = serde_json::to_string(&t.settings)?;
            let id = if t.id == 0 {
                tx.execute(
                    "INSERT INTO templates (name, os, settings, proxy_id, group_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![t.name, t.os.code(), settings, t.proxy_id, t.group_id],
                )
                .map_err(|e| map_err(e, "template name"))?;
                tx.last_insert_rowid()
            } else {
                let n = tx
                    .execute(
                        "UPDATE templates SET name = ?2, os = ?3, settings = ?4, proxy_id = ?5, group_id = ?6 WHERE id = ?1",
                        params![t.id, t.name, t.os.code(), settings, t.proxy_id, t.group_id],
                    )
                    .map_err(|e| map_err(e, "template name"))?;
                if n == 0 {
                    return Err(CmError::NotFound(format!("template #{}", t.id)).into());
                }
                t.id
            };
            tx.execute("DELETE FROM template_extensions WHERE template_id = ?1", [id])?;
            for e in &t.extensions {
                tx.execute("INSERT OR IGNORE INTO template_extensions (template_id, extension_id) VALUES (?1, ?2)", params![id, e])
                    .map_err(|e| map_err(e, "template extension"))?;
            }
            tx.execute("DELETE FROM template_tags WHERE template_id = ?1", [id])?;
            for tag in &t.tags {
                tx.execute("INSERT OR IGNORE INTO template_tags (template_id, tag_id) VALUES (?1, ?2)", params![id, tag])
                    .map_err(|e| map_err(e, "template tag"))?;
            }
            tx.commit()?;
            get(c, id)?.ok_or_else(|| CmError::NotFound(format!("template #{id}")).into())
        })
    }

    async fn delete(&self, id: i64) -> Result<()> {
        self.db.with(|c| match c.execute("DELETE FROM templates WHERE id = ?1", [id])? {
            0 => Err(CmError::NotFound(format!("template #{id}")).into()),
            _ => Ok(()),
        })
    }
}

fn extension_of(r: &Row) -> rusqlite::Result<Extension> {
    Ok(Extension { id: r.get(0)?, name: r.get(1)?, addon_id: r.get(2)?, version: r.get(3)?, file: r.get(4)? })
}

#[async_trait]
impl ExtensionRepo for SqliteExtensions {
    async fn list(&self) -> Result<Vec<Extension>> {
        self.db.with(|c| {
            let mut st = c.prepare("SELECT id, name, addon_id, version, file FROM extensions ORDER BY name COLLATE NOCASE, id")?;
            Ok(st.query_map([], extension_of)?.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    async fn insert(&self, e: &Extension) -> Result<Extension> {
        self.db.with(|c| {
            c.execute(
                "INSERT INTO extensions (name, addon_id, version, file) VALUES (?1, ?2, ?3, ?4)",
                params![e.name, e.addon_id, e.version, e.file],
            )
            .map_err(|er| map_err(er, "extension with this add-on id"))?;
            let id = c.last_insert_rowid();
            let stored = c
                .query_row("SELECT id, name, addon_id, version, file FROM extensions WHERE id = ?1", [id], extension_of)
                .optional()?
                .ok_or_else(|| CmError::NotFound(format!("extension #{id}")))?;
            Ok(stored)
        })
    }

    async fn delete(&self, id: i64) -> Result<()> {
        self.db.with(|c| match c.execute("DELETE FROM extensions WHERE id = ?1", [id])? {
            0 => Err(CmError::NotFound(format!("extension #{id}")).into()),
            _ => Ok(()),
        })
    }

    async fn of_profile(&self, profile: &str) -> Result<Vec<i64>> {
        self.db.with(|c| ids(c, "SELECT extension_id FROM profile_extensions WHERE profile_id = ?1 ORDER BY extension_id", profile))
    }

    async fn set_for_profile(&self, profile: &str, list: &[i64]) -> Result<()> {
        self.db.with(|c| {
            let tx = c.transaction()?;
            tx.execute("DELETE FROM profile_extensions WHERE profile_id = ?1", [profile])?;
            for e in list {
                tx.execute("INSERT OR IGNORE INTO profile_extensions (profile_id, extension_id) VALUES (?1, ?2)", params![profile, e])
                    .map_err(|er| map_err(er, "profile extension"))?;
            }
            tx.commit()?;
            Ok(())
        })
    }

    async fn all_assignments(&self) -> Result<Vec<(String, i64)>> {
        self.db.with(|c| {
            let mut st = c.prepare("SELECT profile_id, extension_id FROM profile_extensions ORDER BY profile_id, extension_id")?;
            Ok(st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cm_core::{Profile, ProfileRepo, TagRepo};

    fn ext(addon: &str) -> Extension {
        Extension { id: 0, name: addon.into(), addon_id: addon.into(), version: Some("1".into()), file: format!("{addon}.xpi") }
    }

    #[tokio::test]
    async fn template_roundtrip_with_assignments() {
        let db = Db::memory().unwrap();
        let (ts, es, tags) = (SqliteTemplates::new(db.clone()), SqliteExtensions::new(db.clone()), crate::SqliteTags::new(db.clone()));
        let e1 = es.insert(&ext("a@x")).await.unwrap();
        let tag = tags.create("work", None).await.unwrap();
        let mut t = Template { name: "Base".into(), os: Os::Mac, extensions: vec![e1.id], tags: vec![tag.id], ..Default::default() };
        t.settings.start_url = Some("https://example.com".into());
        let saved = ts.save(&t).await.unwrap();
        assert!(saved.id > 0);
        assert_eq!(saved.extensions, vec![e1.id]);
        assert_eq!(saved.tags, vec![tag.id]);
        assert_eq!(ts.get(saved.id).await.unwrap().unwrap().settings.start_url.as_deref(), Some("https://example.com"));
        // duplicate name -> conflict (case-insensitive)
        let dup = Template { name: "base".into(), ..Default::default() };
        assert!(ts.save(&dup).await.unwrap_err().downcast_ref::<CmError>().is_some_and(|e| matches!(e, CmError::Conflict(_))));
        // deleting the extension drops it from the template
        es.delete(e1.id).await.unwrap();
        assert!(ts.get(saved.id).await.unwrap().unwrap().extensions.is_empty());
        ts.delete(saved.id).await.unwrap();
        assert!(ts.delete(saved.id).await.is_err());
    }

    #[tokio::test]
    async fn profile_assignments_cascade() {
        let db = Db::memory().unwrap();
        let es = SqliteExtensions::new(db.clone());
        let e1 = es.insert(&ext("a@x")).await.unwrap();
        assert!(es.insert(&ext("a@x")).await.is_err());
        let p = Profile {
            id: "ab12".into(),
            name: "p".into(),
            os: Os::Win,
            notes: None,
            created_at: "t".into(),
            updated_at: "t".into(),
            last_activity: None,
            proxy_id: None,
            machine: serde_json::json!({}),
            settings: Default::default(),
        };
        crate::SqliteProfiles::new(db.clone()).insert(&p).await.unwrap();
        es.set_for_profile("ab12", &[e1.id]).await.unwrap();
        assert_eq!(es.of_profile("ab12").await.unwrap(), vec![e1.id]);
        assert_eq!(es.all_assignments().await.unwrap().len(), 1);
        crate::SqliteProfiles::new(db).delete("ab12").await.unwrap();
        assert!(es.of_profile("ab12").await.unwrap().is_empty());
    }
}
