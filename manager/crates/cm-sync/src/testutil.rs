//! Test doubles shared with other crates' tests (phase 17): a settable clock, an in-memory
//! [`DeviceRepo`], and a [`Device`] row for an [`Identity`].

use crate::identity::Identity;
use anyhow::Result;
use async_trait::async_trait;
use cm_core::{Clock, Device, DeviceRepo};
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

pub struct FakeClock(AtomicU64);

impl FakeClock {
    pub fn new(t: u64) -> FakeClock {
        FakeClock(AtomicU64::new(t))
    }
    pub fn set(&self, t: u64) {
        self.0.store(t, Ordering::SeqCst)
    }
    pub fn advance(&self, s: u64) {
        self.0.fetch_add(s, Ordering::SeqCst);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Default)]
pub struct MemDevices(Mutex<HashMap<String, Device>>);

impl MemDevices {
    pub fn put(&self, d: Device) {
        self.0.lock().unwrap().insert(d.id.clone(), d);
    }
    pub fn get_sync(&self, id: &str) -> Option<Device> {
        self.0.lock().unwrap().get(id).cloned()
    }
    pub fn revoke_sync(&self, id: &str) {
        if let Some(d) = self.0.lock().unwrap().get_mut(id) {
            d.revoked_at = Some("2026-01-01T00:00:00Z".into());
        }
    }
}

#[async_trait]
impl DeviceRepo for MemDevices {
    async fn upsert(&self, d: &Device) -> Result<()> {
        self.put(d.clone());
        Ok(())
    }
    async fn get(&self, id: &str) -> Result<Option<Device>> {
        Ok(self.get_sync(id))
    }
    async fn list(&self) -> Result<Vec<Device>> {
        let mut v: Vec<Device> = self.0.lock().unwrap().values().cloned().collect();
        v.sort_by(|a, b| b.paired_at.cmp(&a.paired_at));
        Ok(v)
    }
    async fn touch(&self, id: &str, at: &str, addr: Option<&str>) -> Result<()> {
        if let Some(d) = self.0.lock().unwrap().get_mut(id) {
            d.last_seen_at = Some(at.into());
            if let Some(a) = addr {
                d.last_seen_addr = Some(a.into());
            }
        }
        Ok(())
    }
    async fn revoke(&self, id: &str, at: &str) -> Result<bool> {
        let mut m = self.0.lock().unwrap();
        Ok(match m.get_mut(id) {
            Some(d) if d.revoked_at.is_none() => {
                d.revoked_at = Some(at.into());
                true
            }
            _ => false,
        })
    }
    async fn delete(&self, id: &str) -> Result<bool> {
        Ok(self.0.lock().unwrap().remove(id).is_some())
    }
}

/// A paired, non-revoked row for `who`.
pub fn device_for(who: &Identity) -> Device {
    Device {
        id: who.id(),
        name: "peer".into(),
        public_key: who.public_hex().into(),
        url: None,
        tailscale_login: None,
        paired_at: "2026-01-01T00:00:00Z".into(),
        last_seen_at: None,
        last_seen_addr: None,
        revoked_at: None,
    }
}
