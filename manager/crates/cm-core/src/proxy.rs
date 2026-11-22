//! Proxy library: named upstream proxies, assigned to profiles, checked on demand.
//!
//! Passwords never leave the process towards the UI: [`Proxy::password`] is skipped when
//! serializing, and labels never contain credentials.

use crate::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub type ProxyId = i64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyScheme {
    #[default]
    Http,
    Https,
    Socks4,
    Socks4a,
    Socks5,
    Socks5h,
}

impl ProxyScheme {
    pub const ALL: [ProxyScheme; 6] =
        [ProxyScheme::Http, ProxyScheme::Https, ProxyScheme::Socks4, ProxyScheme::Socks4a, ProxyScheme::Socks5, ProxyScheme::Socks5h];

    pub fn as_str(self) -> &'static str {
        match self {
            ProxyScheme::Http => "http",
            ProxyScheme::Https => "https",
            ProxyScheme::Socks4 => "socks4",
            ProxyScheme::Socks4a => "socks4a",
            ProxyScheme::Socks5 => "socks5",
            ProxyScheme::Socks5h => "socks5h",
        }
    }

    pub fn parse(s: &str) -> Option<ProxyScheme> {
        ProxyScheme::ALL.into_iter().find(|x| x.as_str().eq_ignore_ascii_case(s))
    }
}

/// Result of checking a proxy through itself (exit IP + where it lands).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyCheck {
    pub ip: String,
    pub country: String,
    pub country_code: String,
    pub city: String,
    /// IANA timezone of the exit IP.
    pub timezone: String,
    pub lat: f64,
    pub lon: f64,
    /// Unix seconds.
    pub checked_at: u64,
    #[serde(default)]
    pub latency_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proxy {
    pub id: ProxyId,
    pub name: String,
    pub scheme: ProxyScheme,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    /// Never serialized (UI/API must not see it).
    #[serde(skip_serializing, default)]
    pub password: Option<String>,
    pub created_at: String,
    pub last_check: Option<ProxyCheck>,
    /// "Change IP" link of rotating providers (http/https GET). May hold a token: never log it.
    #[serde(default)]
    pub rotate_url: Option<String>,
    /// Call `rotate_url` before every profile start.
    #[serde(default)]
    pub rotate_on_start: bool,
}

impl Proxy {
    /// `host:port`, IPv6 in brackets. Safe to log.
    pub fn label(&self) -> String {
        if self.host.contains(':') { format!("[{}]:{}", self.host, self.port) } else { format!("{}:{}", self.host, self.port) }
    }

    pub fn has_password(&self) -> bool {
        self.password.as_deref().is_some_and(|p| !p.is_empty())
    }
}

/// Data for a new library entry.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewProxy {
    pub name: String,
    pub scheme: ProxyScheme,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(default)]
    pub rotate_url: Option<String>,
    #[serde(default)]
    pub rotate_on_start: bool,
}

#[async_trait]
pub trait ProxyRepo: Send + Sync {
    /// All proxies, by id.
    async fn list(&self) -> Result<Vec<Proxy>>;
    async fn get(&self, id: ProxyId) -> Result<Option<Proxy>>;
    async fn insert(&self, p: &NewProxy) -> Result<Proxy>;
    /// Replace name/scheme/host/port/username/rotate settings; `password: None` keeps the stored one.
    async fn update(&self, id: ProxyId, p: &NewProxy) -> Result<()>;
    /// Delete and unassign from every profile.
    async fn delete(&self, id: ProxyId) -> Result<()>;
    async fn set_check(&self, id: ProxyId, check: &ProxyCheck) -> Result<()>;
}

/// Checks a proxy end to end: reachability, exit IP, geo, latency.
#[async_trait]
pub trait ProxyChecker: Send + Sync {
    async fn check(&self, proxy: &Proxy) -> Result<ProxyCheck>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_never_serialized() {
        let p = Proxy {
            id: 1,
            name: "a".into(),
            scheme: ProxyScheme::Http,
            host: "1.2.3.4".into(),
            port: 8080,
            username: Some("u".into()),
            password: Some("secret".into()),
            created_at: String::new(),
            last_check: None,
            rotate_url: None,
            rotate_on_start: false,
        };
        let j = serde_json::to_string(&p).unwrap();
        assert!(!j.contains("secret"));
        assert_eq!(p.label(), "1.2.3.4:8080");
    }
}
