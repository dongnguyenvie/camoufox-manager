//! HTTP client for the running app and one handler per command declared in `spec.rs`.

use super::{CliError, Output, Parsed, spec::Command};
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};
use std::{io::Read, time::Duration};

struct Client {
    base: String,
    port: u16,
    key: Option<String>,
    http: reqwest::Client,
}

fn code_for(status: StatusCode) -> &'static str {
    match status.as_u16() {
        400 => "invalid",
        401 => "invalid_api_key",
        403 => "forbidden",
        404 => "not_found",
        409 => "conflict",
        501 => "not_implemented",
        _ => "error",
    }
}

impl Client {
    fn new(p: &Parsed) -> Result<Self, CliError> {
        let port = match p.one("port").map(str::to_string).or_else(|| std::env::var("CM_PORT").ok()) {
            Some(v) => v.trim().parse().map_err(|_| CliError::usage(format!("`{v}` is not a port")))?,
            None => crate::DEFAULT_PORT,
        };
        let key = p.one("api-key").map(str::to_string).or_else(|| std::env::var("CM_API_KEY").ok()).filter(|k| !k.trim().is_empty());
        let http =
            reqwest::Client::builder().timeout(Duration::from_secs(300)).build().map_err(|e| CliError::new("error", e.to_string()))?;
        Ok(Self { base: format!("http://127.0.0.1:{port}"), port, key, http })
    }

    async fn call(&self, method: Method, path: &str, body: Option<Value>) -> Result<Value, CliError> {
        let mut req = self.http.request(method, format!("{}{path}", self.base));
        if let Some(k) = &self.key {
            req = req.header("X-API-Key", k.trim());
        }
        if let Some(b) = body {
            req = req.json(&b);
        }
        let resp = req.send().await.map_err(|e| {
            if e.is_connect() || e.is_timeout() && e.is_connect() {
                CliError::new(
                    "unreachable",
                    format!(
                        "cannot reach Camoufox Manager at 127.0.0.1:{} - start the app (CM_PORT={} camoufox-manager serve)",
                        self.port, self.port
                    ),
                )
            } else {
                CliError::new("error", e.to_string())
            }
        })?;
        let status = resp.status();
        let v: Value = resp.json().await.unwrap_or(Value::Null);
        if status.is_success() {
            return Ok(v);
        }
        let code = v["code"].as_str().unwrap_or(code_for(status)).to_string();
        let mut message = v["error"].as_str().unwrap_or("request failed").to_string();
        if let Some(rejected) = v["rejected"].as_array() {
            let lines: Vec<String> =
                rejected.iter().map(|r| format!("line {}: {}", r["line"], r["code"].as_str().unwrap_or("invalid"))).collect();
            message = format!("{message} ({})", lines.join(", "));
        }
        Err(CliError { code, message })
    }

