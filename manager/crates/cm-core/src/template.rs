//! Templates and Firefox extensions (phase 9).
//!
//! A template is reusable settings without a machine: profiles created from it get a fresh
//! pinned machine for `os`.

use crate::{Result, profile::Os, profile::ProfileSettings};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: i64,
    pub name: String,
    pub os: Os,
    pub settings: ProfileSettings,
    /// Extension ids installed into profiles made from this template.
    pub extensions: Vec<i64>,
    pub proxy_id: Option<crate::ProxyId>,
}

/// An `.xpi` in the extension library.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Extension {
    pub id: i64,
    pub name: String,
    /// Add-on id from the manifest.
    pub addon_id: String,
    pub version: Option<String>,
    /// File name inside `<data_dir>/extensions/`.
    pub file: String,
}

#[async_trait]
pub trait TemplateRepo: Send + Sync {
    async fn list(&self) -> Result<Vec<Template>>;
    async fn get(&self, id: i64) -> Result<Option<Template>>;
    /// Insert when `t.id == 0`, otherwise update. Returns the stored template.
    async fn save(&self, t: &Template) -> Result<Template>;
    async fn delete(&self, id: i64) -> Result<()>;
}

#[async_trait]
pub trait ExtensionRepo: Send + Sync {
    async fn list(&self) -> Result<Vec<Extension>>;
    async fn insert(&self, e: &Extension) -> Result<Extension>;
    async fn delete(&self, id: i64) -> Result<()>;
    /// Extension ids assigned to a profile.
    async fn of_profile(&self, profile: &str) -> Result<Vec<i64>>;
    async fn set_for_profile(&self, profile: &str, ids: &[i64]) -> Result<()>;
}
