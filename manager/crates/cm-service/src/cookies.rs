//! Cookies import/export (phase 5) for a CLOSED profile.
//!
//! Formats: browser-extension JSON (EditThisCookie / Cookie-Editor: `domain`, `name`, `value`,
//! `path`, `expirationDate`, `secure`, `httpOnly`, `sameSite`, `hostOnly`) and Netscape
//! `cookies.txt` (`#HttpOnly_` prefix for HttpOnly). Parsing, validation and rendering live here;
//! the engine only reads/writes the cookie store (`Engine::{export,import}_cookies`).
//!
//! Import: cookies are validated first (bad ones are skipped and counted by reason), then written
//! with [`CookieImportMode::Domains`] (replace only the hosts present in the file) or
//! [`CookieImportMode::All`] (clear everything first).

use crate::Manager;
use anyhow::Result;
use cm_core::{
    CmError,
    cookies::{Cookie, CookieFormat, CookieImportMode},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

/// A session cookie (no expiry) would vanish when the browser closes, so an imported one is
/// kept for this long.
const SESSION_TTL_SECS: u64 = 30 * 86_400;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CookieImport {
    /// File content (JSON or Netscape text).
    pub content: String,
    /// `None` = detect from the content.
    pub format: Option<CookieFormat>,
    /// `domains` (default) or `all`.
    pub mode: CookieImportMode,
}

/// Why cookies were not imported: `code` is translated by the UI.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Skipped {
    /// `badLine` · `missingName` · `badName` · `badDomain` · `badPath` · `expired` · `duplicate`
    pub code: &'static str,
    pub count: usize,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub format: CookieFormat,
    pub mode: CookieImportMode,
    /// Entries found in the file.
    pub total: usize,
    pub imported: usize,
    pub skipped: usize,
    pub skipped_reasons: Vec<Skipped>,
    /// Session cookies given a 30-day expiry so they survive a restart.
    pub session_extended: usize,
    /// Distinct hosts of the imported cookies (those whose old cookies were replaced).
    pub domains: Vec<String>,
}

pub struct CookieExport {
    pub format: CookieFormat,
    pub count: usize,
    pub body: String,
}

fn invalid(msg: impl Into<String>) -> anyhow::Error {
    CmError::Invalid(msg.into()).into()
}

/// Detect the format: JSON starts with `[` or `{`, everything else is Netscape text.
pub fn detect_format(content: &str) -> CookieFormat {
    match content.trim_start().chars().next() {
        Some('[' | '{') => CookieFormat::Json,
        _ => CookieFormat::Netscape,
    }
}

#[derive(Default)]
struct Tally(BTreeMap<&'static str, usize>);

impl Tally {
    fn add(&mut self, code: &'static str) {
        *self.0.entry(code).or_default() += 1;
    }
}

// ---------------------------------------------------------------- parse

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn boolean(v: Option<&Value>) -> bool {
    match v {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|x| x != 0.0),
        Some(Value::String(s)) => matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes"),
        _ => false,
    }
}

/// Seconds since the epoch; accepts milliseconds too (some exporters write them).
fn to_secs(x: f64) -> f64 {
    if x > 100_000_000_000.0 { (x / 1000.0).floor() } else { x }
}

fn same_site(v: Option<&Value>) -> Option<String> {
    match v.and_then(|v| v.as_str()).map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("strict") => Some("Strict".into()),
        Some("lax") => Some("Lax".into()),
        Some("none" | "no_restriction") => Some("None".into()),
        _ => None, // unspecified
    }
}

/// Cookie from one JSON object, as raw as it came (validated by [`normalise`]).
fn from_json(o: &Map<String, Value>) -> Cookie {
    let text = |k: &str| match o.get(k) {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        Some(Value::Bool(b)) => Some(b.to_string()),
        _ => None,
    };
    let domain = text("domain").or_else(|| text("host")).unwrap_or_default();
    let session = boolean(o.get("session"));
    let expires =
        if session { None } else { ["expirationDate", "expires", "expiry", "expiration"].iter().find_map(|k| o.get(*k).and_then(num)) };
    Cookie {
        name: text("name").unwrap_or_default(),
        value: text("value").unwrap_or_default(),
        host_only: match o.get("hostOnly") {
            Some(v) => boolean(Some(v)),
            None => !domain.starts_with('.'),
        },
        domain,
        path: text("path").unwrap_or_else(|| "/".into()),
        expires: expires.filter(|x| *x > 0.0).map(to_secs),
        http_only: boolean(o.get("httpOnly")),
        secure: boolean(o.get("secure")),
        same_site: same_site(o.get("sameSite")),
    }
}

