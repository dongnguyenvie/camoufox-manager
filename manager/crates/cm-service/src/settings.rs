//! Settings: UI language (phase 1). Phase 12 extends this module (engine status/version,
//! install/update Camoufox, headless default, data dir, API key).

use crate::Manager;
use anyhow::{Result, bail};
use cm_core::{
    CmError,
    settings::{LANGUAGE, LANGUAGES},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    /// `en` | `vi`
    pub language: String,
    pub data_dir: String,
    pub db_path: Option<String>,
    pub proxy_base_port: u16,
}

/// Partial update: only present fields change.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub language: Option<String>,
}

impl Manager {
    pub async fn language(&self) -> String {
        match self.d.settings.get(LANGUAGE).await {
            Ok(Some(v)) => v.as_str().filter(|l| LANGUAGES.contains(l)).unwrap_or("en").to_string(),
            _ => "en".into(),
        }
    }

    pub async fn settings(&self) -> SettingsView {
        SettingsView {
            language: self.language().await,
            data_dir: self.cfg.data_dir.display().to_string(),
            db_path: self.cfg.db_path.as_ref().map(|p| p.display().to_string()),
            proxy_base_port: self.cfg.proxy_base_port,
        }
    }

    pub async fn update_settings(&self, patch: SettingsPatch) -> Result<SettingsView> {
        if let Some(lang) = patch.language {
            if !LANGUAGES.contains(&lang.as_str()) {
                bail!(CmError::Invalid(format!("unsupported language {lang}")));
            }
            self.d.settings.set(LANGUAGE, &json!(lang)).await?;
        }
        Ok(self.settings().await)
    }
}
