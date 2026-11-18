//! Excel / CSV bulk export and import (phase 11).
//!
//! Export: one row per profile, columns [`COLUMNS`]. The proxy cell is the library name (label
//! only, never credentials). Import: rows are matched by `id` (blank id = new profile, new
//! machine); every row is validated; a dry run returns a diff and keeps the uploaded file under a
//! token; apply re-reads that same file, re-diffs against the current state and writes only rows
//! without errors. A column that is absent from the file is never touched; a present column with a
//! blank cell clears that field (os: blank = keep, webrtc: blank = replace).

use crate::{
    Manager,
    profiles::{OsAction, ProfileEdit, new_id, validate_settings},
};
use anyhow::{Context, Result, bail};
use cm_core::{
    CmError, NewProfile, Os, Profile, ProfileSettings, WebRtcMode,
    profile::valid_id,
    sheets::{FieldChange, RowDiff, SheetFormat},
};
use rust_xlsxwriter::{Format, Workbook};
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io::Cursor,
    path::PathBuf,
    time::{Duration, SystemTime},
};

/// Export columns, in order. `status` is informational and ignored on import.
pub const COLUMNS: [&str; 13] =
    ["id", "name", "notes", "os", "group", "tags", "proxy", "timezone", "locale", "webrtc", "window", "start_url", "status"];

/// Most data rows one import may carry.
pub const MAX_ROWS: usize = 20_000;
/// Uploaded sheets older than this are removed.
const TOKEN_TTL: Duration = Duration::from_secs(3600);

// ---------------------------------------------------------------------------------------------
// Parsing: bytes -> table of strings
// ---------------------------------------------------------------------------------------------

/// A sheet reduced to text: normalised header names and `(1-based sheet row, cells)`.
#[derive(Debug, PartialEq)]
pub(crate) struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<(usize, Vec<String>)>,
}

fn header_key(h: &str) -> String {
    let k: String =
        h.trim().trim_start_matches('\u{feff}').to_lowercase().chars().map(|c| if c == ' ' || c == '-' { '_' } else { c }).collect();
    match k.as_str() {
        "profile_id" => "id",
        "tag" => "tags",
        "groups" => "group",
        "proxy_name" => "proxy",
        "tz" => "timezone",
        "starturl" | "start_page" | "url" => "start_url",
        "web_rtc" => "webrtc",
        _ => return k,
    }
    .to_string()
}

fn build_table(grid: Vec<(usize, Vec<String>)>) -> Result<Table> {
    let mut it = grid.into_iter().skip_while(|(_, r)| r.iter().all(|c| c.trim().is_empty()));
    let Some((_, head)) = it.next() else { bail!(CmError::Invalid("the file is empty".into())) };
    let headers: Vec<String> = head.iter().map(|h| header_key(h)).collect();
    if !headers.iter().any(|h| h == "id" || h == "name") {
        bail!(CmError::Invalid("the first row must be a header with at least an id or name column".into()));
    }
    let mut rows = Vec::new();
    for (n, r) in it {
        if r.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        if rows.len() >= MAX_ROWS {
            bail!(CmError::Invalid(format!("more than {MAX_ROWS} rows")));
        }
        rows.push((n, r));
    }
    Ok(Table { headers, rows })
}

fn cell_text(d: &calamine::Data) -> String {
    use calamine::Data;
    match d {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", *f as i64),
        other => other.to_string(),
    }
}

pub(crate) fn parse_xlsx(bytes: &[u8]) -> Result<Table> {
    use calamine::{Reader, open_workbook_auto_from_rs};
    let mut wb =
        open_workbook_auto_from_rs(Cursor::new(bytes.to_vec())).map_err(|e| CmError::Invalid(format!("not a readable xlsx file: {e}")))?;
    let range = wb
        .worksheet_range_at(0)
        .ok_or_else(|| CmError::Invalid("the workbook has no sheet".into()))?
        .map_err(|e| CmError::Invalid(format!("cannot read the first sheet: {e}")))?;
    let (r0, c0) = range.start().map(|(r, c)| (r as usize, c as usize)).unwrap_or((0, 0));
    let grid = range
        .rows()
        .enumerate()
        .map(|(i, row)| {
            let mut cells = vec![String::new(); c0];
            cells.extend(row.iter().map(cell_text));
            (r0 + i + 1, cells)
        })
        .collect();
    build_table(grid)
}

/// Undo [`csv_safe`].
fn csv_unsafe(s: &str) -> &str {
    match s.strip_prefix('\'') {
        Some(rest) if rest.starts_with(['=', '+', '-', '@', '\t', '\r']) => rest,
        _ => s,
    }
}

