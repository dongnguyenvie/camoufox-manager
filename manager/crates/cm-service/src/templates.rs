//! Templates + Firefox extension library (phase 9).
//!
//! - A template is reusable profile settings (os, screen, window, locale, timezone, WebRTC, fonts,
//!   humanize, canvas, start URL, default proxy, tags, group, extensions). It never holds a
//!   machine: every profile made from it gets its own freshly generated pinned machine (D4).
//! - The extension library keeps uploaded `.xpi` files in `<data_dir>/extensions/`; they are
//!   assigned per profile or per template and installed into the profile at launch by the engine.

use crate::{Manager, profiles::ProfileView, profiles::new_id, profiles::validate_settings};
use anyhow::{Context, Result, bail};
use cm_core::{CmError, Extension, Profile, Template, snapshot::expand_pattern};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, io::Read, path::PathBuf};

/// Largest accepted `.xpi`.
pub const MAX_XPI_BYTES: usize = 64 * 1024 * 1024;
const MAX_TEMPLATE_PROFILES: u32 = 100;

fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(CmError::Invalid(msg.into()).into())
}

/// What `.xpi` carries that the library needs.
#[derive(Debug, PartialEq)]
pub struct XpiInfo {
    pub addon_id: String,
    pub name: String,
    pub version: Option<String>,
}

fn read_entry(z: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>, name: &str, max: u64) -> Option<Vec<u8>> {
    let f = z.by_name(name).ok()?;
    let mut buf = Vec::new();
    f.take(max).read_to_end(&mut buf).ok()?;
    Some(buf)
}

/// Validate an `.xpi` (a zip with `manifest.json` and a Gecko add-on id) and read its identity.
pub fn parse_xpi(bytes: &[u8]) -> Result<XpiInfo> {
    if bytes.is_empty() || bytes.len() > MAX_XPI_BYTES {
        return invalid("extension must be a .xpi of at most 64 MiB");
    }
    let mut z =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| CmError::Invalid("not a zip archive (.xpi expected)".into()))?;
    let Some(raw) = read_entry(&mut z, "manifest.json", 1 << 20) else {
        return invalid("the archive has no manifest.json at its root");
    };
    let text = String::from_utf8_lossy(&raw);
    let manifest: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|e| CmError::Invalid(format!("manifest.json is not valid JSON: {e}")))?;
    let gecko_id = ["browser_specific_settings", "applications"]
        .iter()
        .find_map(|k| manifest.get(*k)?.get("gecko")?.get("id")?.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let Some(addon_id) = gecko_id else {
        return invalid("manifest.json has no browser_specific_settings.gecko.id (Firefox needs an add-on id to install it)");
    };
    if addon_id.len() > 200 || addon_id.starts_with('.') || addon_id.chars().any(|c| c.is_control() || c == '/' || c == '\\') {
        return invalid("unusable add-on id in manifest.json");
    }
    let mut name = manifest.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    // `__MSG_key__` names live in _locales/<default_locale>/messages.json.
    if let Some(key) = name.strip_prefix("__MSG_").and_then(|n| n.strip_suffix("__")) {
        let locale = manifest.get("default_locale").and_then(|v| v.as_str()).unwrap_or("en");
        let msgs = read_entry(&mut z, &format!("_locales/{locale}/messages.json"), 1 << 20)
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
        let found = msgs.and_then(|m| {
            m.as_object()?.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).and_then(|(_, v)| v.get("message")?.as_str().map(String::from))
        });
        name = found.unwrap_or_default();
    }
    if name.is_empty() {
        name = addon_id.to_string();
    }
    let version = manifest.get("version").and_then(|v| v.as_str()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    Ok(XpiInfo { addon_id: addon_id.to_string(), name: name.chars().take(120).collect(), version })
}

/// File name used in the library for an add-on id.
fn library_file(addon_id: &str) -> String {
    let safe: String = addon_id.chars().map(|c| if c.is_ascii_alphanumeric() || "@._-".contains(c) { c } else { '_' }).collect();
    format!("{safe}.xpi")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionView {
    #[serde(flatten)]
    pub ext: Extension,
    /// Profiles / templates that use it.
    pub profiles: Vec<String>,
    pub templates: Vec<i64>,
}

/// Body of `POST /api/templates/{id}/create`.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FromTemplate {
    /// How many profiles (default 1, at most 100).
    #[serde(default)]
    pub count: Option<u32>,
    /// Name pattern with `{n}` (`Shop {n}`); required for more than one profile.
    #[serde(default)]
    pub pattern: Option<String>,
    /// Name of a single profile (default: the template name).
    #[serde(default)]
    pub name: Option<String>,
    /// First number for `{n}` (default 1).
    #[serde(default)]
    pub start: Option<u32>,
}

