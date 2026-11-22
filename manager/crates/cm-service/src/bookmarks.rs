//! Profile bookmarks (CM-3.4): validation of the list and import from a Netscape `bookmarks.html`
//! file (the export format of Chrome, Firefox, Edge and Safari).
//!
//! Import is a small tolerant tag scanner, not an HTML parser: `<H3>` names a folder, `<DL>` opens
//! and `</DL>` closes it, `<A HREF>` is a bookmark. The managed folder has one level of sub-folders,
//! so nested folders are flattened into a path like `Work / Docs`. Only http(s) links are kept.

use anyhow::Result;
use cm_core::{Bookmark, CmError};

pub const MAX_BOOKMARKS: usize = 1000;

fn invalid<T>(m: impl Into<String>) -> Result<T> {
    Err(CmError::Invalid(m.into()).into())
}

/// Trim, drop empty/duplicate entries, and refuse anything that is not an http(s) URL.
pub fn validate(list: Vec<Bookmark>) -> Result<Vec<Bookmark>> {
    let mut out: Vec<Bookmark> = Vec::new();
    for b in list {
        let url = b.url.trim().to_string();
        if url.is_empty() {
            continue;
        }
        let lower = url.to_ascii_lowercase();
        if url.len() > 2000
            || url.chars().any(|c| c.is_control() || c.is_whitespace())
            || !(lower.starts_with("http://") || lower.starts_with("https://"))
        {
            return invalid(format!("bookmark URL must be http(s)://... without spaces: {}", url.chars().take(80).collect::<String>()));
        }
        let title = b.title.trim().to_string();
        let folder = b.folder.map(|f| f.trim().to_string()).filter(|f| !f.is_empty());
        if title.chars().count() > 300 || title.chars().any(char::is_control) {
            return invalid("bookmark title is limited to 300 characters");
        }
        if folder.as_ref().is_some_and(|f| f.chars().count() > 120 || f.chars().any(char::is_control)) {
            return invalid("bookmark folder name is limited to 120 characters");
        }
        let cand = Bookmark { title, url, folder };
        if !out.iter().any(|o| o.url == cand.url && o.folder == cand.folder) {
            out.push(cand);
        }
    }
    if out.len() > MAX_BOOKMARKS {
        return invalid(format!("at most {MAX_BOOKMARKS} bookmarks"));
    }
    Ok(out)
}

fn decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';').filter(|e| *e <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let ent = &rest[1..end];
        let ch = match ent {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            _ => ent
                .strip_prefix('#')
                .and_then(|n| match n.strip_prefix(['x', 'X']) {
                    Some(h) => u32::from_str_radix(h, 16).ok(),
                    None => n.parse().ok(),
                })
                .and_then(char::from_u32),
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Value of attribute `name` in the inside of a start tag.
fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name) {
        let at = from + i;
        from = at + name.len();
        let before_ok = at == 0 || lower.as_bytes()[at - 1].is_ascii_whitespace();
        let after = tag[from..].trim_start();
        if !before_ok || !after.starts_with('=') {
            continue;
        }
        let v = after[1..].trim_start();
        return Some(match v.chars().next() {
            Some(q @ ('"' | '\'')) => v[1..].split(q).next().unwrap_or("").to_string(),
            _ => v.split(|c: char| c.is_whitespace() || c == '>').next().unwrap_or("").to_string(),
        });
    }
    None
}

/// Bookmarks of a Netscape `bookmarks.html`, in file order.
pub fn parse_netscape(html: &str) -> Vec<Bookmark> {
    let lower = html.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut stack: Vec<Option<String>> = Vec::new(); // folder name per open <DL>
    let mut pending: Option<String> = None; // <H3> seen, its <DL> not yet
    let mut pos = 0;
    while let Some(lt) = lower[pos..].find('<') {
        let start = pos + lt;
        let Some(gt) = lower[start..].find('>') else { break };
        let end = start + gt;
        let inner = &html[start + 1..end];
        let name = lower[start + 1..end].split(|c: char| c.is_whitespace() || c == '/').find(|s| !s.is_empty()).unwrap_or("");
        let closing = lower[start + 1..].starts_with('/');
        pos = end + 1;
        match (name, closing) {
            ("h3", false) => {
                let close = lower[pos..].find("</h3").map_or(html.len(), |c| pos + c);
                pending = Some(decode(html[pos..close].trim()));
                pos = close;
            }
            ("dl", false) => stack.push(pending.take()),
            ("dl", true) => {
                stack.pop();
            }
            ("a", false) => {
                let close = lower[pos..].find("</a").map_or(html.len(), |c| pos + c);
                let title = decode(html[pos..close].trim());
                pos = close;
                if let Some(url) = attr(inner, "href").map(|u| decode(u.trim())) {
                    let l = url.to_ascii_lowercase();
                    if l.starts_with("http://") || l.starts_with("https://") {
                        let path: Vec<&str> = stack.iter().flatten().map(String::as_str).filter(|s| !s.is_empty()).collect();
                        let folder = (!path.is_empty()).then(|| path.join(" / "));
                        out.push(Bookmark { title, url, folder });
                    }
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bm(title: &str, url: &str, folder: Option<&str>) -> Bookmark {
        Bookmark { title: title.into(), url: url.into(), folder: folder.map(String::from) }
    }

    #[test]
    fn parses_netscape_export() {
        let html = r#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<META HTTP-EQUIV="Content-Type" CONTENT="text/html; charset=UTF-8">
<TITLE>Bookmarks</TITLE><H1>Bookmarks</H1>
<DL><p>
  <DT><A HREF="https://top.test/?a=1&amp;b=2" ADD_DATE="1">Top &amp; Co</A>
  <DT><H3 ADD_DATE="1" PERSONAL_TOOLBAR_FOLDER="true">Bookmarks bar</H3>
  <DL><p>
    <DT><A HREF="https://bar.test/">Bar</A>
    <DT><H3>Docs</H3>
    <DL><p>
      <DT><A href='http://deep.test/x'>Deep</A>
    </DL><p>
    <DT><A HREF="javascript:alert(1)">Bad</A>
  </DL><p>
  <DT><A HREF="https://after.test/">After</A>
</DL><p>"#;
        let l = parse_netscape(html);
        assert_eq!(
            l,
            vec![
                bm("Top & Co", "https://top.test/?a=1&b=2", None),
                bm("Bar", "https://bar.test/", Some("Bookmarks bar")),
                bm("Deep", "http://deep.test/x", Some("Bookmarks bar / Docs")),
                bm("After", "https://after.test/", None),
            ]
        );
    }

    #[test]
    fn validates_list() {
        let ok = validate(vec![bm(" A ", " https://a.test/ ", Some(" ")), bm("dup", "https://a.test/", None), bm("", "", None)]).unwrap();
        assert_eq!(ok, vec![bm("A", "https://a.test/", None)]);
        assert!(validate(vec![bm("x", "javascript:alert(1)", None)]).is_err());
        assert!(validate(vec![bm("x", "https://a b.test/", None)]).is_err());
        assert!(validate((0..1001).map(|i| bm("t", &format!("https://a.test/{i}"), None)).collect()).is_err());
    }
}
