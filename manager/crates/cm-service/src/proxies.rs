//! Proxies: library CRUD, list import, on-demand check, assignment (one proxy → many profiles,
//! or a list → profiles in order), local relays.
//!
//! Passwords never leave this module towards callers: [`ProxyView`] has no password and no error
//! message built here contains credentials.

use crate::Manager;
use anyhow::{Result, bail};
use cm_core::{CmError, NewProxy, Proxy, ProxyCheck, ProxyId, ProxyScheme};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A cached check younger than this is reused at start; older → check again (rotating IPs).
const CHECK_TTL_SECS: u64 = 600;
/// Import limits: đủ cho danh sách proxy lớn khi scale, vẫn chặn input vô lý.
const IMPORT_MAX_LINES: usize = 10_000;
const IMPORT_MAX_CHARS: usize = 2_000_000;
/// Profiles per bulk / sequential assignment.
const ASSIGN_MAX: usize = 10_000;

/// Proxy as the UI sees it (no password).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyView {
    pub id: ProxyId,
    pub name: String,
    pub scheme: &'static str,
    /// `host:port`
    pub label: String,
    pub host: String,
    pub server_port: u16,
    pub username: Option<String>,
    pub has_password: bool,
    /// Local relay port the browser uses for this proxy.
    pub port: Option<u16>,
    /// Last check, if any.
    pub geo: Option<ProxyCheck>,
    /// How many profiles use it (only filled by the library listing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_by: Option<usize>,
}

