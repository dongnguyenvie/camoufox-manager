//! Manifest exchange, per-profile diff and the compatibility gate (phase 17, D9).
//!
//! Each side lists its profiles as [`ManifestEntry`]: id, name, `rev`, content hash, size, whether
//! it is closed, and what *it* remembers about the last sync with the asking device (`link`:
//! which profile on the other side, and the content hash both agreed on). Deciding what changed
//! needs no clock and no shared hash: every side compares its own current hash with its own
//! `last_synced_hash`.
//!
//! | source changed | target changed | status |
//! |---|---|---|
//! | – | – | [`Status::Identical`] |
//! | yes | no | [`Status::SourceChanged`] |
//! | no | yes | [`Status::TargetChanged`] |
//! | yes | yes | [`Status::Conflict`] (never merged) |
//!
//! No counterpart → [`Status::MissingOnTarget`]; a counterpart with the same id that was never
//! synced → [`Status::Unlinked`] (treated like a conflict).

use serde::{Deserialize, Serialize};

/// What decides compatibility: snapshots only move between identical OS, CPU arch and Camoufox build.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Env {
    pub os: String,
    pub arch: String,
    #[serde(default)]
    pub camoufox_build: Option<String>,
}

impl Env {
    /// `Err(reason)` (plain language) unless snapshots can move between `self` and `other`.
    pub fn check_compatible(&self, other: &Env) -> Result<(), String> {
        if self.os != other.os {
            return Err(format!("different operating systems ({} and {})", self.os, other.os));
        }
        if self.arch != other.arch {
            return Err(format!("different processor types ({} and {})", self.arch, other.arch));
        }
        match (&self.camoufox_build, &other.camoufox_build) {
            (Some(a), Some(b)) if a == b => Ok(()),
            (Some(a), Some(b)) => Err(format!("different Camoufox builds ({a} and {b}); update Camoufox on the older one")),
            _ => Err("the Camoufox build of one device is unknown (is Camoufox installed on both?)".into()),
        }
    }
}

/// What a device remembers about the last sync of one profile with the asking device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkInfo {
    /// The same profile's id on the other device.
    pub peer_profile_id: String,
    pub last_synced_hash: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    pub id: String,
    pub name: String,
    /// Local revision counter: grows each time the content hash is seen to change.
    pub rev: u64,
    /// `None` while the profile is running (not hashed).
    pub content_hash: Option<String>,
    /// Bytes of the profile's data (uncompressed).
    pub size: u64,
    pub closed: bool,
    pub link: Option<LinkInfo>,
}

/// Answer of `GET /sync/v1/profiles`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerManifest {
    pub device_id: String,
    pub name: String,
    pub env: Env,
    pub profiles: Vec<ManifestEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Copy the other device's profiles here.
    Pull,
    /// Copy this device's profiles to the other one.
    Push,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    MissingOnTarget,
    SourceChanged,
    TargetChanged,
    Conflict,
    /// Same id exists on the target but the two were never synced.
    Unlinked,
    Identical,
}

/// What to do with one profile. Source = the side the data comes from, target = where it goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// New profile on the target.
    Create,
    /// The source's data replaces the linked profile on the target (a backup is kept there).
    Replace,
    /// Add the source's version as a new, separate profile on the target.
    Copy,
    /// Leave both as they are.
    Skip,
}

impl Status {
    /// Actions the user may choose, the default first.
    pub fn actions(self) -> &'static [Action] {
        match self {
            Status::MissingOnTarget => &[Action::Create, Action::Skip],
            Status::SourceChanged => &[Action::Replace, Action::Copy, Action::Skip],
            Status::TargetChanged | Status::Conflict | Status::Unlinked => &[Action::Skip, Action::Replace, Action::Copy],
            Status::Identical => &[Action::Skip],
        }
    }

    pub fn default_action(self) -> Action {
        self.actions()[0]
    }

    /// Needs an explicit decision (both sides moved on, or never linked).
    pub fn needs_choice(self) -> bool {
        matches!(self, Status::Conflict | Status::Unlinked | Status::TargetChanged)
    }
}

/// One side of a pair, as [`classify`] needs it.
#[derive(Clone, Debug)]
pub struct Side<'a> {
    pub hash: Option<&'a str>,
    pub last_synced: Option<&'a str>,
}

impl Side<'_> {
    fn changed(&self) -> bool {
        match (self.hash, self.last_synced) {
            (Some(h), Some(l)) => h != l,
            _ => true,
        }
    }
}

