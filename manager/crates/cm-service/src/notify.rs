//! Notifications (phase 19): a persisted list behind the bell in the header, plus an optional
//! macOS banner (`osascript`, setting `notify.system`, default off). Local only.
//!
//! Sources: profile crashed (engine crash info), schedule run failed / finished (run history),
//! proxy check failed, import / export finished (emitted by the HTTP layer once the call succeeded).
//! Emitting never fails the action that caused it.

use crate::Manager;
use cm_core::{CrashInfo, NewNotification, Notification, NotificationRepo, Result, ScheduleRunOutcome, notification::SYSTEM_NOTIFY};
use serde::Serialize;
use serde_json::{Value, json};
use std::{collections::HashSet, sync::Arc, sync::Mutex};

/// Attached store + the dedupe keys already seen (so a crash shown by every overview poll costs
/// no database call).
pub struct Notifier {
    repo: Arc<dyn NotificationRepo>,
    seen: Mutex<HashSet<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationList {
    pub items: Vec<Notification>,
    pub unread: u64,
    /// Show a macOS banner for new notifications.
    pub system_notify: bool,
}

/// English title and body of a notification kind (the UI translates `kind` + `params` itself and
/// falls back to these; the macOS banner uses them).
pub fn compose(kind: &str, p: &Value) -> (String, String) {
    let s = |k: &str| p[k].as_str().unwrap_or("").to_string();
    match kind {
        "profile.crashed" => ("Profile crashed".into(), format!("{}: {}", s("profile"), s("detail"))),
        "schedule.failed" => ("Scheduled run failed".into(), format!("{} on {}: {}", s("schedule"), s("profile"), s("message"))),
        "schedule.finished" => ("Scheduled run finished".into(), format!("{} on {}", s("schedule"), s("profile"))),
        "proxy.check_failed" => ("Proxy check failed".into(), format!("{}: {}", s("proxy"), s("error"))),
        "transfer.export_finished" => ("Profile export finished".into(), s("name")),
        "transfer.import_finished" => ("Profile import finished".into(), s("name")),
        "sheets.export_finished" => ("Spreadsheet export finished".into(), s("name")),
        "sheets.import_finished" => ("Spreadsheet import finished".into(), s("name")),
        other => (other.to_string(), String::new()),
    }
}

fn clip(s: &str, max: usize) -> String {
    let one_line: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let t = one_line.trim();
    if t.chars().count() <= max { t.to_string() } else { format!("{}...", t.chars().take(max).collect::<String>()) }
}

/// AppleScript for a banner; text is escaped and shortened, never executed as code.
pub fn banner_script(title: &str, body: &str) -> String {
    let esc = |s: &str| clip(s, 200).replace('\\', "\\\\").replace('"', "\\\"");
    format!("display notification \"{}\" with title \"Camoufox Manager\" subtitle \"{}\"", esc(body), esc(title))
}

fn level_of(kind: &str) -> &'static str {
    if kind.ends_with("failed") || kind.ends_with("crashed") { "error" } else { "info" }
}

impl Manager {
    /// Attach the notification store (binary wiring). Without it every `notify_*` call is a no-op.
    pub fn attach_notifications(&self, repo: Arc<dyn NotificationRepo>) {
        let _ = self.notifier.set(Notifier { repo, seen: Mutex::new(HashSet::new()) });
    }

    pub async fn system_notify(&self) -> bool {
        self.d.settings.get(SYSTEM_NOTIFY).await.ok().flatten().and_then(|v| v.as_bool()).unwrap_or(false)
    }

    pub async fn set_system_notify(&self, on: bool) -> Result<()> {
        self.d.settings.set(SYSTEM_NOTIFY, &json!(on)).await
    }

    /// Record a notification of `kind` with `params` (text fields are shortened). `dedupe`: report once.
    pub async fn notify(&self, kind: &str, params: Value, dedupe: Option<String>) {
        let Some(n) = self.notifier.get() else { return };
        if let Some(k) = &dedupe {
            let mut seen = n.seen.lock().unwrap_or_else(|e| e.into_inner());
            if !seen.insert(k.clone()) {
                return;
            }
            if seen.len() > 2000 {
                seen.clear();
            }
        }
        let mut params = params;
        if let Some(o) = params.as_object_mut() {
            for v in o.values_mut() {
                if let Some(s) = v.as_str() {
                    *v = json!(clip(s, 300));
                }
            }
        }
        let (title, body) = compose(kind, &params);
        let new =
            NewNotification { kind: kind.into(), level: level_of(kind).into(), title: title.clone(), body: body.clone(), params, dedupe };
        match n.repo.add(&new, self.d.clock.now()).await {
            Ok(Some(_)) if self.system_notify().await => banner(&title, &body),
            Ok(_) => {}
            Err(e) => eprintln!("notification not saved: {e:#}"),
        }
    }

    /// A browser died on its own. Reported once per crash (key carries the crash time).
    pub async fn notify_crash(&self, id: &str, name: &str, c: &CrashInfo) {
        self.notify("profile.crashed", json!({ "profile": name, "id": id, "detail": c.detail }), Some(format!("crash:{id}:{}", c.at)))
            .await;
    }

    /// Crash info of a profile the app just noticed is gone.
    pub(crate) async fn notify_crash_of(&self, id: &str) {
        if self.notifier.get().is_none() {
            return;
        }
        if let Some(c) = self.d.engine.crashed(id).await {
            let name = self.d.profiles.get(id).await.ok().flatten().map_or_else(|| id.to_string(), |p| p.name);
            self.notify_crash(id, &name, &c).await;
        }
    }