    async fn get(&self, path: &str) -> Result<Value, CliError> {
        self.call(Method::GET, path, None).await
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value, CliError> {
        self.call(Method::POST, path, Some(body)).await
    }
}

/// Plain text table: header row + rows, columns padded.
fn table(header: &[&str], rows: &[Vec<String>]) -> String {
    let w: Vec<usize> =
        (0..header.len()).map(|i| rows.iter().map(|r| r[i].chars().count()).chain([header[i].len()]).max().unwrap_or(0)).collect();
    let line = |cells: Vec<&str>| {
        cells.iter().zip(&w).map(|(c, w)| format!("{c:<w$}", w = *w)).collect::<Vec<_>>().join("  ").trim_end().to_string()
    };
    let mut out = vec![line(header.to_vec())];
    out.extend(rows.iter().map(|r| line(r.iter().map(String::as_str).collect())));
    out.join("\n")
}

fn s(v: &Value) -> String {
    v.as_str().map(str::to_string).unwrap_or_else(|| if v.is_null() { "-".into() } else { v.to_string() })
}

fn proxy_label(p: &Value) -> String {
    format!("{}://{}", s(&p["scheme"]), s(&p["label"]))
}

/// Compact profile record for lists.
fn profile_row(p: &Value) -> Value {
    json!({
        "id": p["id"], "name": p["name"], "os": p["os"],
        "proxy": p["assigned"]["name"], "running": !p["running"].is_null(),
    })
}

pub async fn run(cmd: &Command, args: &[&str], p: &Parsed) -> Result<Output, CliError> {
    let c = Client::new(p)?;
    match cmd.name {
        "status" => status(&c).await,
        "doctor" => doctor(&c).await,
        "profile list" => {
            let ov = c.get("/api/overview").await?;
            let rows: Vec<Value> = ov["profiles"].as_array().map(|a| a.iter().map(profile_row).collect()).unwrap_or_default();
            let text = if rows.is_empty() {
                "no profiles".into()
            } else {
                let t: Vec<Vec<String>> = rows
                    .iter()
                    .map(|r| {
                        vec![
                            s(&r["id"]),
                            s(&r["name"]),
                            s(&r["os"]),
                            s(&r["proxy"]),
                            if r["running"] == true { "running" } else { "closed" }.into(),
                        ]
                    })
                    .collect();
                table(&["ID", "NAME", "OS", "PROXY", "STATE"], &t)
            };
            Ok(Output { json: json!({ "profiles": rows }), text })
        }
        "profile create" => {
            let name = p.one("name").unwrap_or_default().trim();
            let v = c.post("/api/profiles", json!({ "name": name, "os": p.one("os").unwrap_or("win") })).await?;
            let text = format!("created {} ({})", s(&v["id"]), s(&v["name"]));
            Ok(Output { json: v, text })
        }
        "profile show" => {
            let v = c.get(&format!("/api/profiles/{}", args[0])).await?;
            let text = format!(
                "{} ({})\nid: {}\nos: {}\nstate: {}\nproxy: {}\nuser agent: {}\nresolution: {}\ndata: {}",
                s(&v["name"]),
                s(&v["os"]),
                s(&v["id"]),
                s(&v["os"]),
                if v["running"].is_null() { "closed" } else { "running" },
                if v["assigned"].is_null() {
                    "-".into()
                } else {
                    format!("{} ({})", s(&v["assigned"]["name"]), proxy_label(&v["assigned"]))
                },
                s(&v["fp"]["userAgent"]),
                s(&v["fp"]["resolution"]),
                s(&v["path"]["dir"]),
            );
            Ok(Output { json: v, text })
        }
        "profile start" => {
            let v = c.post(&format!("/api/profiles/{}/start", args[0]), json!({})).await?;
            let out = json!({
                "id": args[0], "wsEndpoint": v["ws"], "protocol": "playwright",
                "port": v["port"], "already": v["already"], "geo": v["geo"],
            });
            let text = format!("{}\n(Playwright: playwright.firefox.connect(\"{}\"))", s(&v["ws"]), s(&v["ws"]));
            Ok(Output { json: out, text })
        }
        "profile stop" => {
            c.post(&format!("/api/profiles/{}/stop", args[0]), json!({})).await?;
            Ok(Output { json: json!({ "ok": true, "id": args[0] }), text: format!("stopped {}", args[0]) })
        }
        "profile delete" => {
            c.call(Method::DELETE, &format!("/api/profiles/{}", args[0]), None).await?;
            Ok(Output { json: json!({ "ok": true, "id": args[0] }), text: format!("deleted {}", args[0]) })
        }
        "proxy list" => {
            let v = c.get("/api/proxies").await?;
            let rows: Vec<Vec<String>> = v["proxies"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|x| {
                            vec![s(&x["id"]), s(&x["name"]), proxy_label(x), s(&x["geo"]["ip"]), s(&x["geo"]["country"]), s(&x["usedBy"])]
                        })
                        .collect()
                })
                .unwrap_or_default();
            let text =
                if rows.is_empty() { "no proxies".into() } else { table(&["ID", "NAME", "PROXY", "EXIT IP", "COUNTRY", "USED BY"], &rows) };
            Ok(Output { json: v, text })
        }
        "proxy import" => {
            let text = match (p.one("file"), p.one("text")) {
                (Some(_), Some(_)) => return Err(CliError::usage("give --file or --text, not both")),
                (Some("-"), None) => {
                    let mut buf = String::new();
                    std::io::stdin().read_to_string(&mut buf).map_err(|e| CliError::new("error", format!("cannot read stdin: {e}")))?;
                    buf
                }
                (Some(f), None) => std::fs::read_to_string(f).map_err(|e| CliError::new("error", format!("cannot read {f}: {e}")))?,
                (None, Some(t)) => t.replace(',', "\n"),
                (None, None) => return Err(CliError::usage("proxy import needs --file <path> (or `-` for stdin) or --text <lines>")),
            };
            let v = c.post("/api/proxies/import", json!({ "text": text })).await?;
            let (n, skipped) = (v["created"].as_array().map_or(0, Vec::len), v["skipped"].as_array().map_or(0, Vec::len));
            Ok(Output { json: v, text: format!("imported {n} proxies, skipped {skipped} duplicates") })
        }
        "proxy check" => {
            let v = c.post(&format!("/api/proxies/{}/check", args[0]), json!({})).await?;
            let text = format!(
                "ok: exit IP {}, {} {}, timezone {}, {} ms",
                s(&v["ip"]),
                s(&v["country"]),
                s(&v["city"]),
                s(&v["timezone"]),
                s(&v["latencyMs"])
            );
            Ok(Output { json: v, text })
        }
        "proxy assign" => {
            let id: i64 = args[0].parse().map_err(|_| CliError::usage(format!("proxy id must be a number, got `{}`", args[0])))?;
            let v = c.post(&format!("/api/proxies/{id}/assign"), json!({ "profileIds": p.list("profile") })).await?;
            let results = v["results"].as_array().cloned().unwrap_or_default();
            let failed: Vec<String> =
                results.iter().filter(|r| r["ok"] != true).map(|r| format!("{} ({})", s(&r["profileId"]), s(&r["reason"]))).collect();
            if !failed.is_empty() && failed.len() == results.len() {
                return Err(CliError::new("conflict", format!("not assigned: {}", failed.join(", "))));
            }
            let text = format!(
                "assigned proxy {id} to {} profile(s){}",
                results.len() - failed.len(),
                if failed.is_empty() { String::new() } else { format!("; skipped: {}", failed.join(", ")) }
            );
            Ok(Output { json: v, text })
        }
        other => Err(CliError::new("error", format!("command `{other}` has no handler"))),
    }
}

