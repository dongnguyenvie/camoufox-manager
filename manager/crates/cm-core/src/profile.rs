//! Profiles: one browser identity = pinned machine (fingerprint, frozen at creation, D4) +
//! editable settings + user data on disk (owned by the engine).
//!
//! Phase 4 fills [`ProfileSettings`] and the profile form; phase 7 adds tags/groups through
//! separate tables (see `org`), so the profile record itself stays small.

use crate::{ProfileId, Result, proxy::ProxyId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Fingerprint OS. Serialized as the short code used everywhere (`win`, `mac`, `lin`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Win,
    Mac,
    Lin,
}

impl Os {
    pub const ALL: [Os; 3] = [Os::Win, Os::Mac, Os::Lin];

    pub fn code(self) -> &'static str {
        match self {
            Os::Win => "win",
            Os::Mac => "mac",
            Os::Lin => "lin",
        }
    }

    pub fn parse(s: &str) -> Option<Os> {
        Os::ALL.into_iter().find(|o| o.code() == s)
    }
}

/// How WebRTC exposes IPs (phase 4 wires it into launch).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebRtcMode {
    /// Report the proxy exit IP (default, matches the old behaviour).
    #[default]
    Replace,
    /// Report the real IP.
    Real,
    /// Let traffic through untouched.
    Forward,
    /// Disable WebRTC.
    None,
}

/// How canvas / WebGL readback noise behaves across launches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CanvasMode {
    /// This profile's own canvas / WebGL / audio fingerprint, the same on every launch (default,
    /// CM-1.20). Existing profiles saved as `stable` get it without migration.
    #[default]
    Stable,
    /// Native rendering of this machine, identical on every launch AND on every profile (the
    /// CM-1.4 behaviour of `stable`; links accounts on one machine).
    Native,
    /// Camoufox's own random noise, different on every launch.
    Random,
}

/// What the profile keeps between runs. `true` = keep (the Firefox default); `false` = do not save
/// it, or clear it when the browser closes. Cookies are never touched.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DataSaving {
    /// Offer to remember passwords.
    pub passwords: bool,
    /// Record browsing history (off = also cleared on shutdown).
    pub history: bool,
    /// Disk cache (off = no disk cache, and cleared on shutdown).
    pub cache: bool,
    /// Site storage (localStorage, IndexedDB, ...); off = cleared on shutdown, cookies stay.
    pub site_storage: bool,
}

impl Default for DataSaving {
    fn default() -> Self {
        DataSaving { passwords: true, history: true, cache: true, site_storage: true }
    }
}

/// User-editable settings. Every field is an override: `None` = follow the machine / proxy.
/// Stored as JSON, so new fields are additive and old rows read with defaults.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ProfileSettings {
    /// Locale override (e.g. `de-DE`); otherwise derived from the proxy (D4).
    pub locale: Option<String>,
    /// IANA timezone override; otherwise the proxy exit timezone (D4).
    pub timezone: Option<String>,
    pub webrtc: WebRtcMode,
    /// Window size `WxH`; `None` = engine default for the pinned screen.
    pub window: Option<String>,
    /// Screen size override `WxH` on top of the pinned machine; `None` = pinned screen.
    pub screen: Option<String>,
    /// Extra fonts added to the pinned font list at launch.
    pub fonts: Vec<String>,
    pub canvas: CanvasMode,
    /// Page opened on start.
    pub start_url: Option<String>,
    pub humanize: Option<bool>,
    /// What the browser keeps between launches (CM-3.3). Default: keep everything.
    pub data: DataSaving,
    /// Extra Firefox prefs set at every launch (CM-3.3), applied after ours; values are
    /// bool / number / string. Keys the engine owns (proxy, fingerprint) are rejected on save.
    pub custom_prefs: BTreeMap<String, Value>,
    /// Bookmarks kept in a "Camoufox Manager" folder on the toolbar (CM-3.4), written while the
    /// profile is closed.
    pub bookmarks: Vec<Bookmark>,
    /// Block pages from reaching localhost / private networks (port scans) (CM-3.6).
    /// `None` = on.
    pub block_local_network: Option<bool>,
}

/// One managed bookmark (CM-3.4).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Bookmark {
    pub title: String,
    pub url: String,
    /// Optional sub-folder inside the managed folder.
    pub folder: Option<String>,
}

