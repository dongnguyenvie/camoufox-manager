//! Interface between the app core and a browser engine.
//!
//! The engine owns processes and the user-data directory of each profile; profile records
//! live in [`ProfileRepo`](crate::ProfileRepo). Camoufox is the only implementation
//! (`cm-engine-camoufox`). The local proxy, geo and assignment logic stay in `cm-service`.

use crate::{
    Result,
    cookies::{Cookie, CookieFormat, CookieImportMode},
    not_implemented,
    profile::{Os, Profile},
};
use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;

/// Why the engine cannot launch yet (UI translates `code`, shows `detail` as is).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EngineHint {
    /// `not_installed` (no python / package) · `browser_missing` (package ok, browser not fetched) · `error`
    pub code: &'static str,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EngineStatus {
    pub ready: bool,
    pub hint: Option<EngineHint>,
    /// Browser/package version when known (phase 12 fills it).
    pub version: Option<String>,
}

/// Installed versions, when the engine can tell (phase 12).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineVersions {
    /// Python package, e.g. `0.5.9`.
    pub package: Option<String>,
    /// Browser build, e.g. `158.0.1-beta.41`.
    pub browser: Option<String>,
}

/// Where the install lands and how to repeat it by hand (shown when the in-app install fails).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallInfo {
    /// Package manager binary the install uses (`uv`); `None` when it cannot be found.
    pub uv_path: Option<String>,
    /// `<venv>/bin/python` the engine runs from.
    pub venv_python: String,
    /// Pinned package version the install requests.
    pub camoufox_version: String,
}

/// Settings applied for one launch. `None` = let the engine decide.
#[derive(Clone, Debug, Default)]
pub struct LaunchOptions {
    /// Local, auth-less HTTP proxy (always `127.0.0.1:port` opened by the app).
    pub proxy: Option<(String, u16)>,
    pub timezone: Option<String>,
    /// (latitude, longitude). Never set without `timezone` (D4).
    pub geolocation: Option<(f64, f64)>,
    /// Public IP of the proxy, so WebRTC reports it instead of the real one.
    pub public_ip: Option<String>,
    pub headless: Option<bool>,
    /// Locale such as `de-DE` (override, or derived from the proxy country).
    pub locale: Option<String>,
    pub webrtc: crate::profile::WebRtcMode,
    /// `WxH` screen / window overrides on top of the pinned machine.
    pub screen: Option<String>,
    pub window: Option<String>,
    /// Fonts added to the pinned list.
    pub extra_fonts: Vec<String>,
    pub humanize: bool,
    pub start_url: Option<String>,
    /// Keep canvas/WebGL readback identical across launches (own value per profile, CM-1.20).
    pub canvas_stable: bool,
    /// Native host rendering, shared by every profile (CM-1.4 behaviour); wins over `canvas_stable`.
    pub canvas_native: bool,
    /// Firefox extensions (`.xpi` files) to install into the profile before the browser opens.
    /// The engine also removes extensions it installed earlier that are no longer listed.
    pub extensions: Vec<PathBuf>,
    /// What the profile keeps between runs (engine maps it to browser prefs).
    pub data: crate::profile::DataSaving,
    /// Extra browser prefs from the profile, applied last (already validated).
    pub custom_prefs: std::collections::BTreeMap<String, serde_json::Value>,
}

/// A running browser, remote-controllable at `ws_url` (protocol: [`Engine::ws_protocol`]).
#[derive(Clone, Debug)]
pub struct Session {
    pub ws_url: String,
}

/// A browser that died without being asked to stop (shown as status `crashed`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CrashInfo {
    /// Unix seconds.
    pub at: u64,
    pub detail: String,
}

/// Read-only summary of a pinned machine for the UI.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineSummary {
    pub user_agent: Option<String>,
    /// `WxH`
    pub resolution: Option<String>,
    pub language: Option<String>,
    pub platform: Option<String>,
    pub cpu: Option<u32>,
    pub ram: Option<f64>,
    pub dpr: Option<f64>,
    pub webgl_vendor: Option<String>,
    pub webgl_renderer: Option<String>,
    pub noise: Noise,
    pub webrtc: Option<String>,
    pub fonts: Option<usize>,
    pub storage: Option<StorageFlags>,
}

