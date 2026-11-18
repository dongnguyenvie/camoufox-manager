//! Agent lease (phase 8): an agent reserves a profile for `ttl_secs` so two agents (or an agent
//! and a scheduled run / the UI) never drive the same browser.
//!
//! - One lease per profile; another holder asking for it gets 409. The same holder asking again
//!   renews (idempotent).
//! - While leased, `start` by anyone but the holder is refused (409). Stopping releases the lease;
//!   it also ends by itself when the TTL passes (checked lazily, no background task).
//! - Leases live in memory: they end when the app exits (the browsers stop then too).

use crate::Manager;
use anyhow::Result;
use cm_core::CmError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MAX_TTL_SECS: u64 = 24 * 3600;

#[derive(Clone, Debug)]
pub(crate) struct Lease {
    holder: String,
    acquired_at: u64,
    expires_at: u64,
}

/// What overview and the lease endpoints show.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LeaseView {
    pub holder: String,
    pub acquired_at: u64,
    pub expires_at: u64,
}

impl Lease {
    fn view(&self) -> LeaseView {
        LeaseView { holder: self.holder.clone(), acquired_at: self.acquired_at, expires_at: self.expires_at }
    }
}

fn check_holder(holder: &str) -> Result<String> {
    let h = holder.trim();
    if h.is_empty() || h.chars().count() > 64 || h.chars().any(char::is_control) {
        return Err(CmError::Invalid("holder must be 1-64 characters".into()).into());
    }
    Ok(h.to_string())
}

fn check_ttl(ttl_secs: u64) -> Result<u64> {
    if !(1..=MAX_TTL_SECS).contains(&ttl_secs) {
        return Err(CmError::Invalid(format!("ttlSecs must be 1-{MAX_TTL_SECS}")).into());
    }
    Ok(ttl_secs)
}

impl Manager {
    /// Live lease of a profile (an expired one is dropped here).
    pub fn lease_of(&self, id: &str) -> Option<LeaseView> {
        let now = self.d.clock.now();
        let mut map = self.leases.lock().unwrap();
        if map.get(id).is_some_and(|l| l.expires_at <= now) {
            map.remove(id);
        }
        map.get(id).map(Lease::view)
    }

    /// Snapshot for the overview.
    pub(crate) fn leases_snapshot(&self) -> HashMap<String, LeaseView> {
        let now = self.d.clock.now();
        let mut map = self.leases.lock().unwrap();
        map.retain(|_, l| l.expires_at > now);
        map.iter().map(|(k, l)| (k.clone(), l.view())).collect()
    }

    pub async fn lease_acquire(&self, id: &str, holder: &str, ttl_secs: u64) -> Result<LeaseView> {
        let (holder, ttl) = (check_holder(holder)?, check_ttl(ttl_secs)?);
        self.profile(id).await?;
        let now = self.d.clock.now();
        let mut map = self.leases.lock().unwrap();
        map.retain(|_, l| l.expires_at > now);
        if let Some(l) = map.get(id)
            && l.holder != holder
        {
            return Err(CmError::Conflict(format!("profile is leased by {} for another {}s", l.holder, l.expires_at - now)).into());
        }
        let acquired_at = map.get(id).map_or(now, |l| l.acquired_at);
        let lease = Lease { holder, acquired_at, expires_at: now + ttl };
        let view = lease.view();
        map.insert(id.to_string(), lease);
        Ok(view)
    }

    /// Extend the holder's lease to `now + ttl`.
    pub async fn lease_renew(&self, id: &str, holder: &str, ttl_secs: u64) -> Result<LeaseView> {
        let (holder, ttl) = (check_holder(holder)?, check_ttl(ttl_secs)?);
        let Some(l) = self.lease_of(id) else { return Err(CmError::NotFound(format!("lease of profile {id}")).into()) };
        if l.holder != holder {
            return Err(CmError::Conflict(format!("profile is leased by {}", l.holder)).into());
        }
        self.lease_acquire(id, &holder, ttl).await
    }

    /// Release. With `holder` it must match (409 otherwise); `None` releases whatever lease exists
    /// (stop, delete). No lease is not an error.
    pub async fn lease_release(&self, id: &str, holder: Option<&str>) -> Result<()> {
        if let (Some(l), Some(h)) = (self.lease_of(id), holder)
            && l.holder != h.trim()
        {
            return Err(CmError::Conflict(format!("profile is leased by {}", l.holder)).into());
        }
        self.leases.lock().unwrap().remove(id);
        Ok(())
    }

    /// Refuse a start by anyone but the lease holder.
    pub(crate) fn lease_check_start(&self, id: &str, holder: Option<&str>) -> Result<()> {
        match (self.lease_of(id), holder.map(str::trim)) {
            (Some(l), Some(h)) if l.holder == h => Ok(()),
            (Some(l), _) => Err(CmError::Conflict(format!("profile is leased by {} until it is released or expires", l.holder)).into()),
            (None, _) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::manager;
    use cm_core::{CmError, NewProfile};

    fn is_conflict(e: &anyhow::Error) -> bool {
        matches!(e.downcast_ref::<CmError>(), Some(CmError::Conflict(_)))
    }

    #[tokio::test]
    async fn second_holder_and_non_holder_start_conflict() {
        let (m, _) = manager(true, 0);
        let p = m.create_profile(NewProfile { name: "a".into(), os: "mac".into() }).await.unwrap();
        m.lease_acquire(&p.id, "agent-1", 60).await.unwrap();
        assert!(is_conflict(&m.lease_acquire(&p.id, "agent-2", 60).await.unwrap_err()));
        assert!(m.lease_acquire(&p.id, "agent-1", 120).await.is_ok(), "same holder renews");
        assert!(is_conflict(&m.start(&p.id).await.err().unwrap()));
        assert!(is_conflict(&m.start_as(&p.id, Some("agent-2")).await.err().unwrap()));
        assert!(m.start_as(&p.id, Some("agent-1")).await.is_ok());
        assert!(is_conflict(&m.lease_release(&p.id, Some("agent-2")).await.unwrap_err()));
        assert_eq!(m.overview().await.profiles[0].lease.as_ref().unwrap().holder, "agent-1");
        m.stop(&p.id).await.unwrap();
        assert!(m.lease_of(&p.id).is_none(), "stop releases the lease");
        assert!(m.start(&p.id).await.is_ok());
    }

    #[tokio::test]
    async fn validation() {
        let (m, _) = manager(true, 0);
        let p = m.create_profile(NewProfile { name: "a".into(), os: "mac".into() }).await.unwrap();
        assert!(m.lease_acquire(&p.id, " ", 60).await.is_err());
        assert!(m.lease_acquire(&p.id, "x", 0).await.is_err());
        assert!(m.lease_renew(&p.id, "x", 60).await.is_err(), "nothing to renew");
    }
}
