//! App settings: a small key/value store. Values are JSON so each area can keep typed data.
//! Well-known keys live here so they are not spelled twice.

use crate::Result;
use async_trait::async_trait;
use serde_json::Value;

/// UI language (`"en"` | `"vi"`), default `"en"` (D1).
pub const LANGUAGE: &str = "ui.language";
/// Set once the old JSON/txt files were imported into the database.
pub const LEGACY_IMPORTED: &str = "legacy.imported";
/// Default headless mode for launches (phase 12).
pub const HEADLESS: &str = "engine.headless";
/// Optional API key for non-UI clients (phase 12). Superseded by named keys ([`ApiKeyRepo`]).
pub const API_KEY: &str = "api.key";
/// `true`: requests that do not come from the embedded UI must carry a valid `X-API-Key` (phase 12).
pub const REQUIRE_API_KEY: &str = "api.require_key";

/// Days a deleted profile stays in the trash (CM-3.1); `0` = delete immediately. Default 7.
pub const TRASH_DAYS: &str = "profiles.trash_days";

pub const LANGUAGES: [&str; 2] = ["en", "vi"];

#[async_trait]
pub trait SettingsRepo: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Value>>;
    async fn set(&self, key: &str, value: &Value) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
}

/// Public part of a named API key (one per agent / machine). The secret is shown once at creation
/// and only a salted hash is stored.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyInfo {
    /// Public id, also the middle part of the key string `cmk_<id>_<secret>`.
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub revoked_at: Option<String>,
}

/// What the store keeps: the info plus `sha256(salt || secret)` and the salt (hex).
#[derive(Clone, Debug, PartialEq)]
pub struct ApiKeyRecord {
    pub info: ApiKeyInfo,
    pub salt: String,
    pub hash: String,
}

#[async_trait]
pub trait ApiKeyRepo: Send + Sync {
    async fn insert(&self, rec: &ApiKeyRecord) -> Result<()>;
    /// Newest first, revoked keys included.
    async fn list(&self) -> Result<Vec<ApiKeyInfo>>;
    async fn get(&self, id: &str) -> Result<Option<ApiKeyRecord>>;
    async fn touch(&self, id: &str, at: &str) -> Result<()>;
    /// `false` when the key does not exist or was already revoked.
    async fn revoke(&self, id: &str, at: &str) -> Result<bool>;
}