/// Noise mode per API (`noise`, `off`, …), engine-specific names.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Noise {
    pub canvas: Option<String>,
    pub webgl: Option<String>,
    pub audio: Option<String>,
    pub rects: Option<String>,
}

/// What the profile keeps between runs.
#[derive(Clone, Debug, Default, Serialize)]
pub struct StorageFlags {
    pub session: bool,
    pub local: bool,
    pub history: bool,
    pub passwords: bool,
}

#[async_trait]
pub trait Engine: Send + Sync {
    /// Display name, e.g. "Camoufox".
    fn name(&self) -> &'static str;

    /// Protocol spoken at [`Session::ws_url`] (`Playwright`, D5).
    fn ws_protocol(&self) -> &'static str;

    /// Installed and able to launch? Cheap enough to call on every overview.
    async fn status(&self) -> EngineStatus;

    /// Package + browser versions (cached by the engine). `None` when not installed or unknown.
    async fn versions(&self) -> Option<EngineVersions> {
        None
    }

    /// Install or update the engine, reporting every output line to `log` (phase 12). Optional.
    async fn install(&self, _log: &(dyn Fn(String) + Send + Sync)) -> Result<()> {
        Err(not_implemented("engine install"))
    }

    /// Paths and version behind [`Engine::install`], for manual-install instructions.
    fn install_info(&self) -> Option<InstallInfo> {
        None
    }

    /// Generate a new pinned machine for `os` (the opaque value stored in [`Profile::machine`]).
    async fn generate(&self, os: Os) -> Result<Value>;

    /// Read-only view of a pinned machine.
    fn summarize(&self, machine: &Value) -> MachineSummary;

    /// Launch the profile's browser with `opts`.
    async fn launch(&self, profile: &Profile, opts: &LaunchOptions) -> Result<Session>;

    /// Close the profile's browser gracefully (no-op if not running).
    async fn stop(&self, id: &str) -> Result<()>;

    /// `Some(false)` = engine knows it is gone (window closed, crash); `None` = unknown.
    async fn alive(&self, _id: &str) -> Option<bool> {
        None
    }

    /// The browser died on its own and has not been started again since (phase 8).
    async fn crashed(&self, _id: &str) -> Option<CrashInfo> {
        None
    }

    /// Closed-profile proof (phase 15): fail with `CmError::Conflict` unless the profile is
    /// provably closed (not running, lock free, no live browser markers); otherwise return a
    /// guard that keeps the profile from starting until it is dropped. Engines without a lock
    /// return a unit guard.
    async fn hold_closed(&self, _id: &str) -> Result<Box<dyn std::any::Any + Send>> {
        Ok(Box::new(()))
    }

    /// Directory with the profile's browser data (may not exist yet).
    fn data_dir(&self, id: &str) -> PathBuf;

    /// Remove all on-disk data of a profile (must not be running).
    async fn delete_data(&self, id: &str) -> Result<()>;

    /// Cookies of a closed profile (phase 5).
    async fn export_cookies(&self, _profile: &Profile, _format: CookieFormat) -> Result<Vec<Cookie>> {
        Err(not_implemented("cookies"))
    }

    /// Write cookies into a closed profile (phase 5). `mode` decides what happens to the
    /// existing ones; with `Domains` the cookies of every host present in `cookies` are replaced.
    /// Returns how many were written.
    async fn import_cookies(&self, _profile: &Profile, _cookies: &[Cookie], _mode: CookieImportMode) -> Result<usize> {
        Err(not_implemented("cookies"))
    }

    /// Browse like a person on a running profile (scheduler warming): open each of `urls`, stay
    /// `dwell_secs` on it with light scrolling. Does not close the browser.
    async fn browse(&self, _ws_url: &str, _urls: &[String], _dwell_secs: u32) -> Result<()> {
        Err(not_implemented("browse"))
    }
}
