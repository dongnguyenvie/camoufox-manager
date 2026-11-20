//! Profile transfer (phase 6): zip export/import, clone, bulk create.

use crate::profile::Os;
use serde::{Deserialize, Serialize};

/// What to do with the id of an imported profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportIdMode {
    /// Always a fresh id.
    #[default]
    New,
    /// Keep the archived id when it is free, otherwise a fresh one.
    KeepIfFree,
    /// Overwrite the data of an existing profile (journalled, with backup); the local id, group,
    /// schedules and proxy assignment stay. Needs a target profile id.
    Replace,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneRequest {
    pub name: String,
    /// Generate a new pinned machine instead of copying it.
    #[serde(default)]
    pub new_machine: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkCreateRequest {
    /// Name pattern, `{n}` replaced by 1..=count (e.g. `Shop {n}`).
    pub pattern: String,
    pub count: u32,
    pub os: Os,
    /// First number (default 1).
    #[serde(default)]
    pub start: Option<u32>,
}