/// Status of a linked pair.
pub fn classify(source: &Side, target: &Side) -> Status {
    if let (Some(a), Some(b)) = (source.hash, target.hash)
        && a == b
    {
        return Status::Identical;
    }
    match (source.changed(), target.changed()) {
        (false, false) => Status::Identical,
        (true, false) => Status::SourceChanged,
        (false, true) => Status::TargetChanged,
        (true, true) => Status::Conflict,
    }
}

/// Why a row cannot be synced right now (stable `code` for the UI + plain `detail`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blocked {
    pub code: String,
    pub detail: String,
}

/// One line of the sync dialog.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    /// Profile id on the source device.
    pub source_id: String,
    pub name: String,
    pub size: u64,
    pub rev: u64,
    pub status: Status,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub actions: Vec<Action>,
    pub default_action: Action,
    pub needs_choice: bool,
    pub blocked: Option<Blocked>,
}

/// Pair the source's profiles with the target's and classify each.
///
/// `compat`: result of the environment gate (applies to every row). Profiles that are running on
/// the source are blocked; a running target profile blocks everything that would change it.
pub fn build_rows(source: &[ManifestEntry], target: &[ManifestEntry], compat: &Result<(), String>) -> Vec<Row> {
    let mut rows = Vec::new();
    for s in source {
        // 1. what the source remembers, 2. what the target remembers, 3. the same id, never synced
        let by_source = s.link.as_ref().and_then(|l| target.iter().find(|t| t.id == l.peer_profile_id));
        let by_target = || target.iter().find(|t| t.link.as_ref().is_some_and(|l| l.peer_profile_id == s.id));
        let linked = by_source.or_else(by_target);
        let unlinked = || target.iter().find(|t| t.id == s.id);
        let (status, t) = match (linked, unlinked()) {
            (Some(t), _) => {
                let ssd = s.link.as_ref().filter(|l| l.peer_profile_id == t.id).and_then(|l| l.last_synced_hash.as_deref());
                let tsd = t.link.as_ref().filter(|l| l.peer_profile_id == s.id).and_then(|l| l.last_synced_hash.as_deref());
                let st = classify(
                    &Side { hash: s.content_hash.as_deref(), last_synced: ssd },
                    &Side { hash: t.content_hash.as_deref(), last_synced: tsd },
                );
                (st, Some(t))
            }
            (None, Some(t)) if s.content_hash.is_some() && s.content_hash == t.content_hash => (Status::Identical, Some(t)),
            (None, Some(t)) => (Status::Unlinked, Some(t)),
            (None, None) => (Status::MissingOnTarget, None),
        };
        let mut actions = status.actions().to_vec();
        let mut blocked = None;
        if let Err(reason) = compat {
            blocked = Some(Blocked { code: "incompatible".into(), detail: reason.clone() });
        } else if !s.closed {
            blocked = Some(Blocked { code: "source_running".into(), detail: "the profile is running; close it first".into() });
        } else if status != Status::Identical && status != Status::MissingOnTarget && t.is_some_and(|t| !t.closed) {
            // Copy adds a new profile and does not touch the running one.
            actions.retain(|a| *a == Action::Copy || *a == Action::Skip);
            if !actions.contains(&Action::Copy) {
                blocked = Some(Blocked { code: "target_running".into(), detail: "the profile is running on the other device".into() });
            }
        }
        let wanted = status.default_action();
        let default_action = if blocked.is_some() || !actions.contains(&wanted) { Action::Skip } else { wanted };
        rows.push(Row {
            source_id: s.id.clone(),
            name: s.name.clone(),
            size: s.size,
            rev: s.rev,
            status,
            target_id: t.map(|t| t.id.clone()),
            target_name: t.map(|t| t.name.clone()),
            actions,
            default_action,
            needs_choice: status.needs_choice(),
            blocked,
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, hash: Option<&str>, link: Option<(&str, Option<&str>)>) -> ManifestEntry {
        ManifestEntry {
            id: id.into(),
            name: format!("P {id}"),
            rev: 1,
            content_hash: hash.map(str::to_owned),
            size: 10,
            closed: hash.is_some(),
            link: link.map(|(p, h)| LinkInfo { peer_profile_id: p.into(), last_synced_hash: h.map(str::to_owned) }),
        }
    }

    fn env(b: Option<&str>) -> Env {
        Env { os: "macos".into(), arch: "aarch64".into(), camoufox_build: b.map(str::to_owned) }
    }

    fn one(s: ManifestEntry, t: Vec<ManifestEntry>) -> Row {
        build_rows(&[s], &t, &Ok(())).remove(0)
    }

    #[test]
    fn missing_on_target() {
        let r = one(entry("a", Some("h1"), None), vec![]);
        assert_eq!(r.status, Status::MissingOnTarget);
        assert_eq!(r.default_action, Action::Create);
        assert!(r.target_id.is_none() && r.blocked.is_none() && !r.needs_choice);
    }

    #[test]
    fn only_source_changed() {
        // both remember "h0"; source moved to h1, target still h0
        let r = one(entry("a", Some("h1"), Some(("b", Some("h0")))), vec![entry("b", Some("h0"), Some(("a", Some("h0"))))]);
        assert_eq!(r.status, Status::SourceChanged);
        assert_eq!(r.default_action, Action::Replace);
        assert_eq!(r.target_id.as_deref(), Some("b"));
    }

    #[test]
    fn only_target_changed_defaults_to_skip() {
        let r = one(entry("a", Some("h0"), Some(("b", Some("h0")))), vec![entry("b", Some("h9"), Some(("a", Some("h0"))))]);
        assert_eq!(r.status, Status::TargetChanged);
        assert_eq!(r.default_action, Action::Skip);
        assert!(r.actions.contains(&Action::Replace) && r.actions.contains(&Action::Copy));
    }

    #[test]
    fn both_changed_is_a_conflict_never_auto() {
        let r = one(entry("a", Some("h1"), Some(("b", Some("h0")))), vec![entry("b", Some("h2"), Some(("a", Some("h0"))))]);
        assert_eq!(r.status, Status::Conflict);
        assert!(r.needs_choice);
        assert_eq!(r.default_action, Action::Skip);
        assert_eq!(r.actions, vec![Action::Skip, Action::Replace, Action::Copy]);
    }

    #[test]
    fn identical() {
        let r = one(entry("a", Some("h0"), Some(("b", Some("h0")))), vec![entry("b", Some("h0"), Some(("a", Some("h0"))))]);
        assert_eq!(r.status, Status::Identical);
        // equal hashes are identical even when the link memory is missing
        let r = one(entry("a", Some("hx"), None), vec![entry("a", Some("hx"), None)]);
        assert_eq!(r.status, Status::Identical);
    }

    #[test]
    fn link_found_from_either_side_and_same_id_unlinked() {
        // only the target remembers the link
        let r = one(entry("a", Some("h0"), None), vec![entry("b", Some("h0"), Some(("a", Some("h0"))))]);
        assert_eq!(r.target_id.as_deref(), Some("b"));
        // same id on the target, never synced, different content
        let r = one(entry("a", Some("h1"), None), vec![entry("a", Some("h2"), None)]);
        assert_eq!(r.status, Status::Unlinked);
        assert!(r.needs_choice && r.default_action == Action::Skip);
        // a deleted counterpart is simply missing again
        let r = one(entry("a", Some("h1"), Some(("gone", Some("h0")))), vec![]);
        assert_eq!(r.status, Status::MissingOnTarget);
    }

    #[test]
    fn running_profiles_are_blocked_with_a_reason() {
        let r = one(entry("a", None, None), vec![]);
        assert_eq!(r.blocked.as_ref().unwrap().code, "source_running");
        assert_eq!(r.default_action, Action::Skip);
        // target running: replace is off, copy stays
        let mut t = entry("b", Some("h2"), Some(("a", Some("h0"))));
        t.closed = false;
        let r = one(entry("a", Some("h1"), Some(("b", Some("h0")))), vec![t]);
        assert_eq!(r.status, Status::Conflict);
        assert_eq!(r.actions, vec![Action::Skip, Action::Copy]);
        assert!(r.blocked.is_none());
    }

    #[test]
    fn compatibility_gate() {
        assert!(env(Some("135.0")).check_compatible(&env(Some("135.0"))).is_ok());
        let e = env(Some("135.0")).check_compatible(&env(Some("136.0"))).unwrap_err();
        assert!(e.contains("Camoufox builds"), "{e}");
        let mut linux = env(Some("135.0"));
        linux.os = "linux".into();
        assert!(env(Some("135.0")).check_compatible(&linux).unwrap_err().contains("operating systems"));
        let mut x86 = env(Some("135.0"));
        x86.arch = "x86_64".into();
        assert!(env(Some("135.0")).check_compatible(&x86).unwrap_err().contains("processor"));
        assert!(env(None).check_compatible(&env(Some("135.0"))).unwrap_err().contains("unknown"));
        assert!(env(None).check_compatible(&env(None)).is_err(), "unknown on both sides cannot be verified");
        // the gate blocks every row with its reason
        let rows = build_rows(&[entry("a", Some("h"), None)], &[], &Err("different Camoufox builds".into()));
        assert_eq!(rows[0].blocked.as_ref().unwrap().code, "incompatible");
    }
}
