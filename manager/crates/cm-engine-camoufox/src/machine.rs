//! Pinned machine = the Camoufox config generated once (`navigator.*`, `screen.*`, `webGl:*`,
//! `fonts`, …). This module reads it for the UI. Phase 4 owns it (pinning details); CM-1.20 owns the canvas modes.

use cm_core::{
    LaunchOptions, WebRtcMode,
    engine::{MachineSummary, StorageFlags},
};
use serde_json::Value;
use std::collections::BTreeMap;
use tokio::process::Command;

/// `helper launch` flags for the profile's setting overrides (the pinned machine itself travels in
/// `launch.json`). Values are passed as separate args, never through a shell.
pub fn override_args(o: &LaunchOptions) -> Vec<String> {
    let mut a = Vec::new();
    let mut flag = |k: &str, v: &str| {
        a.push(format!("--{k}"));
        a.push(v.to_string());
    };
    if let Some(l) = &o.locale {
        flag("locale", l);
    }
    flag(
        "webrtc",
        match o.webrtc {
            WebRtcMode::Replace => "replace",
            WebRtcMode::Real => "real",
            WebRtcMode::Forward => "forward",
            WebRtcMode::None => "none",
        },
    );
    if let Some(s) = &o.screen {
        flag("screen", s);
    }
    if let Some(w) = &o.window {
        flag("window", w);
    }
    if !o.extra_fonts.is_empty() {
        flag("fonts", &serde_json::to_string(&o.extra_fonts).unwrap_or_else(|_| "[]".into()));
    }
    flag(
        "canvas",
        if o.canvas_native {
            "native"
        } else if o.canvas_stable {
            "unique"
        } else {
            "random"
        },
    );
    if let Some(u) = &o.start_url {
        // `--start-url=…`: a value starting with `-` must not be read as an option.
        a.push(format!("--start-url={u}"));
    }
    if o.humanize {
        a.push("--humanize".into());
    }
    let prefs = pref_overrides(o);
    if !prefs.is_empty() {
        a.push("--prefs".into());
        a.push(serde_json::to_string(&prefs).unwrap_or_else(|_| "{}".into()));
    }
    a
}

/// Firefox prefs for the data-saving toggles, then the profile's custom prefs on top (they win).
///
/// Camoufox 0.5.9 ships a recent Firefox (158 here), which has both the legacy
/// `privacy.clearOnShutdown.*` and the v2 `privacy.clearOnShutdown_v2.*` prefs (v2 replaced the
/// legacy ones in Firefox 128). The v2 set has no separate site-storage switch (it is bundled
/// with cookies, which are never cleared), so site storage is cleared through the legacy
/// `offlineApps` pref, and `privacy.sanitize.useOldClearHistoryDialog` makes Firefox read the
/// legacy set. Both are written so the result does not depend on which one the build reads.
pub fn pref_overrides(o: &LaunchOptions) -> BTreeMap<String, Value> {
    let mut p: BTreeMap<String, Value> = BTreeMap::new();
    let mut set = |k: &str, v: bool| {
        p.insert(k.to_string(), Value::Bool(v));
    };
    let d = &o.data;
    if !d.passwords {
        set("signon.rememberSignons", false);
        set("signon.autofillForms", false);
    }
    if !d.history {
        set("places.history.enabled", false);
        set("privacy.clearOnShutdown.history", true);
        set("privacy.clearOnShutdown.downloads", true);
        set("privacy.clearOnShutdown_v2.historyFormDataAndDownloads", true);
    }
    if !d.cache {
        set("browser.cache.disk.enable", false);
        set("privacy.clearOnShutdown.cache", true);
        set("privacy.clearOnShutdown_v2.cache", true);
    }
    if !d.site_storage {
        set("privacy.clearOnShutdown.offlineApps", true);
    }
    if !(d.history && d.cache && d.site_storage) {
        set("privacy.sanitize.sanitizeOnShutdown", true);
        set("privacy.sanitize.useOldClearHistoryDialog", true);
        // Never cookies or open sessions, whatever the profile held before.
        set("privacy.clearOnShutdown.cookies", false);
        set("privacy.clearOnShutdown.sessions", false);
        set("privacy.clearOnShutdown_v2.cookiesAndStorage", false);
    }
    p.extend(o.custom_prefs.clone());
    p
}

pub fn launch_args(o: &LaunchOptions, cmd: &mut Command) {
    cmd.args(override_args(o));
}

fn str_of(c: &Value, k: &str) -> Option<String> {
    c.get(k).and_then(Value::as_str).filter(|s| !s.is_empty()).map(String::from)
}

