//! Profiles: overview (list for the UI), create/delete, start/stop, running state.
//! Phase 4 extends this module (edit settings, pinned machine, OS change).

use crate::{Manager, Running, proxies::ProxyView};
use anyhow::{Context, Result, bail};
use cm_core::{
    CanvasMode, CmError, EngineHint, LaunchOptions, MachineSummary, NewProfile, Os, Profile, ProfileSettings, ProxyCheck, ProxyId,
    WebRtcMode,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path, path::PathBuf};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningView {
    pub ws: String,
    pub port: Option<u16>,
    pub started_at: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathInfo {
    pub dir: String,
    pub exists: bool,
    pub size_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileView {
    pub id: String,
    pub name: String,
    pub os: Os,
    pub notes: Option<String>,
    pub created_at: String,
    /// Last start, or creation time if never started.
    pub last_activity: Option<String>,
    /// Timezone override; `None` + `timezone_by_ip` = follows the proxy.
    pub timezone: Option<String>,
    pub timezone_by_ip: bool,
    pub settings: ProfileSettings,
    /// Pinned machine (read-only).
    pub fp: MachineSummary,
    pub assigned: Option<ProxyView>,
    pub running: Option<RunningView>,
    /// Agent lease (phase 8).
    pub lease: Option<crate::lease::LeaseView>,
    /// The browser died on its own (phase 8); cleared by the next start.
    pub crashed: Option<cm_core::CrashInfo>,
    pub path: PathInfo,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub provider: &'static str,
    /// Engine can launch; false + `provider_hint` when not installed (Camoufox is optional).
    pub provider_available: bool,
    pub provider_hint: Option<EngineHint>,
    /// An engine install / update is running (the UI links to its log instead of offering it again).
    pub provider_installing: bool,
    /// Protocol of the ws URL returned by start (`Playwright`).
    pub ws_protocol: &'static str,
    pub proxy_count: usize,
    pub api_error: Option<String>,
    pub profiles: Vec<ProfileView>,
}

#[derive(Serialize)]
pub struct StartResult {
    pub ws: String,
    pub port: Option<u16>,
    pub geo: Option<ProxyCheck>,
    pub already: bool,
}

fn dir_size(p: &Path) -> u64 {
    let Ok(rd) = fs::read_dir(p) else { return 0 };
    rd.flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => dir_size(&e.path()),
            Ok(t) if t.is_file() => e.metadata().map(|m| m.len()).unwrap_or(0),
            _ => 0,
        })
        .sum()
}

