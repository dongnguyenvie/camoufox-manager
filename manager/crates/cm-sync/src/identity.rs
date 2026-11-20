//! This install's device identity: one Ed25519 keypair, created on first use and kept in
//! `<data_dir>/sync/device.key` (32-byte seed as hex, file 0600, dir 0700). The private key never
//! leaves this file and is never logged ([`Identity`]'s `Debug` prints only the public id).

use crate::codec::{hex, random, sha256_hex, unhex};
use anyhow::{Context, Result, bail};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use std::{
    fs,
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

pub const KEY_DIR: &str = "sync";
pub const KEY_FILE: &str = "device.key";

pub struct Identity {
    key: SigningKey,
    public_hex: String,
    fingerprint: String,
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Identity").field("id", &self.id()).finish_non_exhaustive()
    }
}

/// `sha256(pubkey)` hex: the fingerprint carried in pairing URLs.
pub fn fingerprint(public_key: &[u8; 32]) -> String {
    sha256_hex(public_key)
}

/// Device id from a public key: first 16 hex chars of its fingerprint.
pub fn device_id(public_key: &[u8; 32]) -> String {
    fingerprint(public_key)[..16].to_owned()
}

/// Parse a 64-hex Ed25519 public key.
pub fn parse_public_key(hex_key: &str) -> Option<VerifyingKey> {
    let b: [u8; 32] = unhex(hex_key)?.try_into().ok()?;
    VerifyingKey::from_bytes(&b).ok()
}

/// Verify `sig` (64 bytes) over `msg` with a 64-hex public key.
pub fn verify(public_hex: &str, msg: &[u8], sig: &[u8]) -> bool {
    let Some(k) = parse_public_key(public_hex) else { return false };
    let Ok(sig) = Signature::from_slice(sig) else { return false };
    k.verify(msg, &sig).is_ok()
}

impl Identity {
    fn from_seed(seed: [u8; 32]) -> Identity {
        let key = SigningKey::from_bytes(&seed);
        let public = key.verifying_key().to_bytes();
        Identity { key, public_hex: hex(&public), fingerprint: fingerprint(&public) }
    }

    /// Fresh random identity (tests; [`Identity::load_or_create`] for the app).
    pub fn generate() -> Result<Identity> {
        let seed: [u8; 32] = random(32)?.try_into().map_err(|_| anyhow::anyhow!("short random read"))?;
        Ok(Identity::from_seed(seed))
    }

    pub fn key_path(data_dir: &Path) -> PathBuf {
        data_dir.join(KEY_DIR).join(KEY_FILE)
    }

    /// Load `<data_dir>/sync/device.key`, or create it (0600) on first use.
    pub fn load_or_create(data_dir: &Path) -> Result<Identity> {
        let dir = data_dir.join(KEY_DIR);
        fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
        let path = dir.join(KEY_FILE);
        match fs::read_to_string(&path) {
            Ok(text) => {
                // Tighten permissions if someone loosened them.
                if fs::metadata(&path)?.permissions().mode() & 0o077 != 0 {
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
                }
                let Some(seed) = unhex(text.trim()).and_then(|b| <[u8; 32]>::try_from(b).ok()) else {
                    bail!("{} is not a device key (expected 64 hex chars)", path.display());
                };
                Ok(Identity::from_seed(seed))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let seed: [u8; 32] = random(32)?.try_into().map_err(|_| anyhow::anyhow!("short random read"))?;
                let mut f = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&path)
                    .with_context(|| format!("cannot create {}", path.display()))?;
                f.write_all(format!("{}\n", hex(&seed)).as_bytes())?;
                f.sync_all()?;
                Ok(Identity::from_seed(seed))
            }
            Err(e) => Err(e).with_context(|| format!("cannot read {}", path.display())),
        }
    }

    pub fn id(&self) -> String {
        self.fingerprint[..16].to_owned()
    }

    pub fn public_hex(&self) -> &str {
        &self.public_hex
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn sign(&self, msg: &[u8]) -> [u8; 64] {
        self.key.sign(msg).to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_with_0600_and_signs() {
        let dir = std::env::temp_dir().join(format!("cm-sync-id-{}", crate::codec::hex(&random(6).unwrap())));
        let a = Identity::load_or_create(&dir).unwrap();
        let b = Identity::load_or_create(&dir).unwrap();
        assert_eq!(a.public_hex(), b.public_hex(), "same key on reload");
        let mode = fs::metadata(Identity::key_path(&dir)).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(a.id(), device_id(&unhex(a.public_hex()).unwrap().try_into().unwrap()));
        let sig = a.sign(b"hello");
        assert!(verify(a.public_hex(), b"hello", &sig));
        assert!(!verify(a.public_hex(), b"hellO", &sig));
        assert!(!format!("{a:?}").contains(a.public_hex()), "debug shows only the id");
        fs::remove_dir_all(&dir).unwrap();
    }
}