async fn status(c: &Client) -> Result<Output, CliError> {
    let (ov, st) = (c.get("/api/overview").await?, c.get("/api/settings").await?);
    let profiles = ov["profiles"].as_array().cloned().unwrap_or_default();
    let running = profiles.iter().filter(|p| !p["running"].is_null()).count();
    let engine = &st["engine"];
    let out = json!({
        "app": { "url": c.base, "reachable": true },
        "engine": { "name": engine["name"], "ready": engine["ready"], "version": engine["version"], "versions": engine["versions"], "hint": engine["hint"] },
        "profiles": { "total": profiles.len(), "running": running },
        "proxies": ov["proxyCount"],
        "headless": st["headless"],
        "requireApiKey": st["requireApiKey"],
    });
    let engine_line = if engine["ready"] == true {
        format!("ready, {}", s(&engine["version"]))
    } else {
        format!("not ready: {}", s(&engine["hint"]["detail"]))
    };
    let text = format!(
        "app: {} (reachable)\nengine: {} {}\nprofiles: {} ({} running)\nproxies: {}",
        c.base,
        s(&engine["name"]),
        engine_line,
        profiles.len(),
        running,
        s(&ov["proxyCount"])
    );
    Ok(Output { json: out, text })
}

/// Placeholder until the full diagnostics (CM-1.19): app reachable + engine ready.
async fn doctor(c: &Client) -> Result<Output, CliError> {
    let st = c.get("/api/settings").await?;
    let engine_ok = st["engine"]["ready"] == true;
    let checks = json!([
        { "name": "app", "ok": true, "detail": c.base },
        { "name": "engine", "ok": engine_ok, "detail": if engine_ok { st["engine"]["version"].clone() } else { st["engine"]["hint"]["detail"].clone() } },
    ]);
    let ok = engine_ok;
    let text = format!(
        "app: ok ({})\nengine: {}\n\nnote: full diagnostics (ports, relay, orphaned processes, data dir) are not implemented yet",
        c.base,
        if engine_ok {
            format!("ok ({})", s(&st["engine"]["version"]))
        } else {
            format!("problem: {}", s(&st["engine"]["hint"]["detail"]))
        }
    );
    Ok(Output { json: json!({ "ok": ok, "checks": checks, "note": "placeholder: full diagnostics arrive in a later release" }), text })
}