/// 24 random hex chars (from /dev/urandom, no extra crate).
pub(crate) fn new_id() -> Result<String> {
    let mut b = [0u8; 12];
    getrandom::fill(&mut b).map_err(|e| anyhow::anyhow!("cannot read OS randomness: {e}"))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

impl Manager {
    fn path_info(&self, id: &str) -> PathInfo {
        let d = self.d.engine.data_dir(id);
        let exists = d.is_dir();
        PathInfo { dir: d.display().to_string(), exists, size_bytes: if exists { dir_size(&d) } else { 0 } }
    }

    pub(crate) async fn profile(&self, id: &str) -> Result<Profile> {
        self.d.profiles.get(id).await?.ok_or_else(|| CmError::NotFound(format!("profile {id}")).into())
    }

    /// Forget browsers the engine reports as gone (user closed the window, crash).
    pub(crate) async fn prune_dead(&self) {
        let ids: Vec<String> = self.running.lock().await.keys().cloned().collect();
        for id in ids {
            if self.d.engine.alive(&id).await == Some(false) {
                let proxy = self.running.lock().await.remove(&id).and_then(|r| r.proxy);
                if let Some(px) = proxy {
                    self.release_relay(px).await;
                }
                println!("{id} exited on its own");
                self.notify_crash_of(&id).await;
            }
        }
    }

    pub async fn overview(&self) -> Overview {
        self.prune_dead().await;
        let status = self.d.engine.status().await;
        let (profiles, proxies, api_error) = match (self.d.profiles.list().await, self.d.proxies.list().await) {
            (Ok(p), Ok(x)) => (p, x, None),
            (p, x) => {
                let e = p.as_ref().err().or(x.as_ref().err()).map(|e| format!("{e:#}"));
                (p.unwrap_or_default(), x.unwrap_or_default(), e)
            }
        };
        let by_id: HashMap<ProxyId, ProxyView> = proxies.iter().map(|p| (p.id, self.proxy_view(p))).collect();
        let leases = self.leases_snapshot();
        let mut crashes = HashMap::new();
        for p in &profiles {
            if let Some(c) = self.d.engine.crashed(&p.id).await {
                self.notify_crash(&p.id, &p.name, &c).await;
                crashes.insert(p.id.clone(), c);
            }
        }
        let running = self.running.lock().await;
        let views = profiles
            .into_iter()
            .map(|p| ProfileView {
                lease: leases.get(&p.id).cloned(),
                crashed: crashes.get(&p.id).cloned(),
                assigned: p.proxy_id.and_then(|i| by_id.get(&i).cloned()),
                running: running.get(&p.id).map(|r| RunningView { ws: r.ws.clone(), port: r.port, started_at: r.started_at }),
                path: self.path_info(&p.id),
                fp: self.fp_of(&p),
                timezone: p.settings.timezone.clone(),
                timezone_by_ip: p.settings.timezone.is_none(),
                last_activity: p.last_activity.clone().or_else(|| Some(p.created_at.clone())),
                id: p.id,
                name: p.name,
                os: p.os,
                notes: p.notes,
                created_at: p.created_at,
                settings: p.settings,
            })
            .collect();
        Overview {
            provider: self.d.engine.name(),
            provider_available: status.ready,
            provider_hint: status.hint,
            provider_installing: crate::settings::install_running(),
            ws_protocol: self.d.engine.ws_protocol(),
            proxy_count: proxies.len(),
            api_error,
            profiles: views,
        }
    }

    pub async fn start(&self, id: &str) -> Result<StartResult> {
        self.start_as(id, None).await
    }

    /// Start on behalf of `holder` (the agent holding the lease, if any); refused while someone
    /// else (or an anonymous caller) asks for a leased profile.
    pub async fn start_as(&self, id: &str, holder: Option<&str>) -> Result<StartResult> {
        self.lease_check_start(id, holder)?;
        self.journal_check_start(id).await?;
        self.prune_dead().await;
        if let Some(r) = self.running.lock().await.get(id) {
            return Ok(StartResult { ws: r.ws.clone(), port: r.port, geo: None, already: true });
        }
        let profile = self.profile(id).await?;
        // Fail fast (before the slow proxy check) when the engine cannot launch anyway.
        let status = self.d.engine.status().await;
        if !status.ready {
            let why = status.hint.map(|h| h.detail).unwrap_or_default();
            bail!("{} is not ready: {why}", self.d.engine.name());
        }
        let (mut port, mut geo) = (None, None);
        if let Some(pid) = profile.proxy_id {
            // Relay up and geo known BEFORE the browser opens: dead proxy → no browser (fail-closed).
            let proxy = self.proxy(pid).await?;
            let p = self.ensure_relay(&proxy).await?;
            let g = match self.check_proxy(pid, false).await {
                Ok(g) => g,
                Err(e) => {
                    self.release_relay(pid).await;
                    return Err(e.context("proxy check failed, browser not opened"));
                }
            };
            if let Some(why) = proxy_timezone_gap(&profile, Some(&g)) {
                self.release_relay(pid).await;
                return Err(CmError::Invalid(why.into()).into());
            }
            port = Some(p);
            geo = Some(g);
        }
        // Proxy-derived geography is the default, the profile's overrides win (D4).
        let mut opts = launch_options_for(&profile, geo.as_ref(), port);
        opts.extensions = self.extension_files(id).await; // CM-1.9: assigned Firefox extensions
        // Settings page default (phase 12); unset = the engine's own default (CAMOUFOX_HEADLESS).
        opts.headless = self.headless_default().await;
        let session = match self.d.engine.launch(&profile, &opts).await {
            Ok(s) => s,
            Err(e) => {
                if let Some(pid) = profile.proxy_id {
                    self.release_relay(pid).await;
                }
                return Err(e);
            }
        };
        let now = self.d.clock.now();
        self.running
            .lock()
            .await
            .insert(id.to_string(), Running { ws: session.ws_url.clone(), proxy: profile.proxy_id, port, started_at: now });
        let _ = self.d.profiles.touch(id, &cm_core::clock::iso(now)).await;
        Ok(StartResult { ws: session.ws_url, port, geo, already: false })
    }

    pub async fn stop(&self, id: &str) -> Result<()> {
        let _ = self.lease_release(id, None).await; // phase 8: stopping ends the lease
        self.d.engine.stop(id).await?;
        let proxy = self.running.lock().await.remove(id).and_then(|r| r.proxy);
        if let Some(px) = proxy {
            self.release_relay(px).await;
        }
        Ok(())
    }

    /// Stop every running profile so the engine closes browsers cleanly (app exit).
    pub async fn stop_all(&self) {
        let ids: Vec<String> = self.running.lock().await.keys().cloned().collect();
        for id in ids {
            let _ = self.stop(&id).await;
            println!("stopped {id}");
        }
    }

    pub async fn create_profile(&self, req: NewProfile) -> Result<ProfileView> {
        let name = req.name.trim();
        if name.is_empty() {
            bail!(CmError::Invalid("profile name must not be empty".into()));
        }
        let Some(os) = Os::parse(&req.os) else { bail!(CmError::Invalid("os must be win, mac or lin".into())) };
        let machine = self.d.engine.generate(os).await?;
        let now = self.d.clock.now_iso();
        let p = Profile {
            id: new_id()?,
            name: name.to_string(),
            os,
            notes: None,
            created_at: now.clone(),
            updated_at: now,
            last_activity: None,
            proxy_id: None,
            machine,
            settings: ProfileSettings::default(),
        };
        self.d.profiles.insert(&p).await?;
        let id = p.id.clone();
        self.overview().await.profiles.into_iter().find(|v| v.id == id).context("created profile vanished")
    }

    /// Delete: stop if running, remove the record, then the data on disk.
    pub async fn delete_profile(&self, id: &str) -> Result<()> {
        self.profile(id).await?;
        if self.running.lock().await.contains_key(id) {
            self.stop(id).await?;
        }
        self.d.profiles.delete(id).await?;
        self.d.engine.delete_data(id).await
    }

    /// Data directory of the profile; error if it does not exist yet.
    pub async fn profile_dir(&self, id: &str) -> Result<PathBuf> {
        self.profile(id).await?;
        let dir = self.d.engine.data_dir(id);
        if !dir.is_dir() {
            bail!(CmError::NotFound("profile data on disk (never started)".into()));
        }
        Ok(dir)
    }
}

// ---------------------------------------------------------------------------------------------
// Phase 4: edit settings, pinned machine, launch overrides
// ---------------------------------------------------------------------------------------------

/// What to do with the pinned machine when the requested OS differs from it.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OsAction {
    /// Ignore the OS change, keep the machine (and its OS).
    Keep,
    /// Pin a brand-new machine of the requested OS.
    Regenerate,
}