/// One rejected import line. `code` is stable (the UI translates it); never contains the line.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LineError {
    pub line: usize,
    pub code: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// Non-empty = the whole list was rejected and nothing was saved.
    pub rejected: Vec<LineError>,
    pub created: Vec<ProxyView>,
    /// 1-based line numbers of exact duplicates (already stored, or repeated in the list).
    pub skipped: Vec<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignResult {
    pub profile_id: String,
    pub ok: bool,
    /// `not-found` | `running` | `error`
    pub reason: Option<&'static str>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceRequest {
    /// Selection order = pairing order.
    pub profile_ids: Vec<String>,
    /// Library entries to hand out in order; `None` = the whole library by id.
    #[serde(default)]
    pub proxy_ids: Option<Vec<ProxyId>>,
    /// Only compute the mapping.
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SequencePair {
    pub profile_id: String,
    pub proxy_id: Option<ProxyId>,
    pub applied: bool,
    /// `not-found` | `running` | `no-proxy` | `error`
    pub reason: Option<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedProxy {
    pub proxy_id: ProxyId,
    /// `used-ip`: another profile (or an earlier pair) already uses this exit IP / endpoint.
    pub reason: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceResult {
    pub dry_run: bool,
    pub pairs: Vec<SequencePair>,
    pub skipped_proxies: Vec<SkippedProxy>,
}

fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(CmError::Invalid(msg.into()).into())
}

/// Identity used to avoid giving two profiles the same IP: the exit IP when checked, else the endpoint.
fn ip_key(p: &Proxy) -> String {
    match &p.last_check {
        Some(c) if !c.ip.is_empty() => c.ip.clone(),
        _ => format!("{}:{}", p.host.to_ascii_lowercase(), p.port),
    }
}

/// Normalise and validate a library entry (form or import). `Err` is a stable code.
fn normalize(mut p: NewProxy) -> Result<NewProxy, &'static str> {
    p.name = p.name.trim().to_string();
    let host = p.host.trim().trim_start_matches('[').trim_end_matches(']').to_string();
    if host.is_empty() || host.chars().any(|c| c.is_whitespace() || matches!(c, '/' | '@' | '?' | '#' | '[' | ']')) {
        return Err("bad-host");
    }
    // A colon is only legal inside an IPv6 literal.
    if host.contains(':') && host.parse::<std::net::Ipv6Addr>().is_err() {
        return Err("bad-host");
    }
    p.host = host;
    if p.port == 0 {
        return Err("bad-port");
    }
    if p.name.chars().count() > 100 {
        return Err("bad-name");
    }
    if p.username.as_deref() == Some("") {
        p.username = None;
    }
    let (u, pw) = (p.username.as_deref(), p.password.as_deref().filter(|s| !s.is_empty()));
    match p.scheme {
        ProxyScheme::Socks4 | ProxyScheme::Socks4a if u.is_some() || pw.is_some() => return Err("creds-not-allowed"),
        ProxyScheme::Socks5 | ProxyScheme::Socks5h => {
            // RFC 1929: 1–255 bytes each.
            if pw.is_some() && u.is_none() {
                return Err("bad-credentials");
            }
            if let Some(u) = u
                && (u.len() > 255 || pw.is_none_or(|p| p.len() > 255))
            {
                return Err("bad-credentials");
            }
        }
        _ => {}
    }
    if p.name.is_empty() {
        p.name = if p.host.contains(':') { format!("[{}]:{}", p.host, p.port) } else { format!("{}:{}", p.host, p.port) };
    }
    Ok(p)
}

fn pct_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let h = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(h, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// `host:port` or `[v6]:port` → (host, port).
fn split_host_port(s: &str) -> Result<(String, u16), &'static str> {
    let (host, port) = if let Some(rest) = s.strip_prefix('[') {
        let (h, tail) = rest.split_once(']').ok_or("bad-format")?;
        (h, tail.strip_prefix(':').ok_or("bad-format")?)
    } else {
        s.split_once(':').ok_or("bad-format")?
    };
    let port: u16 = port.parse().map_err(|_| "bad-port")?;
    Ok((host.to_string(), port))
}

/// One import line. Accepted:
/// `[scheme://][user[:pass]@]host:port` (IPv6 in brackets, credentials percent-encoded) and the
/// legacy `host:port:user:pass`. Bare = HTTP. Paths/queries/fragments are rejected.
fn parse_line(line: &str) -> Result<NewProxy, &'static str> {
    let line = line.trim();
    let (scheme, rest) = match line.split_once("://") {
        Some((s, r)) => (ProxyScheme::parse(s).ok_or("bad-scheme")?, r),
        None => (ProxyScheme::Http, line),
    };
    let rest = rest.strip_suffix('/').unwrap_or(rest);
    if rest.rsplit('@').next().is_some_and(|h| h.contains(['/', '?', '#'])) {
        return Err("has-path");
    }
    let mut p = NewProxy { scheme, ..Default::default() };
    if let Some((userinfo, hostport)) = rest.rsplit_once('@') {
        let (u, pw) = match userinfo.split_once(':') {
            Some((u, pw)) => (u, Some(pw)),
            None => (userinfo, None),
        };
        p.username = Some(pct_decode(u).ok_or("bad-credentials")?);
        p.password = pw.map(|x| pct_decode(x).ok_or("bad-credentials")).transpose()?;
        (p.host, p.port) = split_host_port(hostport)?;
    } else if !line.contains("://") && !rest.starts_with('[') && rest.matches(':').count() >= 3 {
        // Legacy host:port:user:pass (password may contain ':').
        let mut it = rest.splitn(4, ':');
        let (host, port, user, pass) = (it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap());
        p.host = host.into();
        p.port = port.parse().map_err(|_| "bad-port")?;
        p.username = Some(user.into());
        p.password = Some(pass.into());
    } else {
        (p.host, p.port) = split_host_port(rest)?;
    }
    normalize(p)
}

/// Parse a whole list; `Err` lists every bad line (1-based, blank lines and `#` comments skipped).
#[allow(clippy::type_complexity)]
fn parse_list(text: &str) -> Result<Result<Vec<(usize, NewProxy)>, Vec<LineError>>> {
    if text.chars().count() > IMPORT_MAX_CHARS {
        return invalid(format!("list too long (max {IMPORT_MAX_CHARS} characters)"));
    }
    let lines: Vec<(usize, &str)> =
        text.lines().enumerate().map(|(i, l)| (i + 1, l.trim())).filter(|(_, l)| !l.is_empty() && !l.starts_with('#')).collect();
    if lines.is_empty() {
        return invalid("nothing to import");
    }
    if lines.len() > IMPORT_MAX_LINES {
        return invalid(format!("too many lines (max {IMPORT_MAX_LINES})"));
    }
    let (mut good, mut bad) = (Vec::new(), Vec::new());
    for (n, l) in lines {
        match parse_line(l) {
            Ok(p) => good.push((n, p)),
            Err(code) => bad.push(LineError { line: n, code }),
        }
    }
    Ok(if bad.is_empty() { Ok(good) } else { Err(bad) })
}

