//! Wire types of the sync protocol and the rules for applying a received snapshot (phase 17).
//!
//! Peer routes (all signed, see `cm_api::sync_peer`), `v1`:
//!
//! | route | |
//! |---|---|
//! | `GET /sync/v1/profiles` | [`crate::PeerManifest`]: this device's profiles (hash, size, closed, link to the caller) |
//! | `POST /sync/v1/transfer/prepare` | [`PrepareRequest`] → [`PrepareResponse`]: export a closed profile for the caller to pull |
//! | `GET /sync/v1/transfer/{id}/chunk/{n}` | raw bytes + `x-chunk-sha256` |
//! | `POST /sync/v1/transfer/{id}/done` | [`DoneRequest`]: caller applied it; the source records the link and drops the export |
//! | `POST /sync/v1/transfer/{id}/cancel` | drop the export |
//! | `POST /sync/v1/inbox/begin` | [`BeginRequest`] → [`BeginResponse`]: the caller wants to push a profile here |
//! | `GET /sync/v1/inbox/{id}` | [`BeginResponse`] (confirmed offset) |
//! | `PUT /sync/v1/inbox/{id}/chunk/{n}` | raw bytes + `x-chunk-sha256` → [`ChunkAck`] |
//! | `POST /sync/v1/inbox/{id}/commit` | verify the whole file, apply through the journal → [`CommitResponse`] |
//! | `POST /sync/v1/inbox/{id}/cancel` | drop the partial file |
//!
//! Apply rules: [`crate::Action::Create`] imports as a new profile (keeping the id when free),
//! [`crate::Action::Replace`] overwrites the linked profile through the phase 15 journal (the local
//! id, name, group, schedules and proxy stay), [`crate::Action::Copy`] imports as a separate
//! profile with a new id and a name that says where it came from. Failures and cancellation never
//! touch the previous profile: the data is staged and swapped atomically with a backup.

use crate::diff::{Action, Env};
use serde::{Deserialize, Serialize};

/// A proxy attached to a transfer when the user opted in. The password is only present when they
/// explicitly ticked "include the password".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyInfo {
    pub name: String,
    pub scheme: String,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareRequest {
    pub profile_id: String,
    /// The caller's environment: the source refuses when it does not match its own.
    pub env: Env,
    /// Resume an earlier export of the same profile when it is still valid.
    #[serde(default)]
    pub resume: Option<String>,
    #[serde(default)]
    pub include_proxy: bool,
    #[serde(default)]
    pub include_proxy_password: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareResponse {
    pub transfer_id: String,
    pub name: String,
    pub size: u64,
    pub sha256: String,
    pub chunk: usize,
    pub content_hash: String,
    pub env: Env,
    #[serde(default)]
    pub proxy: Option<ProxyInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoneRequest {
    /// The profile the data ended up in on the caller (`None` for a copy: no link is recorded).
    #[serde(default)]
    pub result_profile_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginRequest {
    /// Resume token: the id of an earlier begin with the same file, if the sender still has it.
    #[serde(default)]
    pub transfer_id: Option<String>,
    pub profile_id: String,
    pub name: String,
    pub size: u64,
    pub sha256: String,
    pub chunk: usize,
    pub content_hash: String,
    pub env: Env,
    pub action: Action,
    /// Profile to replace on this device (for [`Action::Replace`]).
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub proxy: Option<ProxyInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginResponse {
    pub transfer_id: String,
    /// Bytes already verified here: continue from this offset.
    pub received: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkAck {
    pub received: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitResponse {
    pub profile_id: String,
    pub name: String,
    pub kept_id: bool,
    /// Content hash of the applied profile on this device.
    pub content_hash: String,
    /// Something that did not work but did not stop the apply (e.g. the proxy could not be set).
    #[serde(default)]
    pub warning: Option<String>,
}

/// Name of a copy made from another device's profile (at most 80 characters).
pub fn copy_name(name: &str, device: &str) -> String {
    let full = format!("{name} (from {device})");
    if full.chars().count() <= 80 {
        return full;
    }
    let suffix = format!(" (from {device})");
    let keep = 80usize.saturating_sub(suffix.chars().count()).max(1);
    let head: String = name.chars().take(keep).collect();
    let out = format!("{head}{suffix}");
    out.chars().take(80).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_names_fit_the_limit() {
        assert_eq!(copy_name("Shop", "Mac mini"), "Shop (from Mac mini)");
        let long = "x".repeat(120);
        assert!(copy_name(&long, "Mac mini").chars().count() <= 80);
        assert!(copy_name(&long, &"d".repeat(200)).chars().count() <= 80);
    }

    #[test]
    fn password_is_absent_by_default() {
        let p: ProxyInfo = serde_json::from_str(r#"{"name":"a","scheme":"http","host":"h","port":1,"username":"u"}"#).unwrap();
        assert!(p.password.is_none());
    }
}