/// Full editable document of a profile (what the profile form sends).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileEdit {
    pub name: String,
    #[serde(default)]
    pub notes: Option<String>,
    /// Requested OS; differs from the pinned one → `os_action` is required.
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub os_action: Option<OsAction>,
    #[serde(default)]
    pub settings: ProfileSettings,
}

fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(CmError::Invalid(msg.into()).into())
}

/// `WxH` → (w, h), 200..=10000 each.
fn parse_size(s: &str, what: &str) -> Result<(u32, u32)> {
    let parsed =
        s.to_ascii_lowercase().split_once('x').and_then(|(w, h)| Some((w.trim().parse::<u32>().ok()?, h.trim().parse::<u32>().ok()?)));
    match parsed {
        Some((w, h)) if (200..=10_000).contains(&w) && (200..=10_000).contains(&h) => Ok((w, h)),
        _ => invalid(format!("{what} must look like 1920x1080 (200-10000)")),
    }
}

fn clean(s: Option<String>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// `de_DE` / `de-de` / `de` → `de-DE` / `de`.
fn normalize_locale(s: &str) -> Result<String> {
    let mut parts = s.split(['-', '_']);
    let (lang, region) = (parts.next().unwrap_or(""), parts.next());
    let alpha = |v: &str| v.bytes().all(|b| b.is_ascii_alphabetic());
    let ok_lang = (2..=3).contains(&lang.len()) && alpha(lang);
    let ok_region = region.is_none_or(|r| (r.len() == 2 && alpha(r)) || (r.len() == 3 && r.bytes().all(|b| b.is_ascii_digit())));
    if !ok_lang || !ok_region || parts.next().is_some() {
        return invalid("locale must look like de-DE or fr");
    }
    Ok(match region {
        Some(r) => format!("{}-{}", lang.to_ascii_lowercase(), r.to_ascii_uppercase()),
        None => lang.to_ascii_lowercase(),
    })
}

/// Same parser the scheduler uses for schedule timezones (B1): an unknown IANA name is a 400 on
/// save, not a failure at Start.
fn check_timezone_format(s: &str) -> Result<()> {
    crate::scheduler::parse_tz(s).map(|_| ())
}

/// B2: a proxy was checked but its geo answer has no timezone and the profile sets no override.
/// Launching then would silently use the host timezone, so the start must be refused.
pub(crate) fn proxy_timezone_gap(p: &Profile, geo: Option<&ProxyCheck>) -> Option<&'static str> {
    let g = geo?;
    (p.settings.timezone.is_none() && g.timezone.trim().is_empty())
        .then_some("proxy geo answer has no timezone, set a timezone override on the profile")
}

