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
    /// No `Domain` attribute: the cookie is for exactly `domain` (no leading dot, no subdomains).
    pub host_only: bool,
}

/// What an import does with the cookies already in the profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CookieImportMode {
    /// Replace only the cookies of the domains present in the import; other sessions stay.
    #[default]
    Domains,
    /// Clear every cookie of the profile first.
    All,
}