fn same_entry(a: &NewProxy, b: &NewProxy) -> bool {
    a.scheme == b.scheme
        && a.host.eq_ignore_ascii_case(&b.host)
        && a.port == b.port
        && a.username == b.username
        && a.password.as_deref().unwrap_or("") == b.password.as_deref().unwrap_or("")
}

impl Manager {
    /// Local relay port for proxy `id`: `base + id - 1`.
    pub(crate) fn relay_port(&self, id: ProxyId) -> Option<u16> {
        u16::try_from(id - 1).ok().and_then(|off| self.cfg.proxy_base_port.checked_add(off))
    }

    pub(crate) fn proxy_view(&self, p: &Proxy) -> ProxyView {
        ProxyView {
            id: p.id,
            name: p.name.clone(),
            scheme: p.scheme.as_str(),
            label: p.label(),
            host: p.host.clone(),
            server_port: p.port,
            username: p.username.clone(),
            has_password: p.has_password(),
            port: self.relay_port(p.id),
            geo: p.last_check.clone(),
            used_by: None,
        }
    }

    /// The library, with how many profiles use each entry.
    pub async fn proxies(&self) -> Result<Vec<ProxyView>> {
        let mut uses: HashMap<ProxyId, usize> = HashMap::new();
        for p in self.d.profiles.list().await? {
            if let Some(id) = p.proxy_id {
                *uses.entry(id).or_default() += 1;
            }
        }
        Ok(self
            .d
            .proxies
            .list()
            .await?
            .iter()
            .map(|p| ProxyView { used_by: Some(uses.get(&p.id).copied().unwrap_or(0)), ..self.proxy_view(p) })
            .collect())
    }

    pub(crate) async fn proxy(&self, id: ProxyId) -> Result<Proxy> {
        self.d.proxies.get(id).await?.ok_or_else(|| CmError::NotFound(format!("proxy #{id}")).into())
    }

    async fn in_use_by_running(&self, id: ProxyId) -> bool {
        self.running.lock().await.values().any(|r| r.proxy == Some(id))
    }

    pub async fn create_proxy(&self, p: NewProxy) -> Result<ProxyView> {
        let p = normalize(p).or_else(|c| invalid(format!("invalid proxy: {c}")))?;
        Ok(self.proxy_view(&self.d.proxies.insert(&p).await?))
    }

    /// Replace an entry. `password: None` keeps the stored password, `clear_password` removes it.
    pub async fn update_proxy(&self, id: ProxyId, mut p: NewProxy, clear_password: bool) -> Result<ProxyView> {
        self.proxy(id).await?;
        if clear_password {
            p.password = Some(String::new());
        }
        let p = normalize(p).or_else(|c| invalid(format!("invalid proxy: {c}")))?;
        if self.in_use_by_running(id).await {
            bail!(CmError::Conflict("stop the profiles using this proxy before editing it".into()));
        }
        self.d.proxies.update(id, &p).await?;
        // A relay left over from an earlier run holds the old upstream.
        self.release_relay(id).await;
        Ok(self.proxy_view(&self.proxy(id).await?))
    }

    pub async fn delete_proxy(&self, id: ProxyId) -> Result<()> {
        self.proxy(id).await?;
        if self.in_use_by_running(id).await {
            bail!(CmError::Conflict("stop the profiles using this proxy before deleting it".into()));
        }
        self.d.proxies.delete(id).await?;
        self.release_relay(id).await;
        Ok(())
    }

    /// Import a pasted list. The whole list is validated first; any bad line → nothing is saved.
    pub async fn import_proxies(&self, text: &str) -> Result<ImportReport> {
        let parsed = match parse_list(text)? {
            Ok(v) => v,
            Err(rejected) => return Ok(ImportReport { rejected, created: vec![], skipped: vec![] }),
        };
        let mut known: Vec<NewProxy> = self
            .d
            .proxies
            .list()
            .await?
            .into_iter()
            .map(|p| NewProxy { name: p.name, scheme: p.scheme, host: p.host, port: p.port, username: p.username, password: p.password })
            .collect();
        let (mut created, mut skipped) = (Vec::new(), Vec::new());
        for (line, p) in parsed {
            if known.iter().any(|k| same_entry(k, &p)) {
                skipped.push(line);
                continue;
            }
            created.push(self.proxy_view(&self.d.proxies.insert(&p).await?));
            known.push(p);
        }
        Ok(ImportReport { rejected: vec![], created, skipped })
    }