fn fmt_size((w, h): (u32, u32)) -> String {
    format!("{w}x{h}")
}

/// Validate and normalise edited settings. `pinned_screen` = the machine's screen (for the
/// "window fits the screen" check when there is no screen override).
pub(crate) fn validate_settings(mut s: ProfileSettings, pinned_screen: Option<(u32, u32)>) -> Result<ProfileSettings> {
    s.locale = clean(s.locale).map(|l| normalize_locale(&l)).transpose()?;
    s.timezone = clean(s.timezone);
    if let Some(tz) = &s.timezone {
        check_timezone_format(tz)?;
    }
    let screen = clean(s.screen.take()).map(|v| parse_size(&v, "screen")).transpose()?;
    let window = clean(s.window.take()).map(|v| parse_size(&v, "window")).transpose()?;
    if let (Some((ww, wh)), Some((sw, sh))) = (window, screen.or(pinned_screen))
        && (ww > sw || wh > sh)
    {
        return invalid(format!("window {ww}x{wh} does not fit the screen {sw}x{sh}"));
    }
    s.screen = screen.map(fmt_size);
    s.window = window.map(fmt_size);
    s.start_url = clean(s.start_url);
    if let Some(u) = &s.start_url {
        let lower = u.to_ascii_lowercase();
        if u.len() > 2000 || !(lower.starts_with("http://") || lower.starts_with("https://") || lower == "about:blank") {
            return invalid("start URL must be http(s)://... or about:blank");
        }
    }
    let mut fonts: Vec<String> = Vec::new();
    for f in s.fonts.iter().map(|f| f.trim()).filter(|f| !f.is_empty()) {
        if f.len() > 80 || f.chars().any(char::is_control) {
            return invalid("invalid font name");
        }
        if !fonts.iter().any(|x| x == f) {
            fonts.push(f.to_string());
        }
    }
    if fonts.len() > 200 {
        return invalid("at most 200 extra fonts");
    }
    s.fonts = fonts;
    Ok(s)
}

