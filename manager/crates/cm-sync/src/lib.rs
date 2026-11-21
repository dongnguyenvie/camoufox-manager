//! Device sync between two Camoufox Manager installs over the user's Tailscale (plan CM-1, D9).
//!
//! Phase 16 (this code): who the devices are and how they trust each other.
//!
//! | module | |
//! |---|---|
//! | [`identity`] | this install's Ed25519 key (`<data_dir>/sync/device.key`, 0600), device id + fingerprint |
//! | [`pairing`] | one-time 10-minute code, `cmsync://` link, pair request/response + transcripts |
//! | [`auth`] | `Authorization: Device <id>:<ts>:<nonce>:<sig>` signing + [`auth::Verifier`] (±60 s, replay cache) |
//! | [`policy`] | source-address policy: loopback + Tailscale ranges only |
//! | [`client`] | [`PeerClient`]: signed HTTP client for one peer, plain-language [`PeerError`]s |
//! | [`hub`] | [`DeviceHub`]: status, reachability, pairing both sides, device list, revoke, `client()` |
//! | [`diff`] | phase 17: manifest entries, per-profile diff, compatibility gate |
//! | [`transfer`] | phase 17: chunked resumable transfer, per-chunk + whole-file SHA-256 |
//! | [`apply`] | phase 17: protocol wire types, apply rules |
//! | [`testutil`] | fake clock + in-memory device repo for tests (also for phase 17) |
//!
//! Phase 17 (sync engine) adds its transfer / diff / apply modules here and its signed routes to the
//! sync listener in `cm-api` (`cm_api::sync`), using [`DeviceHub::client`] to call peers.
//!
//! Security rules: the pairing link carries no long-lived secret; after pairing only signatures
//! authenticate; revocation is checked on every request; keys are never logged; Tailscale config
//! changes only through [`cm_tailscale::Confirmed`]; `tailscale funnel` is never used.

pub mod apply;
pub mod auth;
pub mod client;
pub mod codec;
pub mod diff;
pub mod hub;
pub mod identity;
pub mod pairing;
pub mod policy;
pub mod testutil;
pub mod transfer;

pub use apply::{BeginRequest, BeginResponse, ChunkAck, CommitResponse, DoneRequest, PrepareRequest, PrepareResponse, ProxyInfo};
pub use auth::{AuthError, Incoming, Verifier};
pub use client::{PeerClient, PeerError};
pub use diff::{Action, Blocked, Direction, Env, LinkInfo, ManifestEntry, PeerManifest, Row, Status};
pub use hub::{DEFAULT_SYNC_PORT, DeviceHub, Hello, HubConfig, HubDeps, HubStatus, OfferView, PairRefusal, Reach, ReachMode};
pub use identity::Identity;
pub use pairing::{PairRequest, PairResponse, PairUrl};
pub use reqwest::Method;

/// This machine's display name: macOS ComputerName, else the host name, else "Mac".
pub fn local_device_name() -> String {
    let run = |cmd: &str, args: &[&str]| {
        std::process::Command::new(cmd)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .filter(|s| !s.is_empty())
    };
    let name = run("/usr/sbin/scutil", &["--get", "ComputerName"]).or_else(|| run("hostname", &["-s"])).unwrap_or_else(|| "Mac".into());
    name.chars().filter(|c| !c.is_control()).take(80).collect()
}