#[derive(Serialize)]
pub struct FromTemplateResult {
    pub created: Vec<ProfileView>,
    /// Set when creation stopped part-way (the ones already made are kept).
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AssignMode {
    #[default]
    Add,
    Remove,
    Set,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignRequest {
    pub profiles: Vec<String>,
    pub extensions: Vec<i64>,
    #[serde(default)]
    pub mode: AssignMode,
}

impl Manager {
    pub fn extensions_dir(&self) -> PathBuf {
        self.cfg.data_dir.join("extensions")
    }

    // ---- extension library ------------------------------------------------------------------

    pub async fn list_extensions(&self) -> Result<Vec<ExtensionView>> {
        let exts = self.d.extensions.list().await?;
        let assigned = self.d.extensions.all_assignments().await?;
        let templates = self.d.templates.list().await?;
        Ok(exts
            .into_iter()
            .map(|e| ExtensionView {
                profiles: assigned.iter().filter(|(_, x)| *x == e.id).map(|(p, _)| p.clone()).collect(),
                templates: templates.iter().filter(|t| t.extensions.contains(&e.id)).map(|t| t.id).collect(),
                ext: e,
            })
            .collect())
    }

    /// Validate and store an uploaded `.xpi`. The same add-on id twice is a conflict.
    pub async fn add_extension(&self, bytes: &[u8]) -> Result<ExtensionView> {
        let info = parse_xpi(bytes)?;
        let dir = self.extensions_dir();
        fs::create_dir_all(&dir)?;
        crate::transfer::set_mode(&dir, 0o700)?;
        let file = library_file(&info.addon_id);
        if self.d.extensions.list().await?.iter().any(|e| e.addon_id == info.addon_id) {
            bail!(CmError::Conflict(format!(
                "{} ({}) is already in the library; delete it first to upload another version",
                info.name, info.addon_id
            )));
        }
        let tmp = dir.join(format!("{file}.{}.tmp", new_id()?));
        fs::write(&tmp, bytes).context("cannot store the extension")?;
        fs::rename(&tmp, dir.join(&file))?;
        let stored = match self
            .d
            .extensions
            .insert(&Extension { id: 0, name: info.name, addon_id: info.addon_id, version: info.version, file: file.clone() })
            .await
        {
            Ok(e) => e,
            Err(e) => {
                let _ = fs::remove_file(dir.join(&file));
                return Err(e);
            }
        };
        Ok(ExtensionView { ext: stored, profiles: vec![], templates: vec![] })
    }

    /// Remove from the library, from every profile and template, and from disk. Browsers that are
    /// running keep it until their next start; at that start it is uninstalled.
    pub async fn delete_extension(&self, id: i64) -> Result<()> {
        let ext = self
            .d
            .extensions
            .list()
            .await?
            .into_iter()
            .find(|e| e.id == id)
            .ok_or_else(|| CmError::NotFound(format!("extension #{id}")))?;
        self.d.extensions.delete(id).await?;
        let _ = fs::remove_file(self.extensions_dir().join(&ext.file));
        Ok(())
    }

    pub async fn profile_extensions(&self, profile: &str) -> Result<Vec<i64>> {
        self.profile(profile).await?;
        self.d.extensions.of_profile(profile).await
    }

    /// Replace the extensions of one profile. Applied at the next start.
    pub async fn set_profile_extensions(&self, profile: &str, ids: &[i64]) -> Result<Vec<i64>> {
        self.profile(profile).await?;
        let ids = self.known_extensions(ids).await?;
        self.d.extensions.set_for_profile(profile, &ids).await?;
        Ok(ids)
    }

    /// Add / remove / set extensions on many profiles at once.
    pub async fn assign_extensions(&self, req: AssignRequest) -> Result<usize> {
        let ids = self.known_extensions(&req.extensions).await?;
        let mut n = 0;
        for p in &req.profiles {
            self.profile(p).await?;
            let cur = self.d.extensions.of_profile(p).await?;
            let next: BTreeSet<i64> = match req.mode {
                AssignMode::Add => cur.iter().chain(&ids).copied().collect(),
                AssignMode::Remove => cur.iter().copied().filter(|x| !ids.contains(x)).collect(),
                AssignMode::Set => ids.iter().copied().collect(),
            };
            self.d.extensions.set_for_profile(p, &next.into_iter().collect::<Vec<_>>()).await?;
            n += 1;
        }
        Ok(n)
    }

    async fn known_extensions(&self, ids: &[i64]) -> Result<Vec<i64>> {
        let known: Vec<i64> = self.d.extensions.list().await?.into_iter().map(|e| e.id).collect();
        let mut out = BTreeSet::new();
        for id in ids {
            if !known.contains(id) {
                bail!(CmError::NotFound(format!("extension #{id}")));
            }
            out.insert(*id);
        }
        Ok(out.into_iter().collect())
    }

    /// `.xpi` files to install when `profile` starts (library files that still exist).
    pub(crate) async fn extension_files(&self, profile: &str) -> Vec<PathBuf> {
        let Ok(ids) = self.d.extensions.of_profile(profile).await else { return Vec::new() };
        if ids.is_empty() {
            return Vec::new();
        }
        let Ok(all) = self.d.extensions.list().await else { return Vec::new() };
        let dir = self.extensions_dir();
        all.into_iter().filter(|e| ids.contains(&e.id)).map(|e| dir.join(e.file)).filter(|p| p.is_file()).collect()
    }

    // ---- templates --------------------------------------------------------------------------

    pub async fn list_templates(&self) -> Result<Vec<Template>> {
        self.d.templates.list().await
    }

    pub async fn template(&self, id: i64) -> Result<Template> {
        self.d.templates.get(id).await?.ok_or_else(|| CmError::NotFound(format!("template #{id}")).into())
    }

    /// Validate everything a template points at, normalise settings.
    async fn clean_template(&self, mut t: Template) -> Result<Template> {
        t.name = t.name.trim().to_string();
        if t.name.is_empty() || t.name.chars().count() > 80 {
            return invalid("template name must be 1-80 characters");
        }
        t.settings = validate_settings(t.settings, None)?;
        if let Some(p) = t.proxy_id {
            self.proxy(p).await?;
        }
        if let Some(g) = t.group_id
            && !self.d.groups.list().await?.iter().any(|x| x.id == g)
        {
            bail!(CmError::NotFound(format!("group #{g}")));
        }
        let tags = self.d.tags.list().await?;
        for id in &t.tags {
            if !tags.iter().any(|x| x.id == *id) {
                bail!(CmError::NotFound(format!("tag #{id}")));
            }
        }
        t.tags = t.tags.into_iter().collect::<BTreeSet<_>>().into_iter().collect();
        t.extensions = self.known_extensions(&t.extensions).await?;
        Ok(t)
    }

    pub async fn create_template(&self, mut t: Template) -> Result<Template> {
        t.id = 0;
        let t = self.clean_template(t).await?;
        self.d.templates.save(&t).await
    }

    pub async fn update_template(&self, id: i64, mut t: Template) -> Result<Template> {
        self.template(id).await?;
        t.id = id;
        let t = self.clean_template(t).await?;
        self.d.templates.save(&t).await
    }

    pub async fn delete_template(&self, id: i64) -> Result<()> {
        self.d.templates.delete(id).await
    }

    /// "Save profile settings as template": copies the settings, default proxy, tags, group and
    /// extensions of a profile. The pinned machine is deliberately left out; only its OS is kept.
    pub async fn template_from_profile(&self, profile: &str, name: &str) -> Result<Template> {
        let p = self.profile(profile).await?;
        let tags = self.d.tags.of_profile(profile).await?;
        let group_id = self.d.groups.all_assignments().await?.into_iter().find(|(pid, _)| pid == profile).map(|(_, g)| g);
        let extensions = self.d.extensions.of_profile(profile).await?;
        let t =
            Template { id: 0, name: name.to_string(), os: p.os, settings: p.settings, extensions, proxy_id: p.proxy_id, tags, group_id };
        self.create_template(t).await
    }

    /// Create `count` profiles from a template; each one pins its own new machine.
    pub async fn create_from_template(&self, id: i64, req: FromTemplate) -> Result<FromTemplateResult> {
        let t = self.template(id).await?;
        let count = req.count.unwrap_or(1);
        if count == 0 || count > MAX_TEMPLATE_PROFILES {
            return invalid(format!("count must be 1-{MAX_TEMPLATE_PROFILES}"));
        }
        let start = req.start.unwrap_or(1);
        let end = start.checked_add(count).context("number out of range")?;
        let pattern = req.pattern.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let names: Vec<String> = if count == 1 && pattern.is_none() {
            let n = req.name.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or(&t.name).to_string();
            if n.chars().count() > 80 {
                return invalid("profile name must be 1-80 characters");
            }
            vec![n]
        } else {
            let Some(pat) = pattern else { return invalid("a name pattern with {n} is required to create more than one profile") };
            (start..end).map(|n| expand_pattern(pat, n)).collect::<Result<_>>()?
        };
        let mut ids = Vec::new();
        let mut error = None;
        for name in names {
            match self.new_profile_from(&t, name).await {
                Ok(pid) => ids.push(pid),
                Err(e) => {
                    error = Some(format!("{e:#}"));
                    break;
                }
            }
        }
        let mut views = self.overview().await.profiles;
        let mut out = Vec::new();
        for pid in &ids {
            if let Some(pos) = views.iter().position(|v| &v.id == pid) {
                out.push(views.swap_remove(pos));
            }
        }
        Ok(FromTemplateResult { created: out, error })
    }

    async fn new_profile_from(&self, t: &Template, name: String) -> Result<String> {
        let machine = self.d.engine.generate(t.os).await?;
        let now = self.d.clock.now_iso();
        let mut p = Profile {
            id: new_id()?,
            name,
            os: t.os,
            notes: None,
            created_at: now.clone(),
            updated_at: now,
            last_activity: None,
            proxy_id: t.proxy_id,
            machine,
            settings: t.settings.clone(),
        };
        // The template's window/screen was validated without a machine; check it against this one.
        let screen = self.pinned_screen(&p);
        p.settings = validate_settings(p.settings, screen)?;
        self.d.profiles.insert(&p).await?;
        let id = p.id.clone();
        let rest = async {
            for tag in &t.tags {
                self.d.tags.add(std::slice::from_ref(&id), *tag).await?;
            }
            if t.group_id.is_some() {
                self.d.groups.set(std::slice::from_ref(&id), t.group_id).await?;
            }
            if !t.extensions.is_empty() {
                self.d.extensions.set_for_profile(&id, &t.extensions).await?;
            }
            Ok::<(), anyhow::Error>(())
        }
        .await;
        if let Err(e) = rest {
            let _ = self.d.profiles.delete(&id).await; // do not leave a half-made profile behind
            return Err(e);
        }
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::manager;
    use cm_core::Os;
    use std::io::Write;

    pub(crate) fn xpi(manifest: &str) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            z.start_file("manifest.json", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(manifest.as_bytes()).unwrap();
            z.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn xpi_validation() {
        let ok = parse_xpi(&xpi(r#"{"name":"T","version":"1.2","browser_specific_settings":{"gecko":{"id":"t@x"}}}"#)).unwrap();
        assert_eq!(ok, XpiInfo { addon_id: "t@x".into(), name: "T".into(), version: Some("1.2".into()) });
        let legacy = parse_xpi(&xpi(r#"{"applications":{"gecko":{"id":"{abc}"}}}"#)).unwrap();
        assert_eq!((legacy.addon_id.as_str(), legacy.name.as_str()), ("{abc}", "{abc}"));
        assert!(parse_xpi(b"not a zip").is_err());
        assert!(parse_xpi(&xpi("{}")).unwrap_err().to_string().contains("gecko.id"));
        assert!(parse_xpi(&xpi("nope")).unwrap_err().to_string().contains("JSON"));
        assert_eq!(library_file("{a/b}@x"), "_a_b_@x.xpi");
    }

    fn tmp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("cm-tpl-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[tokio::test]
    async fn library_upload_assign_delete() {
        let (mut m, _) = manager(true, 41990);
        m.cfg.data_dir = tmp("lib");
        let bytes = xpi(r#"{"name":"T","version":"1","browser_specific_settings":{"gecko":{"id":"t@x"}}}"#);
        let e = m.add_extension(&bytes).await.unwrap();
        assert!(m.extensions_dir().join(&e.ext.file).is_file());
        assert!(m.add_extension(&bytes).await.is_err(), "same add-on twice");
        let p = m.create_profile(cm_core::NewProfile { name: "a".into(), os: "mac".into() }).await.unwrap();
        m.set_profile_extensions(&p.id, &[e.ext.id]).await.unwrap();
        assert_eq!(m.extension_files(&p.id).await.len(), 1);
        assert!(m.set_profile_extensions(&p.id, &[999]).await.is_err());
        assert_eq!(m.list_extensions().await.unwrap()[0].profiles, vec![p.id.clone()]);
        m.assign_extensions(AssignRequest { profiles: vec![p.id.clone()], extensions: vec![e.ext.id], mode: AssignMode::Remove })
            .await
            .unwrap();
        assert!(m.extension_files(&p.id).await.is_empty());
        m.delete_extension(e.ext.id).await.unwrap();
        assert!(!m.extensions_dir().join(&e.ext.file).exists());
        let _ = fs::remove_dir_all(m.cfg.data_dir.clone());
    }

    #[tokio::test]
    async fn templates_make_profiles_with_own_machines() {
        let (mut m, engine) = manager(true, 41991);
        m.cfg.data_dir = tmp("tpl");
        let tag = m.create_tag("work", None).await.unwrap();
        let group = m.create_group("g", None).await.unwrap();
        let e = m.add_extension(&xpi(r#"{"name":"T","browser_specific_settings":{"gecko":{"id":"t@x"}}}"#)).await.unwrap();
        let mut t = Template {
            name: "Base".into(),
            os: Os::Lin,
            tags: vec![tag.id],
            group_id: Some(group.id),
            extensions: vec![e.ext.id],
            ..Default::default()
        };
        t.settings.start_url = Some("https://example.com".into());
        t.settings.humanize = Some(true);
        let t = m.create_template(t).await.unwrap();
        assert!(m.create_template(Template { name: "bad".into(), tags: vec![999], ..Default::default() }).await.is_err());
        let r = m
            .create_from_template(t.id, FromTemplate { count: Some(3), pattern: Some("Shop {n}".into()), name: None, start: None })
            .await
            .unwrap();
        assert!(r.error.is_none());
        let names: Vec<_> = r.created.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["Shop 1", "Shop 2", "Shop 3"]);
        let mut ids: Vec<_> = r.created.iter().map(|v| v.id.clone()).collect();
        ids.dedup();
        assert_eq!(ids.len(), 3);
        for v in &r.created {
            assert_eq!(v.os, Os::Lin);
            assert_eq!(v.settings.start_url.as_deref(), Some("https://example.com"));
            assert_eq!(m.d.extensions.of_profile(&v.id).await.unwrap(), vec![e.ext.id]);
            assert_eq!(m.d.tags.of_profile(&v.id).await.unwrap(), vec![tag.id]);
        }
        // the extension reaches the engine at launch
        let id = &r.created[0].id;
        m.start(id).await.unwrap();
        assert_eq!(engine.launched.lock().unwrap().last().unwrap().extensions.len(), 1);
        m.stop(id).await.unwrap();
        // single profile, default name; and a profile saved back as a template
        let one = m.create_from_template(t.id, FromTemplate { count: None, pattern: None, name: None, start: None }).await.unwrap();
        assert_eq!(one.created[0].name, "Base");
        let again = m.template_from_profile(id, "Copy of Base").await.unwrap();
        assert_eq!(
            (again.os, again.tags.clone(), again.group_id, again.extensions.clone()),
            (Os::Lin, vec![tag.id], Some(group.id), vec![e.ext.id])
        );
        assert_eq!(again.settings.start_url.as_deref(), Some("https://example.com"));
        assert!(m.create_from_template(t.id, FromTemplate { count: Some(2), pattern: None, name: None, start: None }).await.is_err());
        let _ = fs::remove_dir_all(m.cfg.data_dir.clone());
    }
}