/// Primary locale of a country (ISO 3166-1 alpha-2): the default when the proxy decides geography.
fn locale_for_country(cc: &str) -> Option<&'static str> {
    Some(match cc.to_ascii_uppercase().as_str() {
        "US" => "en-US",
        "GB" => "en-GB",
        "CA" => "en-CA",
        "AU" => "en-AU",
        "NZ" => "en-NZ",
        "IE" => "en-IE",
        "IN" => "en-IN",
        "SG" => "en-SG",
        "ZA" => "en-ZA",
        "PH" => "en-PH",
        "DE" => "de-DE",
        "AT" => "de-AT",
        "CH" => "de-CH",
        "FR" => "fr-FR",
        "BE" => "nl-BE",
        "NL" => "nl-NL",
        "ES" => "es-ES",
        "MX" => "es-MX",
        "AR" => "es-AR",
        "CL" => "es-CL",
        "CO" => "es-CO",
        "PE" => "es-PE",
        "IT" => "it-IT",
        "PT" => "pt-PT",
        "BR" => "pt-BR",
        "PL" => "pl-PL",
        "CZ" => "cs-CZ",
        "SK" => "sk-SK",
        "HU" => "hu-HU",
        "RO" => "ro-RO",
        "BG" => "bg-BG",
        "GR" => "el-GR",
        "TR" => "tr-TR",
        "UA" => "uk-UA",
        "RU" => "ru-RU",
        "SE" => "sv-SE",
        "NO" => "nb-NO",
        "DK" => "da-DK",
        "FI" => "fi-FI",
        "JP" => "ja-JP",
        "KR" => "ko-KR",
        "CN" => "zh-CN",
        "TW" => "zh-TW",
        "HK" => "zh-HK",
        "TH" => "th-TH",
        "VN" => "vi-VN",
        "ID" => "id-ID",
        "MY" => "ms-MY",
        "IL" => "he-IL",
        "SA" => "ar-SA",
        "AE" => "ar-AE",
        "EG" => "ar-EG",
        "IR" => "fa-IR",
        "PK" => "ur-PK",
        "BD" => "bn-BD",
        _ => return None,
    })
}

/// Launch options for a profile: the proxy-derived geography is the default, the profile's own
/// overrides win. D4: geolocation is only ever sent together with a timezone, and only while that
/// timezone is the proxy's (coordinates of one place next to the clock of another is a tell).
pub(crate) fn launch_options_for(p: &Profile, geo: Option<&ProxyCheck>, port: Option<u16>) -> LaunchOptions {
    let s = &p.settings;
    let mut o = LaunchOptions {
        proxy: port.map(|port| ("127.0.0.1".to_string(), port)),
        public_ip: geo.map(|g| g.ip.clone()),
        webrtc: s.webrtc,
        screen: s.screen.clone(),
        window: s.window.clone(),
        extra_fonts: s.fonts.clone(),
        humanize: s.humanize.unwrap_or(false),
        start_url: s.start_url.clone(),
        canvas_stable: s.canvas == CanvasMode::Stable,
        canvas_native: s.canvas == CanvasMode::Native,
        ..Default::default()
    };
    o.timezone = s.timezone.clone().or_else(|| geo.map(|g| g.timezone.clone()).filter(|t| !t.is_empty()));
    if let (Some(g), Some(tz)) = (geo, &o.timezone)
        && *tz == g.timezone
    {
        o.geolocation = Some((g.lat, g.lon));
    }
    o.locale = s.locale.clone().or_else(|| geo.and_then(|g| locale_for_country(&g.country_code)).map(String::from));
    o
}

fn webrtc_code(m: WebRtcMode) -> &'static str {
    match m {
        WebRtcMode::Replace => "replace",
        WebRtcMode::Real => "real",
        WebRtcMode::Forward => "forward",
        WebRtcMode::None => "none",
    }
}

impl Manager {
    /// Read-only machine summary plus what the settings decide at launch.
    fn fp_of(&self, p: &Profile) -> MachineSummary {
        let mut fp = self.d.engine.summarize(&p.machine);
        let canvas = match p.settings.canvas {
            CanvasMode::Stable => "stable",
            CanvasMode::Native => "native",
            CanvasMode::Random => "random",
        };
        fp.noise.canvas = Some(canvas.into());
        fp.noise.webgl = Some(canvas.into());
        fp.noise.audio = Some("native".into());
        fp.webrtc = Some(webrtc_code(p.settings.webrtc).into());
        fp
    }

