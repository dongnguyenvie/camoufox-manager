//! Migration runner. One SQL file per area in `migrations/`, applied in version order, each in
//! its own transaction, recorded in `schema_migrations`.
//!
//! Reserved files are created empty (comments only) by phase 1. **Empty migrations are skipped
//! and not recorded**, so when an area phase fills its file it still runs on databases created
//! before that phase. Never edit a migration that has shipped with SQL in it — add a new one.
//!
//! | file | owner phase |
//! |---|---|
//! | `0001_core.sql` | 1 Foundation (settings, profiles, proxies base tables) |
//! | `0002_proxies.sql` | 2 Proxy library |
//! | `0003_profiles.sql` | 4 Profile settings + pinned machine |
//! | `0004_org.sql` | 7 Organization (tags, groups, saved views) |
//! | `0005_templates.sql` | 9 Templates + extensions |
//! | `0006_schedules.sql` | 10 Scheduling |
//! | `0007_settings.sql` | 12 Settings / API key |
//! | `0008_journal.sql` | 15 Snapshot apply journal |

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub sql: &'static str,
}

macro_rules! migration {
    ($v:expr, $file:literal) => {
        Migration { version: $v, name: $file, sql: include_str!(concat!("migrations/", $file)) }
    };
}

/// All migrations, in order. Phase 1 lists every reserved file; area phases only edit SQL.
pub const MIGRATIONS: &[Migration] = &[
    migration!(1, "0001_core.sql"),
    migration!(2, "0002_proxies.sql"),
    migration!(3, "0003_profiles.sql"),
    migration!(4, "0004_org.sql"),
    migration!(5, "0005_templates.sql"),
    migration!(6, "0006_schedules.sql"),
    migration!(7, "0007_settings.sql"),
    migration!(8, "0008_journal.sql"),
];

/// Only comments / whitespace?
fn is_empty(sql: &str) -> bool {
    sql.lines().map(str::trim).all(|l| l.is_empty() || l.starts_with("--"))
}

/// Apply pending migrations. Returns the names applied.
pub fn run(conn: &mut Connection) -> Result<Vec<&'static str>> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        )",
    )?;
    let mut applied = Vec::new();
    for m in MIGRATIONS {
        if is_empty(m.sql) {
            continue;
        }
        let done: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)", [m.version], |r| r.get(0))?;
        if done {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(m.sql).with_context(|| format!("migration {} failed", m.name))?;
        tx.execute("INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)", params![m.version, m.name])?;
        tx.commit()?;
        applied.push(m.name);
    }
    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_ordered_and_named() {
        for (i, m) in MIGRATIONS.iter().enumerate() {
            assert_eq!(m.version as usize, i + 1);
            assert!(m.name.starts_with(&format!("{:04}_", m.version)));
        }
    }

    #[test]
    fn runs_once_and_skips_empty() {
        let mut c = Connection::open_in_memory().unwrap();
        let first = run(&mut c).unwrap();
        assert!(first.contains(&"0001_core.sql"));
        assert!(run(&mut c).unwrap().is_empty());
        assert!(is_empty("-- reserved\n\n  -- x\n"));
        assert!(!is_empty("-- x\nCREATE TABLE t (a);"));
    }
}
