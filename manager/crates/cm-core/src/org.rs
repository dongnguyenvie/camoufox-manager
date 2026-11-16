//! Organization (phase 7): tags (many per profile), groups (one per profile), saved views.
//! Stored in their own tables so the profile record stays untouched.

use crate::{ProfileId, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: i64,
    pub name: String,
    /// CSS color, optional.
    pub color: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
}

/// A saved filter + sort of the profile list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedView {
    pub id: i64,
    pub name: String,
    /// UI-defined filter/sort state (opaque to the backend).
    pub query: Value,
}

#[async_trait]
pub trait TagRepo: Send + Sync {
    async fn list(&self) -> Result<Vec<Tag>>;
    async fn create(&self, name: &str, color: Option<&str>) -> Result<Tag>;
    async fn delete(&self, id: i64) -> Result<()>;
    /// Tag ids of one profile.
    async fn of_profile(&self, profile: &str) -> Result<Vec<i64>>;
    /// Every (profile, tag) pair — one query for the whole list view.
    async fn all_assignments(&self) -> Result<Vec<(ProfileId, i64)>>;
    async fn add(&self, profiles: &[ProfileId], tag: i64) -> Result<()>;
    async fn remove(&self, profiles: &[ProfileId], tag: i64) -> Result<()>;
}

#[async_trait]
pub trait GroupRepo: Send + Sync {
    async fn list(&self) -> Result<Vec<Group>>;
    async fn create(&self, name: &str, description: Option<&str>) -> Result<Group>;
    async fn rename(&self, id: i64, name: &str) -> Result<()>;
    async fn delete(&self, id: i64) -> Result<()>;
    /// Every (profile, group) pair.
    async fn all_assignments(&self) -> Result<Vec<(ProfileId, i64)>>;
    /// Move profiles to `group` (`None` = no group).
    async fn set(&self, profiles: &[ProfileId], group: Option<i64>) -> Result<()>;
}

#[async_trait]
pub trait SavedViewRepo: Send + Sync {
    async fn list(&self) -> Result<Vec<SavedView>>;
    async fn save(&self, name: &str, query: &Value) -> Result<SavedView>;
    async fn delete(&self, id: i64) -> Result<()>;
}