    pub(crate) fn pinned_screen(&self, p: &Profile) -> Option<(u32, u32)> {
        parse_size(self.d.engine.summarize(&p.machine).resolution.as_deref()?, "screen").ok()
    }

    async fn view(&self, id: &str) -> Result<ProfileView> {
        self.overview().await.profiles.into_iter().find(|v| v.id == id).context("profile vanished")
    }

    /// One profile (same shape as an overview row).
    pub async fn profile_view(&self, id: &str) -> Result<ProfileView> {
        self.profile(id).await?;
        self.view(id).await
    }

    /// Save the edited document. OS different from the pinned one needs an explicit [`OsAction`].
    pub async fn update_profile(&self, id: &str, edit: ProfileEdit) -> Result<ProfileView> {
        let mut p = self.profile(id).await?;
        let name = edit.name.trim();
        if name.is_empty() || name.chars().count() > 80 {
            return invalid("profile name must be 1-80 characters");
        }
        let notes = clean(edit.notes);
        if notes.as_ref().is_some_and(|n| n.chars().count() > 2000) {
            return invalid("notes are limited to 2000 characters");
        }
        let want = match edit.os.as_deref() {
            Some(o) => Os::parse(o).ok_or_else(|| CmError::Invalid("os must be win, mac or lin".into()))?,
            None => p.os,
        };
        let mut regenerate = None;
        if want != p.os {
            match edit.os_action {
                Some(OsAction::Keep) => {}
                Some(OsAction::Regenerate) => regenerate = Some(want),
                None => {
                    bail!(CmError::Conflict(format!(
                        "OS {} differs from the pinned machine ({}): choose osAction keep or regenerate",
                        want.code(),
                        p.os.code()
                    )))
                }
            }
        }
        if regenerate.is_some() && self.running.lock().await.contains_key(id) {
            bail!(CmError::Conflict("stop the profile before pinning a new machine".into()));
        }
        if let Some(os) = regenerate {
            p.machine = self.d.engine.generate(os).await?;
            p.os = os;
        }
        p.settings = validate_settings(edit.settings, self.pinned_screen(&p))?;
        p.name = name.to_string();
        p.notes = notes;
        p.updated_at = self.d.clock.now_iso();
        self.d.profiles.update(&p).await?;
        self.view(id).await
    }