    /// Check proxy `id` through itself. `fresh = false` reuses a recent stored check.
    pub async fn check_proxy(&self, id: ProxyId, fresh: bool) -> Result<ProxyCheck> {
        let p = self.proxy(id).await?;
        if !fresh
            && let Some(c) = &p.last_check
            && self.d.clock.now().saturating_sub(c.checked_at) < CHECK_TTL_SECS
        {
            return Ok(c.clone());
        }
        let c = match self.d.checker.check(&p).await {
            Ok(c) => c,
            Err(e) => {
                self.notify("proxy.check_failed", serde_json::json!({ "proxy": p.name, "id": id, "error": format!("{e:#}") }), None).await;
                return Err(e);
            }
        };
        self.d.proxies.set_check(id, &c).await?;
        Ok(c)
    }

    /// Assign (`Some`) or clear (`None`) a profile's proxy. Applies at the next start.
    pub async fn assign(&self, profile: &str, proxy: Option<ProxyId>) -> Result<()> {
        if self.running.lock().await.contains_key(profile) {
            bail!(CmError::Conflict("stop the profile before changing its proxy".into()));
        }
        if let Some(id) = proxy {
            self.proxy(id).await?;
        }
        self.d.profiles.set_proxy(profile, proxy).await
    }

    /// Assign one proxy (or clear) to several profiles; running / unknown ones are skipped with a reason.
    pub async fn assign_many(&self, proxy: Option<ProxyId>, profiles: &[String]) -> Result<Vec<AssignResult>> {
        if profiles.is_empty() || profiles.len() > ASSIGN_MAX {
            return invalid(format!("select 1–{ASSIGN_MAX} profiles"));
        }
        if let Some(id) = proxy {
            self.proxy(id).await?;
        }
        let mut out = Vec::new();
        for id in profiles {
            let reason = match self.assign(id, proxy).await {
                Ok(()) => None,
                Err(e) => Some(match e.downcast_ref::<CmError>() {
                    Some(CmError::NotFound(_)) => "not-found",
                    Some(CmError::Conflict(_)) => "running",
                    _ => "error",
                }),
            };
            out.push(AssignResult { profile_id: id.clone(), ok: reason.is_none(), reason });
        }
        Ok(out)
    }

    /// Hand the proxies out in order, one per selected profile, skipping entries whose exit IP /
    /// endpoint is already used by a profile outside the selection or by an earlier pair.
    /// Running profiles are skipped without consuming a proxy.
    pub async fn assign_sequence(&self, req: SequenceRequest) -> Result<SequenceResult> {
        if req.profile_ids.is_empty() || req.profile_ids.len() > ASSIGN_MAX {
            return invalid(format!("select 1–{ASSIGN_MAX} profiles"));
        }
        if req.profile_ids.iter().collect::<HashSet<_>>().len() != req.profile_ids.len() {
            return invalid("a profile is selected twice");
        }
        let library = self.d.proxies.list().await?;
        let by_id: HashMap<ProxyId, &Proxy> = library.iter().map(|p| (p.id, p)).collect();
        let order: Vec<ProxyId> = match &req.proxy_ids {
            Some(ids) => {
                if ids.iter().collect::<HashSet<_>>().len() != ids.len() {
                    return invalid("a proxy is selected twice");
                }
                if let Some(missing) = ids.iter().find(|i| !by_id.contains_key(i)) {
                    bail!(CmError::NotFound(format!("proxy #{missing}")));
                }
                ids.clone()
            }
            None => library.iter().map(|p| p.id).collect(),
        };

        let profiles = self.d.profiles.list().await?;
        let selected: HashSet<&String> = req.profile_ids.iter().collect();
        let mut used: HashSet<String> = profiles
            .iter()
            .filter(|p| !selected.contains(&p.id))
            .filter_map(|p| p.proxy_id)
            .filter_map(|i| by_id.get(&i))
            .map(|p| ip_key(p))
            .collect();
        let running = self.running.lock().await.keys().cloned().collect::<HashSet<_>>();

        let (mut pairs, mut skipped_proxies) = (Vec::new(), Vec::new());
        let mut queue = order.into_iter();
        for pid in &req.profile_ids {
            let mut pair = SequencePair { profile_id: pid.clone(), proxy_id: None, applied: false, reason: None };
            if !profiles.iter().any(|p| &p.id == pid) {
                pair.reason = Some("not-found");
            } else if running.contains(pid) {
                pair.reason = Some("running");
            } else {
                for id in queue.by_ref() {
                    if used.insert(ip_key(by_id[&id])) {
                        pair.proxy_id = Some(id);
                        break;
                    }
                    skipped_proxies.push(SkippedProxy { proxy_id: id, reason: "used-ip" });
                }
                if pair.proxy_id.is_none() {
                    pair.reason = Some("no-proxy");
                } else if !req.dry_run {
                    match self.d.profiles.set_proxy(pid, pair.proxy_id).await {
                        Ok(()) => pair.applied = true,
                        Err(_) => pair.reason = Some("error"),
                    }
                }
            }
            pairs.push(pair);
        }
        Ok(SequenceResult { dry_run: req.dry_run, pairs, skipped_proxies })
    }