/// `(cookies, entries that could not even be read)`.
fn parse_json(content: &str) -> Result<(Vec<Cookie>, Vec<&'static str>)> {
    let v: Value = serde_json::from_str(content).map_err(|e| invalid(format!("not valid JSON: {e}")))?;
    let list = match &v {
        Value::Array(a) => a,
        Value::Object(o) => match o.get("cookies") {
            Some(Value::Array(a)) => a,
            _ => return Err(invalid("JSON must be an array of cookies (or an object with a \"cookies\" array)")),
        },
        _ => return Err(invalid("JSON must be an array of cookies")),
    };
    let mut bad = Vec::new();
    let mut out = Vec::new();
    for item in list {
        match item.as_object() {
            Some(o) => out.push(from_json(o)),
            None => bad.push("badLine"),
        }
    }
    Ok((out, bad))
}

fn parse_netscape(content: &str) -> (Vec<Cookie>, Vec<&'static str>) {
    let mut bad = Vec::new();
    let mut out = Vec::new();
    for raw in content.lines() {
        let mut line = raw.trim_end_matches(['\r', '\n']);
        let mut http_only = false;
        if let Some(rest) = line.strip_prefix("#HttpOnly_") {
            http_only = true;
            line = rest;
        } else if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != 7 {
            bad.push("badLine");
            continue;
        }
        let expiry = f[4].trim().parse::<f64>().ok();
        let Some(expiry) = expiry else {
            bad.push("badLine");
            continue;
        };
        out.push(Cookie {
            domain: f[0].trim().to_string(),
            host_only: !f[1].trim().eq_ignore_ascii_case("true"),
            path: f[2].trim().to_string(),
            secure: f[3].trim().eq_ignore_ascii_case("true"),
            expires: (expiry > 0.0).then(|| to_secs(expiry)),
            name: f[5].to_string(),
            value: f[6].to_string(),
            http_only,
            same_site: None,
        });
    }
    (out, bad)
}

// ------------------------------------------------------------- validate

fn valid_host(h: &str) -> bool {
    !h.is_empty() && h.len() <= 253 && h.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':' | '[' | ']'))
}

fn is_ip_or_single_label(h: &str) -> bool {
    !h.contains('.') || h.contains(':') || h.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// Canonical form for the store: lower-case host; a leading dot only for domain cookies.
/// `Err(code)` when the cookie cannot be imported.
fn normalise(mut c: Cookie, now: u64, session_extended: &mut usize) -> std::result::Result<Cookie, &'static str> {
    if c.name.is_empty() {
        return Err("missingName");
    }
    let ctl = |s: &str| s.chars().any(|ch| ch.is_control());
    if ctl(&c.name) || ctl(&c.value) || c.name.contains(['=', ';']) {
        return Err("badName");
    }
    let host = c.domain.trim().trim_start_matches('.').to_ascii_lowercase();
    if !valid_host(&host) {
        return Err("badDomain");
    }
    if is_ip_or_single_label(&host) {
        c.host_only = true;
    }
    c.domain = if c.host_only { host } else { format!(".{host}") };
    c.path = c.path.trim().to_string();
    if c.path.is_empty() {
        c.path = "/".into();
    }
    if !c.path.starts_with('/') || ctl(&c.path) || c.path.contains(';') {
        return Err("badPath");
    }
    match c.expires {
        Some(x) if x <= now as f64 => return Err("expired"),
        None => {
            c.expires = Some((now + SESSION_TTL_SECS) as f64);
            *session_extended += 1;
        }
        _ => {}
    }
    Ok(c)
}

// --------------------------------------------------------------- render

fn to_json(cookies: &[Cookie]) -> String {
    let list: Vec<Value> = cookies
        .iter()
        .map(|c| {
            let mut o = json!({
                "domain": c.domain,
                "hostOnly": c.host_only,
                "httpOnly": c.http_only,
                "name": c.name,
                "path": c.path,
                "sameSite": match c.same_site.as_deref() {
                    Some("Strict") => "strict",
                    Some("Lax") => "lax",
                    Some("None") => "no_restriction",
                    _ => "unspecified",
                },
                "secure": c.secure,
                "session": c.expires.is_none(),
                "storeId": "0",
                "value": c.value,
            });
            if let Some(e) = c.expires {
                o["expirationDate"] = json!(e);
            }
            o
        })
        .collect();
    serde_json::to_string_pretty(&list).unwrap_or_else(|_| "[]".into())
}

