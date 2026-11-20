//! Paired devices (D9, phase 16). A device is another Camoufox Manager install that holds an
//! Ed25519 key; after pairing, only signatures made with that key authenticate it. Revoking is
//! immediate: the verifier reads the row on every request.

use crate::Result;
use async_trait::async_trait;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    /// `sha256(public key)` hex, first 16 chars. Stable for the life of an install.
    pub id: String,
    /// Name the other side announced (its computer name), editable later.
    pub name: String,
    /// Ed25519 public key, 64 hex chars.
    pub public_key: String,
    /// Base URL of its sync listener (`https://mac.tail1234.ts.net:8443`), when known.
    pub url: Option<String>,
    /// `Tailscale-User-Login` seen at pairing (defence in depth), if it came through `serve`.
    pub tailscale_login: Option<String>,
    pub paired_at: String,
    pub last_seen_at: Option<String>,
    /// Source address of the last authenticated request.
    pub last_seen_addr: Option<String>,
    pub revoked_at: Option<String>,
}

#[async_trait]
pub trait DeviceRepo: Send + Sync {
    /// Insert, or replace an existing row with the same id (re-pairing clears `revoked_at`).
    async fn upsert(&self, d: &Device) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<Device>>;
    /// Newest pairing first, revoked included.
    async fn list(&self) -> Result<Vec<Device>>;
    async fn touch(&self, id: &str, at: &str, addr: Option<&str>) -> Result<()>;
    /// `false` when unknown or already revoked.
    async fn revoke(&self, id: &str, at: &str) -> Result<bool>;
    /// Remove the row entirely; `false` when unknown.
    async fn delete(&self, id: &str) -> Result<bool>;
}