    /// Make sure the relay for proxy `id` is listening; returns its port.
    pub(crate) async fn ensure_relay(&self, p: &Proxy) -> Result<u16> {
        let mut relays = self.relays.lock().await;
        if let Some((port, h)) = relays.get(&p.id)
            && !h.is_finished()
        {
            return Ok(*port);
        }
        let Some(port) = self.relay_port(p.id) else { bail!("no local port left for proxy #{}", p.id) };
        let listener = cm_relay::bind(port).await?;
        relays.insert(p.id, (port, tokio::spawn(cm_relay::run(listener, p.into()))));
        Ok(port)
    }

    /// Stop the relay of proxy `id` unless a running profile still uses it.
    pub(crate) async fn release_relay(&self, id: ProxyId) {
        let in_use = self.running.lock().await.values().any(|r| r.proxy == Some(id));
        if !in_use && let Some((_, h)) = self.relays.lock().await.remove(&id) {
            h.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(s: &str) -> NewProxy {
        parse_line(s).unwrap()
    }

    #[test]
    fn parses_all_formats() {
        let p = line("proxy.example:8080");
        assert_eq!((p.scheme, p.host.as_str(), p.port, p.username), (ProxyScheme::Http, "proxy.example", 8080, None));
        let p = line("https://user:p%40ss%3Aword@proxy.example:443");
        assert_eq!((p.scheme, p.username.as_deref(), p.password.as_deref()), (ProxyScheme::Https, Some("user"), Some("p@ss:word")));
        let p = line("socks5://[2001:db8::1]:1080");
        assert_eq!((p.scheme, p.host.as_str(), p.port), (ProxyScheme::Socks5, "2001:db8::1", 1080));
        assert_eq!(line("socks5h://u:p@h.example:1").scheme, ProxyScheme::Socks5h);
        assert_eq!(line("socks4a://h.example:1").scheme, ProxyScheme::Socks4a);
        let p = line("1.2.3.4:8080:user:pa:ss");
        assert_eq!((p.username.as_deref(), p.password.as_deref()), (Some("user"), Some("pa:ss")));
        assert_eq!(line("u:p@h.example:80").username.as_deref(), Some("u"));
        assert_eq!(line("h.example:80").name, "h.example:80");
    }

    #[test]
    fn rejects_bad_lines_without_echoing_them() {
        for (l, code) in [
            ("ftp://h:1", "bad-scheme"),
            ("h:99999", "bad-port"),
            ("h:0", "bad-port"),
            ("h", "bad-format"),
            ("http://h:80/path", "has-path"),
            ("http://u:p@h:80/x?y=1", "has-path"),
            ("http://u:%zz@h:80", "bad-credentials"),
            ("socks4://u:p@h:80", "creds-not-allowed"),
            ("socks5://u@h:80", "bad-credentials"),
            ("http://:80", "bad-host"),
        ] {
            assert_eq!(parse_line(l).unwrap_err(), code, "{l}");
        }
    }

    #[test]
    fn list_is_all_or_nothing_and_limited() {
        let r = parse_list("ok.example:1\n\n# c\nbad\nsocks9://x:1").unwrap().unwrap_err();
        assert_eq!(r, vec![LineError { line: 4, code: "bad-format" }, LineError { line: 5, code: "bad-scheme" }]);
        assert_eq!(parse_list("a:1\nb:2").unwrap().unwrap().len(), 2);
        assert!(parse_list("\n\n").is_err());
        assert!(parse_list(&"a:1\n".repeat(IMPORT_MAX_LINES + 1)).is_err());
    }
}