/// The starter bookmarks a new profile gets (copied from GoLogin's defaults).
pub fn default_bookmarks() -> Vec<Bookmark> {
    const ROWS: &[(Option<&str>, &str, &str)] = &[
        (Some("Facebook"), "Facebook", "https://www.facebook.com/"),
        (Some("Facebook"), "Notifications", "https://www.facebook.com/settings?tab=notifications"),
        (Some("Facebook"), "Account Quality", "https://www.facebook.com/accountquality"),
        (Some("Facebook"), "Ads Manager", "https://www.facebook.com/adsmanager/manage/campaigns"),
        (Some("Facebook"), "Billing", "https://www.facebook.com/ads/manager/account_settings/account_billing"),
        (Some("Facebook"), "Business Manager", "https://business.facebook.com/select"),
        (Some("Dropshipping"), "Amazon", "https://www.amazon.com/"),
        (Some("Dropshipping"), "eBay", "https://ebay.com/"),
        (Some("Dropshipping"), "Walmart", "https://walmart.com/"),
        (Some("Dropshipping"), "Shopify", "https://www.shopify.com/"),
        (Some("Dropshipping"), "Taobao", "https://taobao.com/"),
        (Some("Dropshipping"), "Aliexpress", "https://aliexpress.com/"),
        (Some("Dropshipping"), "PayPal", "https://paypal.com/"),
        (Some("Dropshipping"), "Payoneer", "https://payoneer.com/"),
        (Some("Crypto"), "Coinlist", "https://coinlist.co/"),
        (Some("Crypto"), "Huobi", "https://www.huobi.com/"),
        (Some("Crypto"), "Discord", "https://discord.com/"),
        (Some("Crypto"), "Premint", "https://www.premint.xyz/"),
        (None, "Facebook", "https://www.facebook.com/"),
        (None, "Amazon", "https://www.amazon.com/"),
        (None, "TikTok", "https://www.tiktok.com/"),
        (None, "Google Ads", "https://ads.google.com/"),
        (None, "Browser check", "https://iphey.com/"),
    ];
    ROWS.iter().map(|(f, t, u)| Bookmark { title: (*t).into(), url: (*u).into(), folder: f.map(String::from) }).collect()
}

/// A stored profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: ProfileId,
    pub name: String,
    pub os: Os,
    pub notes: Option<String>,
    /// ISO 8601 UTC.
    pub created_at: String,
    pub updated_at: String,
    /// Last start (ISO 8601), `None` if never started.
    pub last_activity: Option<String>,
    /// Proxy from the library assigned to this profile.
    pub proxy_id: Option<ProxyId>,
    /// Pinned machine: the engine's frozen fingerprint config (opaque to everything but the engine).
    pub machine: Value,
    pub settings: ProfileSettings,
}

/// Request to create a profile.
#[derive(Clone, Debug, Deserialize)]
pub struct NewProfile {
    pub name: String,
    /// `win` | `mac` | `lin`
    pub os: String,
}

/// A profile in the trash (CM-3.1) and when it was put there.
#[derive(Clone, Debug, PartialEq)]
pub struct TrashedProfile {
    pub profile: Profile,
    /// ISO 8601 UTC.
    pub deleted_at: String,
}

#[async_trait]
pub trait ProfileRepo: Send + Sync {
    /// All profiles, oldest first. Trashed profiles (CM-3.1) are hidden here and in `get`.
    async fn list(&self) -> Result<Vec<Profile>>;
    async fn get(&self, id: &str) -> Result<Option<Profile>>;
    async fn insert(&self, p: &Profile) -> Result<()>;
    /// Replace the whole record (by id). `CmError::NotFound` if missing.
    async fn update(&self, p: &Profile) -> Result<()>;
    /// Remove the record for good (trashed or not).
    async fn delete(&self, id: &str) -> Result<()>;
    /// Move to the trash at `at` (ISO 8601). `NotFound` if missing or already trashed.
    async fn trash(&self, id: &str, at: &str) -> Result<()>;
    /// Trashed profiles, most recently trashed first.
    async fn list_trashed(&self) -> Result<Vec<TrashedProfile>>;
    /// Take a profile out of the trash. `NotFound` if it is not trashed.
    async fn restore(&self, id: &str) -> Result<()>;
    /// Remove a trashed record for good. `NotFound` if it is not trashed.
    async fn purge(&self, id: &str) -> Result<()>;
    /// Assign (`Some`) or clear (`None`) the profile's proxy.
    async fn set_proxy(&self, id: &str, proxy: Option<ProxyId>) -> Result<()>;
    /// Record a start at `at` (ISO 8601).
    async fn touch(&self, id: &str, at: &str) -> Result<()>;
}

/// Only ids the app generated (hex) are accepted — guards file paths built from ids.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bookmarks_count() {
        let d = default_bookmarks();
        assert_eq!(d.len(), 23);
        assert_eq!(d.iter().filter(|b| b.folder.is_none()).count(), 5);
    }

    #[test]
    fn os_codes() {
        assert_eq!(Os::parse("mac"), Some(Os::Mac));
        assert_eq!(Os::parse("android"), None);
        assert_eq!(serde_json::to_string(&Os::Lin).unwrap(), "\"lin\"");
    }

    #[test]
    fn ids() {
        assert!(valid_id("abcdef0123456789abcdef01"));
        assert!(!valid_id("../x"));
        assert!(!valid_id(""));
    }

    #[test]
    fn settings_default_on_missing_fields() {
        let s: ProfileSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(s, ProfileSettings::default());
    }
}