pub(crate) fn parse_csv(bytes: &[u8]) -> Result<Table> {
    let text = std::str::from_utf8(bytes).map_err(|_| CmError::Invalid("the CSV file must be UTF-8".into()))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let first = text.lines().next().unwrap_or("");
    let delim = b",;\t".iter().copied().max_by_key(|d| first.bytes().filter(|b| b == d).count()).unwrap_or(b',');
    let mut rd = csv::ReaderBuilder::new().has_headers(false).flexible(true).delimiter(delim).from_reader(text.as_bytes());
    let mut grid = Vec::new();
    for rec in rd.records() {
        let rec = rec.map_err(|e| CmError::Invalid(format!("CSV error: {e}")))?;
        // The reader folds skipped blank lines into the next record: step over them for the row number.
        let mut at = rec.position().map_or(0, |p| p.byte() as usize);
        while text.as_bytes().get(at).is_some_and(|b| *b == b'\n' || *b == b'\r') {
            at += 1;
        }
        let row = text.as_bytes()[..at].iter().filter(|b| **b == b'\n').count() + 1;
        grid.push((row, rec.iter().map(|c| csv_unsafe(c).to_string()).collect()));
    }
    build_table(grid)
}

pub(crate) fn parse_sheet(bytes: &[u8], fmt: SheetFormat) -> Result<Table> {
    match fmt {
        SheetFormat::Xlsx => parse_xlsx(bytes),
        SheetFormat::Csv => parse_csv(bytes),
    }
}

/// xlsx files are zip archives; anything else is treated as CSV.
pub fn detect_format(bytes: &[u8]) -> SheetFormat {
    if bytes.starts_with(b"PK") { SheetFormat::Xlsx } else { SheetFormat::Csv }
}

// ---------------------------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------------------------

/// Spreadsheet programs run cells starting with these as formulas.
fn csv_safe(s: &str) -> String {
    if s.starts_with(['=', '+', '-', '@', '\t', '\r']) { format!("'{s}") } else { s.to_string() }
}

fn webrtc_code(m: WebRtcMode) -> &'static str {
    match m {
        WebRtcMode::Replace => "replace",
        WebRtcMode::Real => "real",
        WebRtcMode::Forward => "forward",
        WebRtcMode::None => "none",
    }
}

fn parse_webrtc(s: &str) -> Option<WebRtcMode> {
    Some(match s {
        "" | "replace" => WebRtcMode::Replace,
        "real" => WebRtcMode::Real,
        "forward" => WebRtcMode::Forward,
        "none" => WebRtcMode::None,
        _ => return None,
    })
}

fn csv_bytes(rows: &[Vec<String>]) -> Result<Vec<u8>> {
    let mut w = csv::WriterBuilder::new().from_writer(Vec::new());
    w.write_record(COLUMNS)?;
    for r in rows {
        w.write_record(r.iter().map(|c| csv_safe(c)))?;
    }
    let mut out = b"\xEF\xBB\xBF".to_vec(); // BOM so Excel reads UTF-8
    out.extend(w.into_inner().map_err(|e| anyhow::anyhow!("csv: {e}"))?);
    Ok(out)
}

fn xlsx_bytes(rows: &[Vec<String>]) -> Result<Vec<u8>> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("profiles")?;
    let bold = Format::new().set_bold();
    for (c, h) in COLUMNS.iter().enumerate() {
        ws.write_string_with_format(0, c as u16, *h, &bold)?;
    }
    let mut width = COLUMNS.map(|h| h.len());
    for (r, row) in rows.iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            // Always a string cell: ids and names are never turned into numbers or formulas.
            ws.write_string(r as u32 + 1, c as u16, v)?;
            width[c] = width[c].max(v.chars().count().min(60));
        }
    }
    for (c, w) in width.iter().enumerate() {
        ws.set_column_width(c as u16, (*w as f64 + 2.0).min(62.0))?;
    }
    ws.set_freeze_panes(1, 0)?;
    ws.autofilter(0, 0, rows.len() as u32, COLUMNS.len() as u16 - 1)?;
    Ok(wb.save_to_buffer()?)
}

/// Proxy library entry as the sheet sees it.
struct ProxyRef {
    id: i64,
    name: String,
    label: String,
}

impl ProxyRef {
    fn shown(&self) -> String {
        if self.name.is_empty() { self.label.clone() } else { self.name.clone() }
    }
}

/// Lookup tables of the organisation and proxy library, loaded once per export / import.
struct Ctx {
    profiles: Vec<Profile>,
    running: HashSet<String>,
    proxies: Vec<ProxyRef>,
    groups: HashMap<String, i64>,
    group_names: HashMap<i64, String>,
    tags: HashMap<String, i64>,
    tag_names: HashMap<i64, String>,
    profile_group: HashMap<String, i64>,
    profile_tags: HashMap<String, Vec<i64>>,
}

impl Ctx {
    fn group_of(&self, id: &str) -> String {
        self.profile_group.get(id).and_then(|g| self.group_names.get(g)).cloned().unwrap_or_default()
    }

    fn tags_of(&self, id: &str) -> Vec<String> {
        let mut v: Vec<String> = self.profile_tags.get(id).into_iter().flatten().filter_map(|t| self.tag_names.get(t).cloned()).collect();
        v.sort_by_key(|t| t.to_lowercase());
        v
    }

    fn proxy_name(&self, p: &Profile) -> String {
        p.proxy_id.and_then(|id| self.proxies.iter().find(|x| x.id == id)).map(ProxyRef::shown).unwrap_or_default()
    }

