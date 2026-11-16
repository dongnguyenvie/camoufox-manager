//! Pinned machine = the Camoufox config generated once (`navigator.*`, `screen.*`, `webGl:*`,
//! `fonts`, …). This module reads it for the UI. Phase 4 owns it (pinning details, canvas).

use cm_core::engine::{MachineSummary, StorageFlags};
use serde_json::Value;

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
    fn summary() {
        let s = summarize(&json!({"screen.width": 1920, "screen.height": 1080, "navigator.hardwareConcurrency": 8, "fonts": ["a", "b"]}));
        assert_eq!(s.resolution.as_deref(), Some("1920x1080"));
        assert_eq!(s.cpu, Some(8));
        assert_eq!(s.fonts, Some(2));
        assert_eq!(s.language.as_deref(), Some("en-US"));
    }
}
