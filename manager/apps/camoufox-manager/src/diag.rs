//! Doctor probes (phase 19): the read-only looks that need the engine and store crates.

use cm_service::doctor::{DbProbe, HelperProbe, OrphanProbe, Probes};
use cm_store::Db;
use std::path::PathBuf;

pub struct AppProbes {
    /// `<data_dir>/camoufox`
    pub engine_root: PathBuf,
    pub db: Db,
}

impl Probes for AppProbes {
    fn helper(&self) -> Option<HelperProbe> {
        let s = cm_engine_camoufox::helper_state(&self.engine_root);
        Some(HelperProbe { missing: s.missing, outdated: s.outdated, total: s.total })
    }

    fn db(&self) -> Option<DbProbe> {
        let h = self.db.health();
        Some(DbProbe { integrity: h.integrity, write_error: h.write_error })
    }

    fn orphans(&self) -> Vec<OrphanProbe> {
        cm_engine_camoufox::find_orphans(&self.engine_root)
            .into_iter()
            .map(|o| OrphanProbe { profile: o.profile, pids: o.pids, stale_lock: o.stale_lock })
            .collect()
    }
}
