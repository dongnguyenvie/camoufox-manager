//! Organization (phase 7): tags (many per profile, with colour), groups (one per profile),
//! saved views (opaque filter + sort owned by the UI) and bulk actions on selected profiles.
//! Bulk actions return one result per profile so the UI can show what failed and why.

use crate::Manager;
use anyhow::Result;
use cm_core::{CmError, Group, ProxyId, SavedView, Tag};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Most profiles one bulk request may touch.
pub const BULK_MAX: usize = 500;
const NAME_MAX: usize = 60;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgOverview {
    pub tags: Vec<Tag>,
    pub groups: Vec<Group>,
    pub views: Vec<SavedView>,
    /// profile id -> tag ids
    pub profile_tags: HashMap<String, Vec<i64>>,
    /// profile id -> group id (profiles without a group are absent)
    pub profile_groups: HashMap<String, i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkRequest {
    /// `start` | `stop` | `delete` | `proxy` | `tag` | `untag` | `group`
    pub action: String,
    pub profile_ids: Vec<String>,
    /// `proxy`: proxy to assign (`null` clears it).
    #[serde(default)]
    pub proxy_id: Option<ProxyId>,
    /// `tag` / `untag`
    #[serde(default)]
    pub tag_id: Option<i64>,
    /// `group`: target group (`null` = remove from group).
    #[serde(default)]
    pub group_id: Option<i64>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BulkItem {
    pub profile_id: String,
    pub ok: bool,
    /// `not-found` | `running` | `not-running` | `error`, plus a message.
    pub reason: Option<&'static str>,
    pub error: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BulkResult {
    pub action: String,
    pub ok: usize,
    pub failed: usize,
    pub results: Vec<BulkItem>,
}

fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(CmError::Invalid(msg.into()).into())
}

fn clean_name(name: &str, what: &str) -> Result<String> {
    let n = name.trim();
    if n.is_empty() {
        return invalid(format!("{what} name must not be empty"));
    }
    if n.chars().count() > NAME_MAX {
        return invalid(format!("{what} name is longer than {NAME_MAX} characters"));
    }
    Ok(n.to_string())
}

/// `#rgb` or `#rrggbb`.
fn clean_color(color: Option<&str>) -> Result<Option<String>> {
    match color.map(str::trim).filter(|c| !c.is_empty()) {
        None => Ok(None),
        Some(c) if c.starts_with('#') && matches!(c.len(), 4 | 7) && c[1..].chars().all(|x| x.is_ascii_hexdigit()) => {
            Ok(Some(c.to_ascii_lowercase()))
        }
        Some(_) => invalid("colour must look like #rrggbb"),
    }
}

fn item(profile_id: &str, r: Result<()>) -> BulkItem {
    match r {
        Ok(()) => BulkItem { profile_id: profile_id.into(), ok: true, reason: None, error: None },
        Err(e) => {
            let reason = match e.downcast_ref::<CmError>() {
                Some(CmError::NotFound(_)) => "not-found",
                Some(CmError::Conflict(_)) => "running",
                _ => "error",
            };
            BulkItem { profile_id: profile_id.into(), ok: false, reason: Some(reason), error: Some(format!("{e:#}")) }
        }
    }
}

impl Manager {
    pub async fn org_overview(&self) -> Result<OrgOverview> {
        let mut profile_tags: HashMap<String, Vec<i64>> = HashMap::new();
        for (p, t) in self.d.tags.all_assignments().await? {
            profile_tags.entry(p).or_default().push(t);
        }
        Ok(OrgOverview {
            tags: self.d.tags.list().await?,
            groups: self.d.groups.list().await?,
            views: self.d.views.list().await?,
            profile_tags,
            profile_groups: self.d.groups.all_assignments().await?.into_iter().collect(),
        })
    }

    pub async fn create_tag(&self, name: &str, color: Option<&str>) -> Result<Tag> {
        self.d.tags.create(&clean_name(name, "tag")?, clean_color(color)?.as_deref()).await
    }

    pub async fn delete_tag(&self, id: i64) -> Result<()> {
        self.d.tags.delete(id).await
    }

    pub async fn create_group(&self, name: &str, description: Option<&str>) -> Result<Group> {
        let d = description.map(str::trim).filter(|d| !d.is_empty());
        self.d.groups.create(&clean_name(name, "group")?, d).await
    }

    pub async fn rename_group(&self, id: i64, name: &str) -> Result<()> {
        self.d.groups.rename(id, &clean_name(name, "group")?).await
    }

    pub async fn delete_group(&self, id: i64) -> Result<()> {
        self.d.groups.delete(id).await
    }

    /// Save a view; an existing name is overwritten.
    pub async fn save_view(&self, name: &str, query: &Value) -> Result<SavedView> {
        self.d.views.save(&clean_name(name, "view")?, query).await
    }

    pub async fn delete_view(&self, id: i64) -> Result<()> {
        self.d.views.delete(id).await
    }

    /// Run `req.action` on each selected profile; a failure on one never stops the rest.
    pub async fn bulk(&self, req: BulkRequest) -> Result<BulkResult> {
        let mut ids: Vec<String> = Vec::new();
        for id in &req.profile_ids {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        if ids.is_empty() || ids.len() > BULK_MAX {
            return invalid(format!("select 1–{BULK_MAX} profiles"));
        }
        let mut results = Vec::with_capacity(ids.len());
        match req.action.as_str() {
            "start" => {
                for id in &ids {
                    results.push(item(id, self.start(id).await.map(drop)));
                }
            }
            "stop" => {
                for id in &ids {
                    let r = match self.profile(id).await {
                        Ok(_) => self.stop(id).await,
                        Err(e) => Err(e),
                    };
                    results.push(item(id, r));
                }
            }
            "delete" => {
                for id in &ids {
                    results.push(item(id, self.delete_profile(id).await));
                }
            }
            "proxy" => {
                for r in self.assign_many(req.proxy_id, &ids).await? {
                    let reason = r.reason;
                    results.push(BulkItem { error: reason.map(|x| x.to_string()), profile_id: r.profile_id, ok: r.ok, reason });
                }
            }
            "tag" | "untag" => {
                let Some(tag) = req.tag_id else { return invalid("tagId is required") };
                if !self.d.tags.list().await?.iter().any(|t| t.id == tag) {
                    return Err(CmError::NotFound(format!("tag #{tag}")).into());
                }
                for id in &ids {
                    let r = match self.profile(id).await {
                        Ok(_) if req.action == "tag" => self.d.tags.add(std::slice::from_ref(id), tag).await,
                        Ok(_) => self.d.tags.remove(std::slice::from_ref(id), tag).await,
                        Err(e) => Err(e),
                    };
                    results.push(item(id, r));
                }
            }
            "group" => {
                if let Some(g) = req.group_id
                    && !self.d.groups.list().await?.iter().any(|x| x.id == g)
                {
                    return Err(CmError::NotFound(format!("group #{g}")).into());
                }
                for id in &ids {
                    let r = match self.profile(id).await {
                        Ok(_) => self.d.groups.set(std::slice::from_ref(id), req.group_id).await,
                        Err(e) => Err(e),
                    };
                    results.push(item(id, r));
                }
            }
            other => return invalid(format!("unknown bulk action “{other}”")),
        }
        let ok = results.iter().filter(|r| r.ok).count();
        Ok(BulkResult { action: req.action, ok, failed: results.len() - ok, results })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::manager;
    use cm_core::{NewProfile, NewProxy};

    fn req(action: &str, ids: &[&str]) -> BulkRequest {
        BulkRequest {
            action: action.into(),
            profile_ids: ids.iter().map(|s| s.to_string()).collect(),
            proxy_id: None,
            tag_id: None,
            group_id: None,
        }
    }

    #[tokio::test]
    async fn bulk_actions_report_per_profile() {
        let (m, _engine) = manager(true, 39100);
        let a = m.create_profile(NewProfile { name: "a".into(), os: "win".into() }).await.unwrap();
        let b = m.create_profile(NewProfile { name: "b".into(), os: "mac".into() }).await.unwrap();
        let ids = [a.id.as_str(), b.id.as_str(), "ghost"];

        let tag = m.create_tag(" shop ", Some("#ABCDEF")).await.unwrap();
        assert_eq!((tag.name.as_str(), tag.color.as_deref()), ("shop", Some("#abcdef")));
        assert!(m.create_tag("x", Some("red")).await.is_err());
        assert!(m.create_tag("  ", None).await.is_err());
        let r = m.bulk(BulkRequest { tag_id: Some(tag.id), ..req("tag", &ids) }).await.unwrap();
        assert_eq!((r.ok, r.failed), (2, 1));
        assert_eq!(r.results[2].reason, Some("not-found"));
        let ov = m.org_overview().await.unwrap();
        assert_eq!(ov.profile_tags[&a.id], vec![tag.id]);
        m.bulk(BulkRequest { tag_id: Some(tag.id), ..req("untag", &[&a.id]) }).await.unwrap();
        assert!(!m.org_overview().await.unwrap().profile_tags.contains_key(&a.id));

        let g = m.create_group("G", None).await.unwrap();
        m.bulk(BulkRequest { group_id: Some(g.id), ..req("group", &[&a.id, &b.id]) }).await.unwrap();
        assert_eq!(m.org_overview().await.unwrap().profile_groups.len(), 2);
        m.bulk(req("group", &[&a.id])).await.unwrap(); // null = ungroup
        assert_eq!(m.org_overview().await.unwrap().profile_groups.len(), 1);
        assert!(m.bulk(BulkRequest { group_id: Some(99), ..req("group", &[&a.id]) }).await.is_err());

        let px =
            m.deps().proxies.insert(&NewProxy { name: "p".into(), host: "127.0.0.1".into(), port: 9, ..Default::default() }).await.unwrap();
        let r = m.bulk(BulkRequest { proxy_id: Some(px.id), ..req("proxy", &ids) }).await.unwrap();
        assert_eq!((r.ok, r.failed), (2, 1));

        // start a (needs proxy check ok in the fake), then bulk stop + delete
        let r = m.bulk(req("start", &[&a.id, "ghost"])).await.unwrap();
        assert_eq!((r.ok, r.failed), (1, 1));
        let r = m.bulk(req("stop", &[&a.id, &b.id])).await.unwrap();
        assert_eq!(r.ok, 2);
        // With the trash off (0 days) delete is immediate and the assignments go with the rows.
        m.update_settings(crate::settings::SettingsPatch { trash_days: Some(0), ..Default::default() }).await.unwrap();
        let r = m.bulk(req("delete", &ids)).await.unwrap();
        assert_eq!((r.ok, r.failed), (2, 1));
        let ov = m.org_overview().await.unwrap();
        assert!(ov.profile_groups.is_empty() && ov.profile_tags.is_empty(), "assignments go with the profiles");
        assert!(m.bulk(req("explode", &[&a.id])).await.is_err());
        assert!(m.bulk(req("start", &[])).await.is_err());
    }
}
