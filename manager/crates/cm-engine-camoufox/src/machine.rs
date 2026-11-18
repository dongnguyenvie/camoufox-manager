//! Pinned machine = the Camoufox config generated once (`navigator.*`, `screen.*`, `webGl:*`,
//! `fonts`, …). This module reads it for the UI. Phase 4 owns it (pinning details); CM-1.20 owns the canvas modes.

use cm_core::{
    LaunchOptions, WebRtcMode,
    engine::{MachineSummary, StorageFlags},
};
use serde_json::Value;
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
    a
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
    fn summary() {
        let s = summarize(&json!({"screen.width": 1920, "screen.height": 1080, "navigator.hardwareConcurrency": 8, "fonts": ["a", "b"]}));
        assert_eq!(s.resolution.as_deref(), Some("1920x1080"));
        assert_eq!(s.cpu, Some(8));
        assert_eq!(s.fonts, Some(2));
        assert_eq!(s.language.as_deref(), Some("en-US"));
    }
}