    /// Pin a fresh machine (new hardware, screen, GPU, fonts) of `os` (default: the current OS).
    pub async fn regenerate_machine(&self, id: &str, os: Option<&str>) -> Result<ProfileView> {
        let mut p = self.profile(id).await?;
        if self.running.lock().await.contains_key(id) {
            bail!(CmError::Conflict("stop the profile before pinning a new machine".into()));
        }
        let os = match os {
            Some(o) => Os::parse(o).ok_or_else(|| CmError::Invalid("os must be win, mac or lin".into()))?,
            None => p.os,
        };
        p.machine = self.d.engine.generate(os).await?;
        p.os = os;
        // Screen/window overrides belonged to the old machine and may not fit the new one.
        if validate_settings(p.settings.clone(), self.pinned_screen(&p)).is_err() {
            p.settings.screen = None;
            p.settings.window = None;
        }
        p.updated_at = self.d.clock.now_iso();
        self.d.profiles.update(&p).await?;
        self.view(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(tz: &str, cc: &str) -> ProxyCheck {
        ProxyCheck { ip: "9.9.9.9".into(), country_code: cc.into(), timezone: tz.into(), lat: -23.5, lon: -46.6, ..Default::default() }
    }

    fn profile(settings: ProfileSettings) -> Profile {
        Profile {
            id: "aa".into(),
            name: "n".into(),
            os: Os::Mac,
            notes: None,
            created_at: String::new(),
            updated_at: String::new(),
            last_activity: None,
            proxy_id: None,
            machine: serde_json::json!({}),
            settings,
        }
    }

    #[test]
    fn proxy_geography_is_the_default() {
        let o = launch_options_for(&profile(Default::default()), Some(&check("America/Sao_Paulo", "BR")), Some(31000));
        assert_eq!(o.timezone.as_deref(), Some("America/Sao_Paulo"));
        assert_eq!(o.geolocation, Some((-23.5, -46.6)));
        assert_eq!(o.locale.as_deref(), Some("pt-BR"));
        assert_eq!(o.public_ip.as_deref(), Some("9.9.9.9"));
        assert!(o.canvas_stable);
    }

    #[test]
    fn overrides_win_and_geolocation_never_without_timezone() {
        let s = ProfileSettings { timezone: Some("Europe/Berlin".into()), locale: Some("de-DE".into()), ..Default::default() };
        let o = launch_options_for(&profile(s), Some(&check("America/Sao_Paulo", "BR")), Some(1));
        assert_eq!(o.timezone.as_deref(), Some("Europe/Berlin"));
        assert_eq!(o.locale.as_deref(), Some("de-DE"));
        assert_eq!(o.geolocation, None, "proxy coordinates next to another timezone would be a tell");
        // proxy without a known timezone: no timezone, so no geolocation either
        let o = launch_options_for(&profile(Default::default()), Some(&check("", "BR")), Some(1));
        assert!(o.timezone.is_none() && o.geolocation.is_none());
        // no proxy at all: nothing derived, only overrides
        let o = launch_options_for(&profile(Default::default()), None, None);
        assert!(o.timezone.is_none() && o.locale.is_none() && o.proxy.is_none());
    }

    #[test]
    fn settings_validation() {
        let ok = validate_settings(
            ProfileSettings {
                locale: Some(" de_de ".into()),
                window: Some("1200X800".into()),
                fonts: vec![" A ".into(), "A".into(), "".into()],
                ..Default::default()
            },
            Some((1920, 1080)),
        )
        .unwrap();
        assert_eq!(ok.locale.as_deref(), Some("de-DE"));
        assert_eq!(ok.window.as_deref(), Some("1200x800"));
        assert_eq!(ok.fonts, vec!["A".to_string()]);
        let bad = |s: ProfileSettings| validate_settings(s, Some((1920, 1080))).is_err();
        assert!(bad(ProfileSettings { window: Some("2000x800".into()), ..Default::default() }), "window larger than screen");
        assert!(bad(ProfileSettings { timezone: Some("Berlin".into()), ..Default::default() }));
        assert!(bad(ProfileSettings { timezone: Some("Mars/Base".into()), ..Default::default() }), "unknown IANA name (B1)");
        assert!(!bad(ProfileSettings { timezone: Some("Europe/Berlin".into()), ..Default::default() }));
        assert!(!bad(ProfileSettings { timezone: Some("UTC".into()), ..Default::default() }));
        assert!(bad(ProfileSettings { start_url: Some("file:///etc/passwd".into()), ..Default::default() }));
        assert!(bad(ProfileSettings { locale: Some("x".into()), ..Default::default() }));
        assert!(!bad(ProfileSettings { screen: Some("2560x1440".into()), window: Some("2400x1300".into()), ..Default::default() }));
    }

    #[test]
    fn proxy_without_timezone_fails_closed() {
        // B2: geo answer with an empty timezone and no override -> refuse (no host-timezone launch)
        let p = profile(Default::default());
        assert!(proxy_timezone_gap(&p, Some(&check("", "BR"))).is_some());
        assert!(proxy_timezone_gap(&p, Some(&check("  ", "BR"))).is_some());
        // a profile override covers the missing proxy timezone
        let with_tz = profile(ProfileSettings { timezone: Some("Europe/Berlin".into()), ..Default::default() });
        assert!(proxy_timezone_gap(&with_tz, Some(&check("", "BR"))).is_none());
        // a known proxy timezone, or no proxy at all, is not a gap
        assert!(proxy_timezone_gap(&p, Some(&check("America/Sao_Paulo", "BR"))).is_none());
        assert!(proxy_timezone_gap(&p, None).is_none());
    }
}