pub fn summarize(c: &Value) -> MachineSummary {
    let num = |k: &str| c.get(k).and_then(Value::as_f64);
    let resolution = match (num("screen.width"), num("screen.height")) {
        (Some(w), Some(h)) => Some(format!("{w}x{h}")),
        _ => None,
    };
    // Camoufox defaults to en-US when the config sets no language.
    let language = str_of(c, "navigator.language")
        .or_else(|| str_of(c, "locale:language").map(|l| str_of(c, "locale:region").map_or(l.clone(), |r| format!("{l}-{r}"))))
        .or(Some("en-US".into()));
    MachineSummary {
        user_agent: str_of(c, "navigator.userAgent"),
        resolution,
        language,
        platform: str_of(c, "navigator.platform"),
        cpu: num("navigator.hardwareConcurrency").map(|n| n as u32),
        ram: None, // Firefox has no navigator.deviceMemory
        // Missing = Camoufox keeps the screen's real DPR.
        dpr: num("window.devicePixelRatio"),
        webgl_vendor: str_of(c, "webGl:vendor"),
        webgl_renderer: str_of(c, "webGl:renderer"),
        noise: Default::default(),
        webrtc: None,
        fonts: c.get("fonts").and_then(Value::as_array).map(Vec::len),
        // A persistent Firefox profile keeps everything in user-data.
        storage: Some(StorageFlags { session: true, local: true, history: true, passwords: true }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn override_args_cover_settings() {
        let o = LaunchOptions {
            locale: Some("de-DE".into()),
            webrtc: WebRtcMode::None,
            window: Some("1200x800".into()),
            extra_fonts: vec!["Foo".into()],
            humanize: true,
            start_url: Some("https://example.com".into()),
            canvas_stable: true,
            ..Default::default()
        };
        let a = override_args(&o);
        let has = |x: &str| a.iter().any(|v| v == x);
        assert!(has("--locale") && has("de-DE") && has("none") && has("1200x800") && has("--humanize"));
        assert!(has("--start-url=https://example.com") && has("unique") && has("[\"Foo\"]"));
        let native = override_args(&LaunchOptions { canvas_stable: true, canvas_native: true, ..Default::default() });
        assert!(native.iter().any(|v| v == "native") && !native.iter().any(|v| v == "unique"));
        let random = override_args(&LaunchOptions::default());
        assert!(random.iter().any(|v| v == "random"));
        assert!(!has("--screen"));
    }

    #[test]
    fn data_saving_prefs() {
        // defaults change nothing and add no argument
        assert!(pref_overrides(&LaunchOptions::default()).is_empty());
        assert!(!override_args(&LaunchOptions::default()).iter().any(|v| v == "--prefs"));
        let mut o = LaunchOptions::default();
        o.data.passwords = false;
        o.data.cache = false;
        o.data.site_storage = false;
        o.custom_prefs.insert("browser.startup.homepage".into(), json!("about:blank"));
        o.custom_prefs.insert("privacy.clearOnShutdown.sessions".into(), json!(true));
        let p = pref_overrides(&o);
        assert_eq!(p["signon.rememberSignons"], json!(false));
        assert_eq!(p["browser.cache.disk.enable"], json!(false));
        assert_eq!(p["privacy.clearOnShutdown_v2.cache"], json!(true));
        assert_eq!(p["privacy.clearOnShutdown.offlineApps"], json!(true));
        assert_eq!(p["privacy.sanitize.sanitizeOnShutdown"], json!(true));
        assert_eq!(p["privacy.clearOnShutdown.cookies"], json!(false), "cookies are never cleared");
        assert!(!p.contains_key("places.history.enabled"), "history stays on");
        assert_eq!(p["browser.startup.homepage"], json!("about:blank"));
        assert_eq!(p["privacy.clearOnShutdown.sessions"], json!(true), "custom prefs win");
        let a = override_args(&o);
        let i = a.iter().position(|v| v == "--prefs").unwrap();
        let parsed: Value = serde_json::from_str(&a[i + 1]).unwrap();
        assert_eq!(parsed["browser.startup.homepage"], json!("about:blank"));
    }

    #[test]
    fn summary() {
        let s = summarize(&json!({"screen.width": 1920, "screen.height": 1080, "navigator.hardwareConcurrency": 8, "fonts": ["a", "b"]}));
        assert_eq!(s.resolution.as_deref(), Some("1920x1080"));
        assert_eq!(s.cpu, Some(8));
        assert_eq!(s.fonts, Some(2));
        assert_eq!(s.language.as_deref(), Some("en-US"));
    }
}