    /// A schedule run ended: `Failed` and `Ok` are reported, skipped runs are not.
    pub(crate) async fn notify_schedule_run(
        &self,
        s: &cm_core::Schedule,
        profile: Option<&str>,
        outcome: ScheduleRunOutcome,
        message: &str,
    ) {
        let kind = match outcome {
            ScheduleRunOutcome::Failed => "schedule.failed",
            ScheduleRunOutcome::Ok => "schedule.finished",
            _ => return,
        };
        if self.notifier.get().is_none() {
            return;
        }
        let name = match profile {
            Some(id) => self.d.profiles.get(id).await.ok().flatten().map_or_else(|| id.to_string(), |p| p.name),
            None => String::new(),
        };
        self.notify(kind, json!({ "schedule": s.name, "scheduleId": s.id, "profile": name, "message": message }), None).await;
    }

    pub async fn notifications(&self, limit: usize) -> Result<NotificationList> {
        let system_notify = self.system_notify().await;
        let Some(n) = self.notifier.get() else { return Ok(NotificationList { items: vec![], unread: 0, system_notify }) };
        Ok(NotificationList { items: n.repo.list(limit.clamp(1, 200)).await?, unread: n.repo.unread().await?, system_notify })
    }

    pub async fn mark_notifications_read(&self, ids: Option<&[i64]>) -> Result<()> {
        let Some(n) = self.notifier.get() else { return Ok(()) };
        match ids {
            Some(ids) => n.repo.mark_read(ids).await,
            None => n.repo.mark_all_read().await,
        }
    }

    pub async fn clear_notifications(&self) -> Result<()> {
        match self.notifier.get() {
            Some(n) => n.repo.clear().await,
            None => Ok(()),
        }
    }
}

/// Fire-and-forget macOS banner; elsewhere nothing happens.
fn banner(title: &str, body: &str) {
    if !cfg!(target_os = "macos") {
        return;
    }
    let script = banner_script(title, body);
    if let Ok(mut child) = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::manager;
    use cm_store::{Db, SqliteNotifications};

    fn attached() -> Manager {
        let (m, _) = manager(false, 0);
        m.attach_notifications(Arc::new(SqliteNotifications::new(Db::memory().unwrap())));
        m
    }

    #[tokio::test]
    async fn crash_is_reported_once_and_survives_clear() {
        let m = attached();
        let c = CrashInfo { at: 100, detail: "the browser exited".into() };
        m.notify_crash("abc", "Shop", &c).await;
        m.notify_crash("abc", "Shop", &c).await;
        let l = m.notifications(50).await.unwrap();
        assert_eq!((l.items.len(), l.unread), (1, 1));
        assert_eq!(l.items[0].kind, "profile.crashed");
        assert_eq!(l.items[0].level, "error");
        assert_eq!(l.items[0].body, "Shop: the browser exited");
        m.mark_notifications_read(None).await.unwrap();
        assert_eq!(m.notifications(50).await.unwrap().unread, 0);
        m.clear_notifications().await.unwrap();
        m.notify_crash("abc", "Shop", &c).await;
        assert!(m.notifications(50).await.unwrap().items.is_empty(), "same crash does not come back");
        m.notify_crash("abc", "Shop", &CrashInfo { at: 200, detail: "again".into() }).await;
        assert_eq!(m.notifications(50).await.unwrap().items.len(), 1);
    }

    #[tokio::test]
    async fn failed_proxy_check_notifies() {
        let m = attached();
        let px = m
            .deps()
            .proxies
            .insert(&cm_core::NewProxy { name: "EU 1".into(), host: "127.0.0.1".into(), port: 9, ..Default::default() })
            .await
            .unwrap();
        assert!(m.check_proxy(px.id, true).await.is_err());
        let l = m.notifications(10).await.unwrap();
        assert_eq!(l.items[0].kind, "proxy.check_failed");
        assert_eq!(l.items[0].params["proxy"], "EU 1");
        assert!(l.items[0].body.contains("dead proxy"));
    }

    #[tokio::test]
    async fn without_a_store_everything_is_a_noop() {
        let (m, _) = manager(false, 0);
        m.notify("proxy.check_failed", json!({}), None).await;
        let l = m.notifications(10).await.unwrap();
        assert!(l.items.is_empty());
        m.clear_notifications().await.unwrap();
    }

    #[tokio::test]
    async fn system_banner_is_off_by_default_and_toggles() {
        let m = attached();
        assert!(!m.system_notify().await);
        m.set_system_notify(true).await.unwrap();
        assert!(m.notifications(1).await.unwrap().system_notify);
    }

    #[test]
    fn banner_script_is_escaped_and_short() {
        let s = banner_script("T\"itle", "a\"; do shell script \"rm -rf ~\" \\ \nline");
        assert!(s.starts_with("display notification \""));
        assert!(!s.contains('\n'));
        assert!(s.contains("\\\"; do shell script \\\"rm"), "quotes escaped so the text stays a string: {s}");
        let long = banner_script("t", &"x".repeat(1000));
        assert!(long.len() < 400);
    }

    #[test]
    fn titles_in_english() {
        let (t, b) = compose("schedule.failed", &json!({ "schedule": "Warm", "profile": "P1", "message": "boom" }));
        assert_eq!((t.as_str(), b.as_str()), ("Scheduled run failed", "Warm on P1: boom"));
    }
}