fn to_netscape(cookies: &[Cookie]) -> String {
    let mut s = String::from(
        "# Netscape HTTP Cookie File\n# https://curl.se/docs/http-cookies.html\n# This file was generated by Camoufox Manager. Edit at your own risk.\n\n",
    );
    for c in cookies {
        let pre = if c.http_only { "#HttpOnly_" } else { "" };
        let tf = |b: bool| if b { "TRUE" } else { "FALSE" };
        let domain =
            if c.host_only { c.domain.trim_start_matches('.').to_string() } else { format!(".{}", c.domain.trim_start_matches('.')) };
        s += &format!(
            "{pre}{domain}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            tf(!c.host_only),
            c.path,
            tf(c.secure),
            c.expires.map_or(0, |e| e as u64),
            c.name,
            c.value
        );
    }
    s
}

pub fn render(cookies: &[Cookie], format: CookieFormat) -> String {
    match format {
        CookieFormat::Json => to_json(cookies),
        CookieFormat::Netscape => to_netscape(cookies),
    }
}

/// Parse + validate file content. Returns the cookies to write and the report skeleton
/// (`imported` is filled in after the engine wrote them).
pub fn prepare(req: &CookieImport, now: u64) -> Result<(Vec<Cookie>, ImportReport)> {
    if req.content.trim().is_empty() {
        return Err(invalid("the cookie file is empty"));
    }
    let format = req.format.unwrap_or_else(|| detect_format(&req.content));
    let (raw, unreadable) = match format {
        CookieFormat::Json => parse_json(&req.content)?,
        CookieFormat::Netscape => parse_netscape(&req.content),
    };
    let mut tally = Tally::default();
    for code in &unreadable {
        tally.add(code);
    }
    let total = raw.len() + unreadable.len();
    let mut session_extended = 0;
    // Same (name, domain, path) twice: the later entry wins (like the browser would).
    let mut by_key: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    let mut good: Vec<Cookie> = Vec::new();
    for c in raw {
        match normalise(c, now, &mut session_extended) {
            Ok(c) => {
                let key = (c.name.clone(), c.domain.clone(), c.path.clone());
                if let Some(&i) = by_key.get(&key) {
                    good[i] = c;
                    tally.add("duplicate");
                } else {
                    by_key.insert(key, good.len());
                    good.push(c);
                }
            }
            Err(code) => tally.add(code),
        }
    }
    let skipped_reasons: Vec<Skipped> = tally.0.into_iter().map(|(code, count)| Skipped { code, count }).collect();
    let skipped = skipped_reasons.iter().map(|r| r.count).sum();
    let mut domains: Vec<String> = good.iter().map(|c| c.domain.trim_start_matches('.').to_string()).collect();
    domains.sort();
    domains.dedup();
    let report = ImportReport { format, mode: req.mode, total, imported: 0, skipped, skipped_reasons, session_extended, domains };
    Ok((good, report))
}

impl Manager {
    /// The profile must exist and its browser must not be running (the cookie store is locked).
    async fn require_closed(&self, id: &str) -> Result<cm_core::Profile> {
        let p = self.profile(id).await?;
        if self.running.lock().await.contains_key(id) || self.d.engine.alive(id).await == Some(true) {
            return Err(CmError::Conflict("stop the profile before importing or exporting its cookies".into()).into());
        }
        Ok(p)
    }

    pub async fn export_cookies(&self, id: &str, format: CookieFormat) -> Result<CookieExport> {
        let p = self.require_closed(id).await?;
        let cookies = self.d.engine.export_cookies(&p, format).await?;
        Ok(CookieExport { format, count: cookies.len(), body: render(&cookies, format) })
    }