    /// Proxy by library name (case-insensitive), else by `host:port`; ambiguous = error.
    fn find_proxy(&self, text: &str) -> Result<&ProxyRef, String> {
        let key = text.to_lowercase();
        let mut found: Vec<&ProxyRef> = self.proxies.iter().filter(|x| x.name.to_lowercase() == key).collect();
        if found.is_empty() {
            found = self.proxies.iter().filter(|x| x.label.to_lowercase() == key).collect();
        }
        match found.as_slice() {
            [one] => Ok(one),
            [] => Err(format!("unknown proxy \"{text}\"")),
            _ => Err(format!("proxy \"{text}\" matches more than one library entry")),
        }
    }
}

fn field_cells(p: &Profile, c: &Ctx) -> Vec<String> {
    let s = &p.settings;
    vec![
        p.id.clone(),
        p.name.clone(),
        p.notes.clone().unwrap_or_default(),
        p.os.code().to_string(),
        c.group_of(&p.id),
        c.tags_of(&p.id).join(", "),
        c.proxy_name(p),
        s.timezone.clone().unwrap_or_default(),
        s.locale.clone().unwrap_or_default(),
        webrtc_code(s.webrtc).to_string(),
        s.window.clone().unwrap_or_default(),
        s.start_url.clone().unwrap_or_default(),
        if c.running.contains(&p.id) { "running" } else { "stopped" }.to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------
// Diff
// ---------------------------------------------------------------------------------------------

/// What a row wants, after validation. `None` = column absent (leave alone).
#[derive(Debug, Default)]
struct Want {
    name: Option<String>,
    notes: Option<String>,
    os: Option<Os>,
    group: Option<String>,
    tags: Option<Vec<String>>,
    proxy: Option<Option<i64>>,
    timezone: Option<String>,
    locale: Option<String>,
    webrtc: Option<WebRtcMode>,
    window: Option<String>,
    start_url: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RowAction {
    Create,
    Update,
    Unchanged,
    Error,
}

/// One planned row: the public diff plus what apply needs.
struct Plan {
    diff: RowDiff,
    action: RowAction,
    name: Option<String>,
    want: Want,
    warnings: Vec<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetRow {
    #[serde(flatten)]
    pub diff: RowDiff,
    pub action: RowAction,
    /// Profile name (new value if it changes) for display.
    pub name: Option<String>,
    /// Stable codes the UI translates, e.g. `os-regenerate`.
    pub warnings: Vec<&'static str>,
}

#[derive(Serialize, Default, Debug, PartialEq)]
pub struct SheetSummary {
    pub create: usize,
    pub update: usize,
    pub unchanged: usize,
    pub errors: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetPreview {
    /// Hand this back to apply: the uploaded file is kept under it for an hour.
    pub token: String,
    pub format: SheetFormat,
    pub summary: SheetSummary,
    pub rows: Vec<SheetRow>,
    /// Groups / tags that apply would create (only with `create_missing`).
    pub new_groups: Vec<String>,
    pub new_tags: Vec<String>,
    /// Header names the file does not use (ignored).
    pub ignored_columns: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowFailure {
    pub row: usize,
    pub error: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetApplyResult {
    pub created: usize,
    pub updated: usize,
    pub unchanged: usize,
    /// Rows skipped because they had errors in the dry run.
    pub skipped: usize,
    /// Rows that failed while writing.
    pub failed: Vec<RowFailure>,
}

fn split_list(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in s.split([',', ';']).map(str::trim).filter(|t| !t.is_empty()) {
        if !out.iter().any(|o| o.eq_ignore_ascii_case(t)) {
            out.push(t.to_string());
        }
    }
    out
}

fn parse_os(s: &str) -> Option<Os> {
    match s.trim().to_lowercase().as_str() {
        "win" | "windows" => Some(Os::Win),
        "mac" | "macos" | "osx" => Some(Os::Mac),
        "lin" | "linux" => Some(Os::Lin),
        _ => None,
    }
}

fn shown(v: &str) -> Option<String> {
    if v.is_empty() { None } else { Some(v.to_string()) }
}

struct RowInput<'a> {
    row: usize,
    cells: &'a HashMap<&'a str, String>,
    cur: Option<&'a Profile>,
    /// Pinned screen `WxH` of the existing profile (for the window-fits-screen check).
    screen: Option<&'a str>,
}

struct Opts<'a> {
    create_missing: bool,
    new_groups: &'a mut Vec<String>,
    new_tags: &'a mut Vec<String>,
}

/// Validate one row and compute its changes.
fn plan_row(inp: RowInput, ctx: &Ctx, opts: &mut Opts) -> Plan {
    let RowInput { row, cells, cur, screen } = inp;
    let mut errors: BTreeMap<String, String> = BTreeMap::new();
    let mut changes: Vec<FieldChange> = Vec::new();
    let mut warnings = Vec::new();
    let mut want = Want::default();
    let creating = cur.is_none();
    let cur_cells: Vec<String> = cur.map(|p| field_cells(p, ctx)).unwrap_or_else(|| vec![String::new(); COLUMNS.len()]);
    let before = |f: &str| cur_cells[COLUMNS.iter().position(|c| *c == f).unwrap_or(0)].clone();
    let push = |changes: &mut Vec<FieldChange>, field: &str, old: &str, new: &str| {
        if old != new {
            changes.push(FieldChange { field: field.into(), before: shown(old), after: shown(new) });
        }
    };

    // name
    if let Some(v) = cells.get("name") {
        let n = v.trim();
        if n.is_empty() || n.chars().count() > 80 {
            errors.insert("name".into(), "name must be 1-80 characters".into());
        } else {
            push(&mut changes, "name", &before("name"), n);
            want.name = Some(n.to_string());
        }
    } else if creating {
        errors.insert("name".into(), "a new profile needs a name".into());
    }
    // notes
    if let Some(v) = cells.get("notes") {
        let n = v.trim();
        if n.chars().count() > 2000 {
            errors.insert("notes".into(), "notes are limited to 2000 characters".into());
        } else {
            push(&mut changes, "notes", before("notes").trim(), n);
            want.notes = Some(n.to_string());
        }
    }
    // os
    match cells.get("os").map(|v| v.trim()) {
        Some(v) if !v.is_empty() => match parse_os(v) {
            None => {
                errors.insert("os".into(), "os must be win, mac or lin".into());
            }
            Some(os) => {
                if cur.is_some_and(|p| p.os != os) {
                    if cur.is_some_and(|p| ctx.running.contains(&p.id)) {
                        errors.insert("os".into(), "stop the profile before changing its OS".into());
                    } else {
                        warnings.push("os-regenerate");
                    }
                }
                push(&mut changes, "os", &before("os"), os.code());
                want.os = Some(os);
            }
        },
        _ if creating => {
            errors.insert("os".into(), "a new profile needs an os (win, mac or lin)".into());
        }
        _ => {}
    }
    // group
    if let Some(v) = cells.get("group") {
        let g = v.trim();
        if !g.is_empty() && !ctx.groups.contains_key(&g.to_lowercase()) {
            if g.chars().count() > 60 {
                errors.insert("group".into(), "group name is longer than 60 characters".into());
            } else if opts.create_missing {
                if !opts.new_groups.iter().any(|x| x.eq_ignore_ascii_case(g)) {
                    opts.new_groups.push(g.to_string());
                }
            } else {
                errors.insert("group".into(), format!("unknown group \"{g}\""));
            }
        }
        // show the canonical spelling of an existing group
        let canon = ctx.groups.get(&g.to_lowercase()).and_then(|id| ctx.group_names.get(id)).cloned().unwrap_or_else(|| g.to_string());
        push(&mut changes, "group", &before("group"), &canon);
        want.group = Some(canon);
    }
    // tags
    if let Some(v) = cells.get("tags") {
        let mut names = Vec::new();
        for t in split_list(v) {
            match ctx.tags.get(&t.to_lowercase()).and_then(|id| ctx.tag_names.get(id)) {
                Some(canon) => names.push(canon.clone()),
                None if t.chars().count() > 60 => {
                    errors.insert("tags".into(), format!("tag \"{t}\" is longer than 60 characters"));
                }
                None if opts.create_missing => {
                    if !opts.new_tags.iter().any(|x| x.eq_ignore_ascii_case(&t)) {
                        opts.new_tags.push(t.clone());
                    }
                    names.push(t);
                }
                None => {
                    errors.insert("tags".into(), format!("unknown tag \"{t}\""));
                }
            }
        }
        names.sort_by_key(|t| t.to_lowercase());
        names.dedup_by_key(|t| t.to_lowercase());
        push(&mut changes, "tags", &before("tags"), &names.join(", "));
        want.tags = Some(names);
    }
    // proxy: a library entry by name. A sheet cannot create one (it has no host or credentials).
    if let Some(v) = cells.get("proxy") {
        let p = v.trim();
        if p.is_empty() {
            push(&mut changes, "proxy", &before("proxy"), "");
            want.proxy = Some(None);
        } else {
            match ctx.find_proxy(p) {
                Ok(px) => {
                    if cur.is_some_and(|c| c.proxy_id != Some(px.id) && ctx.running.contains(&c.id)) {
                        errors.insert("proxy".into(), "stop the profile before changing its proxy".into());
                    }
                    push(&mut changes, "proxy", &before("proxy"), &px.shown());
                    want.proxy = Some(Some(px.id));
                }
                Err(e) => {
                    errors.insert("proxy".into(), e);
                }
            }
        }
    }
    // settings: validated one by one for a precise message, then window against the screen
    let mut one = |field: &str, set: &dyn Fn(&mut ProfileSettings, String), raw: &str| -> Option<ProfileSettings> {
        let mut s = ProfileSettings::default();
        set(&mut s, raw.to_string());
        match validate_settings(s, None) {
            Ok(s) => Some(s),
            Err(e) => {
                errors.insert(field.into(), format!("{e:#}"));
                None
            }
        }
    };
    if let Some(v) = cells.get("timezone")
        && let Some(s) = one("timezone", &|s, v| s.timezone = Some(v), v)
    {
        let n = s.timezone.unwrap_or_default();
        push(&mut changes, "timezone", &before("timezone"), &n);
        want.timezone = Some(n);
    }
    if let Some(v) = cells.get("locale")
        && let Some(s) = one("locale", &|s, v| s.locale = Some(v), v)
    {
        let n = s.locale.unwrap_or_default();
        push(&mut changes, "locale", &before("locale"), &n);
        want.locale = Some(n);
    }
    if let Some(v) = cells.get("window")
        && let Some(s) = one("window", &|s, v| s.window = Some(v), v)
    {
        let n = s.window.unwrap_or_default();
        push(&mut changes, "window", &before("window"), &n);
        want.window = Some(n);
    }
    if let Some(v) = cells.get("start_url")
        && let Some(s) = one("start_url", &|s, v| s.start_url = Some(v), v)
    {
        let n = s.start_url.unwrap_or_default();
        push(&mut changes, "start_url", &before("start_url"), &n);
        want.start_url = Some(n);
    }
    if let Some(v) = cells.get("webrtc") {
        match parse_webrtc(&v.trim().to_lowercase()) {
            Some(m) => {
                push(&mut changes, "webrtc", &before("webrtc"), webrtc_code(m));
                want.webrtc = Some(m);
            }
            None => {
                errors.insert("webrtc".into(), "webrtc must be replace, real, forward or none".into());
            }
        }
    }
    if creating {
        // a new profile starts from the defaults: list only what differs from them
        changes.retain(|c| !(c.field == "webrtc" && c.after.as_deref() == Some("replace")));
    }
    if !errors.contains_key("window")
        && let Some(w) = want.window.as_deref().filter(|w| !w.is_empty())
        && let Some(screen) = cur.and_then(|p| p.settings.screen.as_deref()).or(screen)
    {
        let s = ProfileSettings { window: Some(w.to_string()), screen: Some(screen.to_string()), ..Default::default() };
        if let Err(e) = validate_settings(s, None) {
            errors.insert("window".into(), format!("{e:#}"));
        }
    }

    let action = if !errors.is_empty() {
        RowAction::Error
    } else if creating {
        RowAction::Create
    } else if changes.is_empty() {
        RowAction::Unchanged
    } else {
        RowAction::Update
    };
    let name = want.name.clone().or_else(|| cur.map(|p| p.name.clone()));
    Plan { diff: RowDiff { row, profile_id: cur.map(|p| p.id.clone()), changes, errors }, action, name, want, warnings }
}

struct Planned {
    plans: Vec<Plan>,
    new_groups: Vec<String>,
    new_tags: Vec<String>,
}

/// Plan every row of `table` against the current state. `screens`: profile id -> pinned `WxH`.
fn plan_table(table: &Table, ctx: &Ctx, screens: &HashMap<String, String>, create_missing: bool) -> Planned {
    let by_id: HashMap<&str, &Profile> = ctx.profiles.iter().map(|p| (p.id.as_str(), p)).collect();
    let (mut new_groups, mut new_tags) = (Vec::new(), Vec::new());
    let mut seen: HashSet<String> = HashSet::new();
    let mut plans = Vec::new();
    for (row, vals) in &table.rows {
        let mut cells: HashMap<&str, String> = HashMap::new();
        for (i, h) in table.headers.iter().enumerate() {
            if COLUMNS.contains(&h.as_str()) && h != "status" && !cells.contains_key(h.as_str()) {
                cells.insert(h.as_str(), vals.get(i).cloned().unwrap_or_default());
            }
        }
        let id = cells.get("id").map(|s| s.trim().to_string()).unwrap_or_default();
        let mut early: Option<&str> = None;
        let mut cur: Option<&Profile> = None;
        if !id.is_empty() {
            if !valid_id(&id) {
                early = Some("not a valid profile id");
            } else if !seen.insert(id.clone()) {
                early = Some("the same id appears twice in the file");
            } else {
                match by_id.get(id.as_str()) {
                    Some(p) => cur = Some(p),
                    None => early = Some("no profile with this id (leave the id blank to create a new one)"),
                }
            }
        }
        if let Some(msg) = early {
            plans.push(Plan {
                diff: RowDiff {
                    row: *row,
                    profile_id: Some(id),
                    changes: vec![],
                    errors: BTreeMap::from([("id".to_string(), msg.to_string())]),
                },
                action: RowAction::Error,
                name: cells.get("name").map(|n| n.trim().to_string()),
                want: Want::default(),
                warnings: vec![],
            });
            continue;
        }
        let screen = cur.and_then(|p| screens.get(&p.id)).map(String::as_str);
        let mut opts = Opts { create_missing, new_groups: &mut new_groups, new_tags: &mut new_tags };
        plans.push(plan_row(RowInput { row: *row, cells: &cells, cur, screen }, ctx, &mut opts));
    }
    Planned { plans, new_groups, new_tags }
}

fn summarize(plans: &[Plan]) -> SheetSummary {
    let mut s = SheetSummary::default();
    for p in plans {
        match p.action {
            RowAction::Create => s.create += 1,
            RowAction::Update => s.update += 1,
            RowAction::Unchanged => s.unchanged += 1,
            RowAction::Error => s.errors += 1,
        }
    }
    s
}

// ---------------------------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------------------------

/// A sheet export ready to send.
pub struct SheetFile {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
    pub file_name: String,
}

impl Manager {
    async fn sheet_ctx(&self) -> Result<Ctx> {
        let profiles = self.d.profiles.list().await?;
        let proxies = self.d.proxies.list().await?.into_iter().map(|p| ProxyRef { id: p.id, label: p.label(), name: p.name }).collect();
        let groups = self.d.groups.list().await?;
        let tags = self.d.tags.list().await?;
        let mut profile_tags: HashMap<String, Vec<i64>> = HashMap::new();
        for (p, t) in self.d.tags.all_assignments().await? {
            profile_tags.entry(p).or_default().push(t);
        }
        let running: HashSet<String> = self.running.lock().await.keys().cloned().collect();
        Ok(Ctx {
            profiles,
            running,
            proxies,
            groups: groups.iter().map(|g| (g.name.to_lowercase(), g.id)).collect(),
            group_names: groups.into_iter().map(|g| (g.id, g.name)).collect(),
            tags: tags.iter().map(|t| (t.name.to_lowercase(), t.id)).collect(),
            tag_names: tags.into_iter().map(|t| (t.id, t.name)).collect(),
            profile_group: self.d.groups.all_assignments().await?.into_iter().collect(),
            profile_tags,
        })
    }

    fn screens(&self, ctx: &Ctx) -> HashMap<String, String> {
        ctx.profiles.iter().filter_map(|p| Some((p.id.clone(), self.d.engine.summarize(&p.machine).resolution?))).collect()
    }

    /// Export profiles (all, or only `ids`) as xlsx or csv.
    pub async fn export_sheet(&self, fmt: SheetFormat, ids: Option<&[String]>) -> Result<SheetFile> {
        let ctx = self.sheet_ctx().await?;
        let rows: Vec<Vec<String>> =
            ctx.profiles.iter().filter(|p| ids.is_none_or(|ids| ids.contains(&p.id))).map(|p| field_cells(p, &ctx)).collect();
        let stamp: String = self.d.clock.now_iso().chars().take(10).collect();
        Ok(match fmt {
            SheetFormat::Xlsx => SheetFile {
                bytes: xlsx_bytes(&rows)?,
                content_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                file_name: format!("profiles-{stamp}.xlsx"),
            },
            SheetFormat::Csv => {
                SheetFile { bytes: csv_bytes(&rows)?, content_type: "text/csv; charset=utf-8", file_name: format!("profiles-{stamp}.csv") }
            }
        })
    }

    fn sheet_dir(&self) -> Result<PathBuf> {
        use std::os::unix::fs::PermissionsExt;
        let dir = self.cfg.data_dir.join("tmp");
        fs::create_dir_all(&dir)?;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
        Ok(dir)
    }

    fn sheet_path(&self, token: &str, fmt: SheetFormat) -> Result<PathBuf> {
        if !valid_id(token) {
            bail!(CmError::Invalid("invalid file token".into()));
        }
        let ext = if fmt == SheetFormat::Xlsx { "xlsx" } else { "csv" };
        Ok(self.sheet_dir()?.join(format!("sheet-{token}.{ext}")))
    }

    fn sweep_sheets(&self) {
        let Ok(dir) = self.sheet_dir() else { return };
        for e in fs::read_dir(dir).into_iter().flatten().flatten() {
            let old =
                e.metadata().and_then(|m| m.modified()).is_ok_and(|t| SystemTime::now().duration_since(t).unwrap_or_default() > TOKEN_TTL);
            if old && e.file_name().to_string_lossy().starts_with("sheet-") {
                let _ = fs::remove_file(e.path());
            }
        }
    }

    /// Dry run: parse + validate + diff. Keeps the file under the returned token for [`Self::apply_sheet`].
    pub async fn preview_sheet(&self, bytes: Vec<u8>, fmt: Option<SheetFormat>, create_missing: bool) -> Result<SheetPreview> {
        let fmt = fmt.unwrap_or_else(|| detect_format(&bytes));
        let table = parse_sheet(&bytes, fmt)?;
        let ctx = self.sheet_ctx().await?;
        let planned = plan_table(&table, &ctx, &self.screens(&ctx), create_missing);
        self.sweep_sheets();
        let token = new_id()?;
        fs::write(self.sheet_path(&token, fmt)?, &bytes).context("cannot keep the uploaded file")?;
        Ok(SheetPreview {
            token,
            format: fmt,
            summary: summarize(&planned.plans),
            ignored_columns: table.headers.iter().filter(|h| !COLUMNS.contains(&h.as_str()) && !h.is_empty()).cloned().collect(),
            rows: planned
                .plans
                .into_iter()
                .map(|p| SheetRow { diff: p.diff, action: p.action, name: p.name, warnings: p.warnings })
                .collect(),
            new_groups: planned.new_groups,
            new_tags: planned.new_tags,
        })
    }

    /// Apply the file kept under `token`: re-diffed against the current state, rows with errors skipped.
    pub async fn apply_sheet(&self, token: &str, create_missing: bool) -> Result<SheetApplyResult> {
        if !valid_id(token) {
            bail!(CmError::Invalid("invalid file token".into()));
        }
        let (path, fmt) = [SheetFormat::Xlsx, SheetFormat::Csv]
            .into_iter()
            .filter_map(|f| self.sheet_path(token, f).ok().map(|p| (p, f)))
            .find(|(p, _)| p.is_file())
            .ok_or_else(|| CmError::NotFound("file token (kept for an hour; run the check again)".into()))?;
        let table = parse_sheet(&fs::read(&path)?, fmt)?;
        let mut ctx = self.sheet_ctx().await?;
        let planned = plan_table(&table, &ctx, &self.screens(&ctx), create_missing);
        let mut res = SheetApplyResult { created: 0, updated: 0, unchanged: 0, skipped: 0, failed: vec![] };
        if create_missing {
            for g in planned.new_groups {
                let g = self.create_group(&g, None).await?;
                ctx.group_names.insert(g.id, g.name.clone());
                ctx.groups.insert(g.name.to_lowercase(), g.id);
            }
            for t in planned.new_tags {
                let t = self.create_tag(&t, None).await?;
                ctx.tag_names.insert(t.id, t.name.clone());
                ctx.tags.insert(t.name.to_lowercase(), t.id);
            }
        }
        for plan in planned.plans {
            match plan.action {
                RowAction::Error => res.skipped += 1,
                RowAction::Unchanged => res.unchanged += 1,
                RowAction::Create | RowAction::Update => {
                    let creating = plan.action == RowAction::Create;
                    match self.apply_row(&plan, &ctx).await {
                        Ok(()) if creating => res.created += 1,
                        Ok(()) => res.updated += 1,
                        Err(e) => res.failed.push(RowFailure { row: plan.diff.row, error: format!("{e:#}") }),
                    }
                }
            }
        }
        let _ = fs::remove_file(path);
        Ok(res)
    }

    async fn apply_row(&self, plan: &Plan, ctx: &Ctx) -> Result<()> {
        let w = &plan.want;
        let mut id = plan.diff.profile_id.clone();
        let creating = id.is_none();
        if creating {
            let name = w.name.clone().context("name")?;
            let os = w.os.context("os")?;
            id = Some(self.create_profile(NewProfile { name, os: os.code().into() }).await?.id);
        }
        let id = id.context("id")?;
        let p = self.profile(&id).await?;
        let mut settings = p.settings.clone();
        if let Some(v) = &w.timezone {
            settings.timezone = shown(v);
        }
        if let Some(v) = &w.locale {
            settings.locale = shown(v);
        }
        if let Some(v) = &w.window {
            settings.window = shown(v);
        }
        if let Some(v) = &w.start_url {
            settings.start_url = shown(v);
        }
        if let Some(m) = w.webrtc {
            settings.webrtc = m;
        }
        let os_changed = w.os.is_some_and(|o| o != p.os);
        let notes_changed = w.notes.as_ref().is_some_and(|n| *n != p.notes.clone().unwrap_or_default());
        if os_changed || notes_changed || w.name.as_ref().is_some_and(|n| *n != p.name) || settings != p.settings {
            self.update_profile(
                &id,
                ProfileEdit {
                    name: w.name.clone().unwrap_or(p.name.clone()),
                    notes: w.notes.clone().or(p.notes.clone()),
                    os: w.os.map(|o| o.code().to_string()),
                    os_action: os_changed.then_some(OsAction::Regenerate),
                    settings,
                },
            )
            .await?;
        }
        if let Some(g) = &w.group {
            let gid = if g.is_empty() { None } else { Some(*ctx.groups.get(&g.to_lowercase()).context("group missing")?) };
            if ctx.profile_group.get(&id).copied() != gid {
                self.d.groups.set(std::slice::from_ref(&id), gid).await?;
            }
        }
        if let Some(names) = &w.tags {
            let want: HashSet<i64> = names.iter().filter_map(|n| ctx.tags.get(&n.to_lowercase()).copied()).collect();
            let have: HashSet<i64> = ctx.profile_tags.get(&id).into_iter().flatten().copied().collect();
            for t in want.difference(&have) {
                self.d.tags.add(std::slice::from_ref(&id), *t).await?;
            }
            for t in have.difference(&want) {
                self.d.tags.remove(std::slice::from_ref(&id), *t).await?;
            }
        }
        if let Some(px) = w.proxy
            && px != p.proxy_id
        {
            self.assign(&id, px).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::manager;
    use cm_core::{NewProxy, ProxyScheme};

    fn csv(rows: &[&str]) -> Vec<u8> {
        rows.join("\n").into_bytes()
    }

    #[test]
    fn csv_parses_bom_delimiters_and_row_numbers() {
        let t = parse_csv("\u{feff}ID;Name;Start URL\n\n;x;http://a\n".as_bytes()).unwrap();
        assert_eq!(t.headers, ["id", "name", "start_url"]);
        assert_eq!(t.rows, vec![(3, vec!["".to_string(), "x".into(), "http://a".into()])]);
        assert!(parse_csv(b"foo,bar\n1,2").is_err());
        assert!(parse_csv(b"").is_err());
    }

    #[test]
    fn csv_formula_guard_unescapes() {
        let mut row = vec!["id1".to_string(), "=cmd".into(), "-note".into(), "'quoted".into()];
        row.resize(COLUMNS.len(), String::new());
        let rows = vec![row];
        let t = parse_csv(&csv_bytes(&rows).unwrap()).unwrap();
        assert_eq!(t.rows[0].1[..4], ["id1", "=cmd", "-note", "'quoted"]);
    }

    #[test]
    fn xlsx_round_trip_keeps_text() {
        let rows = vec![vec!["0123".to_string(); COLUMNS.len()]];
        let bytes = xlsx_bytes(&rows).unwrap();
        let t = parse_xlsx(&bytes).unwrap();
        assert_eq!(t.headers, COLUMNS);
        assert_eq!(t.rows, vec![(2, vec!["0123".to_string(); COLUMNS.len()])]);
        assert_eq!(detect_format(&bytes), SheetFormat::Xlsx);
    }

    fn dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("cm-sheets-{}", new_id().unwrap()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[tokio::test]
    async fn export_diff_and_apply() {
        let (mut m, _) = manager(true, 0);
        m.cfg.data_dir = dir();
        let a = m.create_profile(NewProfile { name: "Alpha".into(), os: "win".into() }).await.unwrap().id;
        let b = m.create_profile(NewProfile { name: "Beta".into(), os: "mac".into() }).await.unwrap().id;
        m.create_group("Shops", None).await.unwrap();
        m.create_tag("vip", None).await.unwrap();
        let px = m
            .create_proxy(NewProxy {
                name: "DE one".into(),
                scheme: ProxyScheme::Http,
                host: "1.2.3.4".into(),
                port: 80,
                password: Some("sekret".into()),
                ..Default::default()
            })
            .await
            .unwrap();

        // export never contains credentials
        let ex = m.export_sheet(SheetFormat::Csv, None).await.unwrap();
        let text = String::from_utf8(ex.bytes).unwrap();
        assert!(!text.contains("sekret") && text.contains("Alpha"));

        // unchanged round trip
        let p = m.preview_sheet(text.into_bytes(), None, false).await.unwrap();
        assert_eq!(p.summary, SheetSummary { unchanged: 2, ..Default::default() });

        let file = csv(&[
            "id,name,group,tags,proxy,timezone,webrtc,window",
            &format!("{a},Alpha 2,Shops,vip,DE one,Europe/Berlin,none,1280x720"),
            &format!("{b},Beta,Nope,,,,replace,"),
            ",Gamma,,,,,,",
            "zzz,Bad,,,,,,",
            &format!("{a},Dup,,,,,,"),
        ]);
        let p = m.preview_sheet(file.clone(), Some(SheetFormat::Csv), false).await.unwrap();
        assert_eq!(p.summary, SheetSummary { create: 0, update: 1, unchanged: 0, errors: 4 });
        let r = &p.rows[0];
        assert_eq!(r.action, RowAction::Update);
        let fields: Vec<&str> = r.diff.changes.iter().map(|c| c.field.as_str()).collect();
        assert_eq!(fields, ["name", "group", "tags", "proxy", "timezone", "window", "webrtc"]);
        assert!(p.rows[1].diff.errors.contains_key("group"));
        assert_eq!(p.rows[2].diff.row, 4);
        assert!(p.rows[2].diff.errors.contains_key("os"));
        assert!(p.rows[3].diff.errors.contains_key("id"));
        assert!(p.rows[4].diff.errors["id"].contains("twice"));

        // with create-missing the unknown group becomes valid
        let p = m.preview_sheet(file, None, true).await.unwrap();
        assert_eq!(p.new_groups, ["Nope"]);
        assert_eq!(p.summary.update, 2);

        let res = m.apply_sheet(&p.token, true).await.unwrap();
        assert_eq!((res.updated, res.skipped, res.failed.len()), (2, 3, 0));
        let pa = m.profile(&a).await.unwrap();
        assert_eq!(pa.name, "Alpha 2");
        assert_eq!(pa.proxy_id, Some(px.id));
        assert_eq!(pa.settings.window.as_deref(), Some("1280x720"));
        assert_eq!(pa.settings.webrtc, WebRtcMode::None);
        assert_eq!(m.org_overview().await.unwrap().profile_groups.len(), 2);
        // token is single use
        assert!(m.apply_sheet(&p.token, true).await.is_err());

        // create a new profile + os change on an existing one
        let p = m.preview_sheet(csv(&["id,name,os", ",New one,lin", &format!("{b},Beta,win")]), None, false).await.unwrap();
        assert_eq!(p.rows[0].action, RowAction::Create);
        assert_eq!(p.rows[1].warnings, ["os-regenerate"]);
        let res = m.apply_sheet(&p.token, false).await.unwrap();
        assert_eq!((res.created, res.updated), (1, 1));
        assert_eq!(m.profile(&b).await.unwrap().os, Os::Win);
        assert_eq!(m.d.profiles.list().await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn absent_columns_are_left_alone_and_bad_tokens_rejected() {
        let (mut m, _) = manager(true, 0);
        m.cfg.data_dir = dir();
        let a = m.create_profile(NewProfile { name: "Alpha".into(), os: "win".into() }).await.unwrap().id;
        m.create_group("G", None).await.unwrap();
        let p = m.preview_sheet(csv(&["id,group", &format!("{a},G")]), None, false).await.unwrap();
        m.apply_sheet(&p.token, false).await.unwrap();
        let p = m.preview_sheet(csv(&["id,name", &format!("{a},Renamed")]), None, false).await.unwrap();
        assert_eq!(p.rows[0].diff.changes.len(), 1);
        m.apply_sheet(&p.token, false).await.unwrap();
        assert_eq!(m.org_overview().await.unwrap().profile_groups.len(), 1);
        assert!(m.apply_sheet("../etc", false).await.is_err());
        assert!(m.apply_sheet("abcdef0123", false).await.is_err());
    }
}
