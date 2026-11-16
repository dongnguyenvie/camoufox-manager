//! Cookies import/export for a closed profile (phase 5).

use serde::{Deserialize, Serialize};

/// File formats accepted / produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CookieFormat {
    /// Browser-extension JSON (EditThisCookie / Cookie-Editor shape).
    Json,
    /// Netscape `cookies.txt`.
    Netscape,
}

/// One cookie, engine-neutral.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    /// Unix seconds; `None` = session cookie.
    pub expires: Option<f64>,
    pub http_only: bool,
    pub secure: bool,
    /// `Strict` | `Lax` | `None`
    pub same_site: Option<String>,
}
