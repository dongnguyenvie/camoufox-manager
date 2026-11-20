//! Read-only database health for the doctor (phase 19).

use crate::Db;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DbHealth {
    /// Rows of `PRAGMA quick_check`: `["ok"]` when healthy, else the problems found.
    pub integrity: Vec<String>,
    /// `None` = a write transaction could be opened; `Some(reason)` otherwise (read-only file, locked).
    pub write_error: Option<String>,
}

impl DbHealth {
    pub fn intact(&self) -> bool {
        self.integrity.len() == 1 && self.integrity[0] == "ok"
    }
}

impl Db {
    /// `PRAGMA quick_check` plus a write-lock probe (`BEGIN IMMEDIATE` then `ROLLBACK`: nothing is written).
    pub fn health(&self) -> DbHealth {
        let mut h = DbHealth::default();
        let r = self.with(|c| {
            let mut st = c.prepare("PRAGMA quick_check")?;
            let rows: Vec<String> = st.query_map([], |r| r.get(0))?.collect::<std::result::Result<_, _>>()?;
            h.integrity = rows;
            Ok(())
        });
        if let Err(e) = r {
            h.integrity = vec![format!("{e:#}")];
        }
        h.write_error = self
            .with(|c| {
                c.execute_batch("BEGIN IMMEDIATE; ROLLBACK;")?;
                Ok(())
            })
            .err()
            .map(|e| format!("{e:#}"));
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_memory_db() {
        let h = Db::memory().unwrap().health();
        assert!(h.intact(), "{h:?}");
        assert_eq!(h.write_error, None);
    }

    #[test]
    fn corrupt_file_is_reported() {
        let dir = std::env::temp_dir().join(format!("cm-health-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let db = Db::open(&dir).unwrap();
        db.with(|c| {
            c.execute_batch("CREATE TABLE junk (a INTEGER, b TEXT); CREATE INDEX junk_b ON junk (b);")?;
            for i in 0..2000 {
                c.execute("INSERT INTO junk VALUES (?1, ?2)", rusqlite::params![i, format!("value-{i}")])?;
            }
            c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
            Ok(())
        })
        .unwrap();
        let path = dir.join(crate::DB_FILE);
        drop(db);
        let mut bytes = std::fs::read(&path).unwrap();
        // Scribble over most pages after the first quarter (never page 1: the file must still open),
        // so the damage always hits live b-tree pages whatever the migrations' page layout is.
        let (from, to) = (bytes.len() / 4, bytes.len() - 4096);
        for b in &mut bytes[from..to] {
            *b = 0xA5;
        }
        std::fs::write(&path, bytes).unwrap();
        let _ = std::fs::remove_file(dir.join(format!("{}-wal", crate::DB_FILE)));
        let _ = std::fs::remove_file(dir.join(format!("{}-shm", crate::DB_FILE)));
        let conn = rusqlite::Connection::open(&path).unwrap();
        let db = Db { conn: std::sync::Arc::new(std::sync::Mutex::new(conn)), path: Some(path) };
        assert!(!db.health().intact());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