    pub async fn import_cookies(&self, id: &str, req: CookieImport) -> Result<ImportReport> {
        let p = self.require_closed(id).await?;
        let (cookies, mut report) = prepare(&req, self.d.clock.now())?;
        if cookies.is_empty() {
            // Nothing valid: do not touch the profile (not even in "all" mode).
            return Ok(report);
        }
        report.imported = self.d.engine.import_cookies(&p, &cookies, req.mode).await?;
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_700_000_000;

    fn req(content: &str) -> CookieImport {
        CookieImport { content: content.into(), ..Default::default() }
    }

    #[test]
    fn json_extension_format() {
        let j = r#"[
          {"domain":".example.com","hostOnly":false,"httpOnly":true,"name":"sid","path":"/","sameSite":"no_restriction","secure":true,"session":false,"expirationDate":1800000000.5,"value":"abc"},
          {"domain":"example.org","name":"t","value":"1"},
          {"domain":"example.com","name":"old","value":"x","expirationDate":1000},
          {"domain":"","name":"nodomain","value":"x"},
          {"domain":"a.com","name":"","value":"x"},
          {"domain":"a b.com","name":"n","value":"x"},
          {"domain":"a.com","name":"n","value":"x","path":"nope"}
        ]"#;
        let (c, r) = prepare(&req(j), NOW).unwrap();
        assert_eq!(r.format, CookieFormat::Json);
        assert_eq!((r.total, r.skipped, c.len()), (7, 5, 2));
        assert_eq!(c[0].domain, ".example.com");
        assert!(c[0].http_only && c[0].secure && !c[0].host_only);
        assert_eq!(c[0].same_site.as_deref(), Some("None"));
        assert_eq!(c[0].expires, Some(1_800_000_000.5));
        // no expiry -> session, extended; no leading dot -> host-only
        assert!(c[1].host_only && c[1].domain == "example.org");
        assert_eq!(c[1].expires, Some((NOW + SESSION_TTL_SECS) as f64));
        assert_eq!(r.session_extended, 1);
        let codes: Vec<_> = r.skipped_reasons.iter().map(|s| (s.code, s.count)).collect();
        assert_eq!(codes, vec![("badDomain", 2), ("badPath", 1), ("expired", 1), ("missingName", 1)]);
        assert_eq!(r.domains, vec!["example.com", "example.org"]);
    }

    #[test]
    fn json_object_wrapper_and_millis() {
        let (c, _) = prepare(&req(r#"{"cookies":[{"domain":"a.com","name":"n","value":"v","expires":1800000000000}]}"#), NOW).unwrap();
        assert_eq!(c[0].expires, Some(1_800_000_000.0));
        assert!(prepare(&req("{\"x\":1}"), NOW).is_err());
        assert!(prepare(&req("[1,"), NOW).is_err());
        assert!(prepare(&req("  "), NOW).is_err());
    }

    #[test]
    fn netscape_parse_and_render() {
        let t = "# Netscape HTTP Cookie File\n\n.example.com\tTRUE\t/\tTRUE\t1800000000\tsid\tabc\n#HttpOnly_www.example.org\tFALSE\t/p\tFALSE\t0\tt\tv=1\nbroken line\n# comment\n";
        let (c, r) = prepare(&req(t), NOW).unwrap();
        assert_eq!(r.format, CookieFormat::Netscape);
        assert_eq!((r.total, r.imported, r.skipped, c.len()), (3, 0, 1, 2));
        assert!(!c[0].host_only && c[0].secure && c[0].domain == ".example.com");
        assert!(c[1].host_only && c[1].http_only && c[1].value == "v=1" && c[1].path == "/p");
        let out = to_netscape(&c);
        assert!(out.contains(".example.com\tTRUE\t/\tTRUE\t1800000000\tsid\tabc\n"));
        assert!(out.contains("#HttpOnly_www.example.org\tFALSE\t/p\tFALSE\t"));
    }

    #[test]
    fn duplicates_later_wins_and_ip_hosts() {
        let t = "a.com\tFALSE\t/\tFALSE\t1800000000\tn\t1\n.a.com\tTRUE\t/\tFALSE\t1800000000\tn\t2\n127.0.0.1\tTRUE\t/\tFALSE\t1800000000\tn\t3\nlocalhost\tTRUE\t/\tFALSE\t1800000000\tn\t4\n";
        let (c, r) = prepare(&req(t), NOW).unwrap();
        assert_eq!(
            c.iter().map(|c| (c.domain.as_str(), c.value.as_str())).collect::<Vec<_>>(),
            vec![("a.com", "1"), (".a.com", "2"), ("127.0.0.1", "3"), ("localhost", "4")]
        );
        assert_eq!(r.skipped, 0);
    }

    #[test]
    fn json_round_trip() {
        let c = Cookie {
            name: "n".into(),
            value: "v".into(),
            domain: ".a.com".into(),
            path: "/".into(),
            expires: Some(1.8e9),
            http_only: true,
            secure: true,
            same_site: Some("Lax".into()),
            host_only: false,
        };
        let (back, _) = prepare(&req(&to_json(std::slice::from_ref(&c))), NOW).unwrap();
        assert_eq!(back, vec![c]);
    }
}
