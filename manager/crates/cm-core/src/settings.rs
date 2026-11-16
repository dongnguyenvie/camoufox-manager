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
/// Optional API key for non-UI clients (phase 12).
pub const API_KEY: &str = "api.key";

pub const LANGUAGES: [&str; 2] = ["en", "vi"];

#[async_trait]
pub trait SettingsRepo: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Value>>;
    async fn set(&self, key: &str, value: &Value) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
}
