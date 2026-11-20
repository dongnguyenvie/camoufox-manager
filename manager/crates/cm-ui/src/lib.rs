//! Static UI, embedded into the binary (nothing is loaded from the network at run time).
//!
//! `assets/` layout (D6: Vue 3 global build, Tailwind compiled ahead of time by
//! `manager/ui-build.sh`, no bundler, one file per page/component with its own en/vi messages):
//!
//! - `index.html` — shell, loads the scripts below in order
//! - `core.js` (registry `CM`, `api()`), `i18n.js` (`t()`), `app.js` (root, router, language)
//! - `components/*.js` — shared UI + per-area components (stubs until their phase)
//! - `pages/*.js` — one per sidebar entry
//! - `app.css` (generated from `input.css`), `vendor/vue.global.prod.js`
//!
//! A new asset file must be added to [`ASSETS`] and to `index.html`.

pub struct Asset {
    /// URL path without the leading `/`.
    pub path: &'static str,
    pub content_type: &'static str,
    pub body: &'static str,
}

const JS: &str = "application/javascript; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";

macro_rules! asset {
    ($path:literal, $ct:expr) => {
        Asset { path: $path, content_type: $ct, body: include_str!(concat!("../assets/", $path)) }
    };
}

pub const INDEX_HTML: &str = include_str!("../assets/index.html");

pub const ASSETS: &[Asset] = &[
    asset!("app.css", CSS),
    asset!("vendor/vue.global.prod.js", JS),
    asset!("core.js", JS),
    asset!("i18n.js", JS),
    asset!("app.js", JS),
    // shared components
    asset!("components/icons.js", JS),
    asset!("components/toast.js", JS),
    asset!("components/dialog.js", JS),
    asset!("components/fields.js", JS),
    asset!("components/table.js", JS),
    asset!("components/page.js", JS),
    asset!("components/sidebar.js", JS),
    // per-area components (filled by their phase)
    asset!("components/profile-form.js", JS),
    asset!("components/cookies.js", JS),
    asset!("components/transfer.js", JS),
    asset!("components/org.js", JS),
    asset!("components/sheets.js", JS),
    asset!("components/bell.js", JS),
    asset!("components/doctor.js", JS),
    // pages
    asset!("pages/profiles.js", JS),
    asset!("pages/proxies.js", JS),
    asset!("pages/templates.js", JS),
    asset!("pages/schedules.js", JS),
    asset!("pages/settings.js", JS),
    asset!("pages/devices.js", JS),
];

/// Asset by URL path (without leading `/`).
pub fn get(path: &str) -> Option<&'static Asset> {
    ASSETS.iter().find(|a| a.path == path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_script_in_index_is_an_asset_and_back() {
        for a in ASSETS.iter().filter(|a| a.path.ends_with(".js")) {
            assert!(INDEX_HTML.contains(&format!("src=\"/{}\"", a.path)), "{} not loaded by index.html", a.path);
        }
        for src in INDEX_HTML.split("src=\"/").skip(1).filter_map(|s| s.split('"').next()) {
            assert!(get(src).is_some(), "index.html loads /{src} but it is not in ASSETS");
        }
    }
}
