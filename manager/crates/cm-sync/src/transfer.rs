//! Chunked, resumable file transfer with SHA-256 per chunk and for the whole file (phase 17).
//!
//! The unit is the phase 6 snapshot zip. The sender cuts it into `chunk`-byte pieces (4 MiB by
//! default, never more than [`MAX_CHUNK`]); every piece travels with its own SHA-256 and is only
//! appended to the receiver's `.part` file after it verified. The receiver's confirmed offset is
//! therefore always the length of verified data: after a crash, a lost connection or a restart of
//! either side, the transfer continues from that offset ([`PartFile::open`] drops a torn tail).
//! The whole file is hashed again before anyone may apply it ([`PartFile::finish`]).
//!
//! The network is hidden behind [`ChunkSource`] (receiver pulls) and [`ChunkSink`] (sender pushes)
//! so the loops are tested without a server.

use crate::client::PeerError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

/// Default chunk size.
pub const CHUNK_SIZE: usize = 4 * 1024 * 1024;
/// Largest chunk a receiver accepts (signed bodies are capped at 16 MiB).
pub const MAX_CHUNK: usize = 8 * 1024 * 1024;
/// Header carrying a chunk's SHA-256.
pub const CHUNK_SHA_HEADER: &str = "x-chunk-sha256";
/// Chunk retries after a hash mismatch before the transfer fails.
const CHUNK_RETRIES: u32 = 3;

/// Chunk size to use: `CM_SYNC_CHUNK_SIZE` (bytes, tests / development) else [`CHUNK_SIZE`].
pub fn chunk_size_from_env() -> usize {
    std::env::var("CM_SYNC_CHUNK_SIZE")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|n| (1024..=MAX_CHUNK).contains(n))
        .unwrap_or(CHUNK_SIZE)
}

pub fn sha256_hex(b: &[u8]) -> String {
    hex(&Sha256::digest(b))
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn sha256_file(p: &Path) -> std::io::Result<String> {
    let mut f = File::open(p)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}

/// Number of chunks of a `size`-byte file.
pub fn chunk_count(size: u64, chunk: usize) -> u64 {
    size.div_ceil(chunk as u64)
}

/// Length of chunk `index` (the last one is shorter).
pub fn chunk_len(size: u64, chunk: usize, index: u64) -> usize {
    let start = index.saturating_mul(chunk as u64);
    size.saturating_sub(start).min(chunk as u64) as usize
}

#[derive(Debug, thiserror::Error)]
pub enum TransferError {
    #[error(transparent)]
    Peer(#[from] PeerError),
    #[error("the other device expected data from byte {expected}, not {got}")]
    Offset { expected: u64, got: u64 },
    #[error("piece {0} arrived damaged (its checksum does not match); try again")]
    ChunkHash(u64),
    #[error("the copied file does not match its checksum; it was discarded, start the sync again")]
    WholeHash,
    #[error("the copied file has the wrong size ({got} bytes, expected {expected})")]
    Size { expected: u64, got: u64 },
    #[error("cancelled")]
    Cancelled,
    #[error("{0}")]
    Io(String),
}

impl From<std::io::Error> for TransferError {
    fn from(e: std::io::Error) -> Self {
        TransferError::Io(e.to_string())
    }
}

/// What both sides remember about a transfer between steps (`<data>/sync/{in,out}/<id>.json`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub id: String,
    /// The other device.
    pub peer: String,
    /// The profile on the sending side.
    pub profile_id: String,
    pub name: String,
    pub size: u64,
    pub sha256: String,
    pub chunk: usize,
    /// Content hash of the profile at export time (what the sender records as synced).
    pub content_hash: String,
    pub created_at: u64,
    /// Receiver only: `create` | `replace` | `copy` and the profile to replace.
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub target_id: Option<String>,
    /// Receiver only: proxy details the sender attached.
    #[serde(default)]
    pub proxy: Option<serde_json::Value>,
}

/// A transfer id from a peer: 16-64 lowercase hex chars, usable as a file name.
pub fn valid_transfer_id(s: &str) -> bool {
    (16..=64).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// `<dir>/<id>.json` + data file `<id>.<ext>` store of [`Meta`].
pub struct Store {
    dir: PathBuf,
    ext: &'static str,
}

impl Store {
    /// `ext` is `part` (receiving) or `zip` (sending).
    pub fn new(dir: PathBuf, ext: &'static str) -> Store {
        Store { dir, ext }
    }

    pub fn data_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.{}", self.ext))
    }

    fn meta_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    pub fn ensure_dir(&self) -> std::io::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        fs::create_dir_all(&self.dir)?;
        fs::set_permissions(&self.dir, fs::Permissions::from_mode(0o700))
    }

    pub fn save(&self, m: &Meta) -> std::io::Result<()> {
        self.ensure_dir()?;
        let tmp = self.dir.join(format!("{}.json.tmp", m.id));
        let mut f = OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?;
        f.write_all(&serde_json::to_vec_pretty(m).map_err(std::io::Error::other)?)?;
        f.sync_all()?;
        fs::rename(tmp, self.meta_path(&m.id))
    }

    pub fn load(&self, id: &str) -> Option<Meta> {
        if !valid_transfer_id(id) {
            return None;
        }
        serde_json::from_slice(&fs::read(self.meta_path(id)).ok()?).ok()
    }

    pub fn all(&self) -> Vec<Meta> {
        let Ok(rd) = fs::read_dir(&self.dir) else { return Vec::new() };
        rd.filter_map(Result::ok)
            .filter_map(|e| e.file_name().into_string().ok())
            .filter_map(|n| n.strip_suffix(".json").map(str::to_owned))
            .filter_map(|id| self.load(&id))
            .collect()
    }

    pub fn remove(&self, id: &str) {
        if valid_transfer_id(id) {
            let _ = fs::remove_file(self.data_path(id));
            let _ = fs::remove_file(self.meta_path(id));
        }
    }

    /// Drop transfers older than `max_age` seconds (and data files without a record).
    pub fn gc(&self, now: u64, max_age: u64) {
        for m in self.all() {
            if now.saturating_sub(m.created_at) > max_age {
                self.remove(&m.id);
            }
        }
        let Ok(rd) = fs::read_dir(&self.dir) else { return };
        for e in rd.filter_map(Result::ok) {
            let name = e.file_name().to_string_lossy().into_owned();
            if let Some(id) = name.strip_suffix(&format!(".{}", self.ext))
                && self.load(id).is_none()
            {
                let _ = fs::remove_file(e.path());
            }
        }
    }
}

/// The receiver's partial file: only verified data, in order.
pub struct PartFile {
    path: PathBuf,
    size: u64,
    sha256: String,
    chunk: usize,
    received: u64,
}

impl PartFile {
    /// Open (or create) the part file. Whatever is beyond the last complete chunk is cut off: a
    /// torn write must never be mistaken for verified data. A file longer than `size` restarts.
    pub fn open(path: &Path, size: u64, sha256: &str, chunk: usize) -> Result<PartFile, TransferError> {
        if chunk == 0 || chunk > MAX_CHUNK {
            return Err(TransferError::Io(format!("chunk size {chunk} is out of range")));
        }
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let f = OpenOptions::new().read(true).write(true).create(true).truncate(false).mode(0o600).open(path)?;
        let len = f.metadata()?.len();
        let keep = if len > size {
            0
        } else if len == size {
            size
        } else {
            len / chunk as u64 * chunk as u64
        };
        if keep != len {
            f.set_len(keep)?;
        }
        Ok(PartFile { path: path.to_path_buf(), size, sha256: sha256.to_owned(), chunk, received: keep })
    }

    /// Bytes verified so far = the offset the sender continues from.
    pub fn received(&self) -> u64 {
        self.received
    }

    pub fn complete(&self) -> bool {
        self.received == self.size
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Next chunk index to ask for.
    pub fn next_index(&self) -> u64 {
        self.received / self.chunk as u64
    }

    /// Verify `bytes` against `claimed_sha` and append them as the chunk starting at `offset`.
    /// Nothing is written unless the offset, the length and the checksum are right.
    pub fn append(&mut self, offset: u64, bytes: &[u8], claimed_sha: &str) -> Result<u64, TransferError> {
        if offset != self.received {
            return Err(TransferError::Offset { expected: self.received, got: offset });
        }
        let index = offset / self.chunk as u64;
        if !offset.is_multiple_of(self.chunk as u64) || bytes.is_empty() || bytes.len() != chunk_len(self.size, self.chunk, index) {
            return Err(TransferError::ChunkHash(index));
        }
        if !sha256_hex(bytes).eq_ignore_ascii_case(claimed_sha) {
            return Err(TransferError::ChunkHash(index));
        }
        let mut f = OpenOptions::new().write(true).open(&self.path)?;
        f.seek(SeekFrom::Start(offset))?;
        f.write_all(bytes)?;
        f.sync_data()?;
        self.received += bytes.len() as u64;
        Ok(self.received)
    }

    /// Check size and whole-file SHA-256. On a hash mismatch the part file is deleted.
    pub fn finish(&self) -> Result<(), TransferError> {
        let len = fs::metadata(&self.path)?.len();
        if len != self.size || self.received != self.size {
            return Err(TransferError::Size { expected: self.size, got: len });
        }
        if !sha256_file(&self.path)?.eq_ignore_ascii_case(&self.sha256) {
            let _ = fs::remove_file(&self.path);
            return Err(TransferError::WholeHash);
        }
        Ok(())
    }
}

/// Read chunk `index` of a file of `size` bytes (sender side).
pub fn read_chunk(path: &Path, size: u64, chunk: usize, index: u64) -> Result<Vec<u8>, TransferError> {
    let n = chunk_len(size, chunk, index);
    if n == 0 {
        return Err(TransferError::Io(format!("no piece {index} in a {size}-byte file")));
    }
    let mut f = File::open(path)?;
    f.seek(SeekFrom::Start(index * chunk as u64))?;
    let mut buf = vec![0u8; n];
    f.read_exact(&mut buf)?;
    Ok(buf)
}

/// Where a receiver gets chunks from (the peer's `GET …/chunk/{n}`).
#[async_trait]
pub trait ChunkSource: Send + Sync {
    /// Bytes of chunk `index` and the SHA-256 the sender states for them.
    async fn chunk(&self, index: u64) -> Result<(Vec<u8>, String), PeerError>;
}

/// Where a sender puts chunks (the peer's `PUT …/chunk/{n}`).
#[async_trait]
pub trait ChunkSink: Send + Sync {
    /// Store chunk `index`; returns the receiver's confirmed offset.
    async fn put(&self, index: u64, bytes: Vec<u8>, sha256: &str) -> Result<u64, PeerError>;
    /// The receiver's confirmed offset now.
    async fn received(&self) -> Result<u64, PeerError>;
}

fn check_cancel(c: &AtomicBool) -> Result<(), TransferError> {
    if c.load(Ordering::SeqCst) { Err(TransferError::Cancelled) } else { Ok(()) }
}

/// Receiver loop: fetch the missing chunks (from the confirmed offset) into `part`.
/// `progress` gets the verified byte count after each chunk. Does not verify the whole file.
pub async fn pull_file(
    src: &dyn ChunkSource,
    part: &mut PartFile,
    cancel: &AtomicBool,
    progress: &(dyn Fn(u64) + Send + Sync),
) -> Result<(), TransferError> {
    progress(part.received());
    while !part.complete() {
        check_cancel(cancel)?;
        let index = part.next_index();
        let offset = part.received();
        let mut tries = 0;
        loop {
            let (bytes, sha) = src.chunk(index).await?;
            match part.append(offset, &bytes, &sha) {
                Ok(_) => break,
                Err(TransferError::ChunkHash(_)) if tries < CHUNK_RETRIES => {
                    tries += 1;
                    check_cancel(cancel)?;
                }
                Err(e) => return Err(e),
            }
        }
        progress(part.received());
    }
    Ok(())
}

/// Sender loop: put the chunks from the receiver's confirmed offset on. When the receiver
/// disagrees about the offset it is asked again and the loop continues from there.
pub async fn push_file(
    sink: &dyn ChunkSink,
    path: &Path,
    size: u64,
    chunk: usize,
    cancel: &AtomicBool,
    progress: &(dyn Fn(u64) + Send + Sync),
) -> Result<(), TransferError> {
    let mut offset = sink.received().await?;
    if offset > size || (!offset.is_multiple_of(chunk as u64) && offset != size) {
        offset = 0;
    }
    progress(offset);
    let mut resyncs = 0;
    while offset < size {
        check_cancel(cancel)?;
        let index = offset / chunk as u64;
        let bytes = read_chunk(path, size, chunk, index)?;
        let sha = sha256_hex(&bytes);
        match sink.put(index, bytes, &sha).await {
            Ok(got) => offset = got,
            Err(PeerError::Status { code, .. }) if (code == "bad_offset" || code == "bad_chunk") && resyncs < CHUNK_RETRIES => {
                resyncs += 1;
                offset = sink.received().await?;
            }
            Err(e) => return Err(e.into()),
        }
        progress(offset);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Mutex,
        atomic::{AtomicU64, AtomicUsize},
    };

    fn tmp(tag: &str) -> PathBuf {
        static N: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!("cm-sync-xfer-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn data(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 31 % 251) as u8).collect()
    }

    struct Mem {
        data: Vec<u8>,
        chunk: usize,
        asked: Mutex<Vec<u64>>,
        fail_from: AtomicU64,
        corrupt_once: AtomicU64,
    }

    impl Mem {
        fn new(data: Vec<u8>, chunk: usize) -> Mem {
            Mem { data, chunk, asked: Mutex::new(vec![]), fail_from: AtomicU64::new(u64::MAX), corrupt_once: AtomicU64::new(u64::MAX) }
        }
    }

    #[async_trait]
    impl ChunkSource for Mem {
        async fn chunk(&self, index: u64) -> Result<(Vec<u8>, String), PeerError> {
            if index >= self.fail_from.load(Ordering::SeqCst) {
                return Err(PeerError::Timeout { host: "peer".into() });
            }
            self.asked.lock().unwrap().push(index);
            let s = index as usize * self.chunk;
            let e = (s + self.chunk).min(self.data.len());
            let mut b = self.data[s..e].to_vec();
            let sha = sha256_hex(&b);
            if self.corrupt_once.compare_exchange(index, u64::MAX, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                b[0] ^= 0xff;
            }
            Ok((b, sha))
        }
    }

    fn noop(_: u64) {}

    #[test]
    fn chunk_math() {
        assert_eq!(chunk_count(10, 4), 3);
        assert_eq!(chunk_len(10, 4, 2), 2);
        assert_eq!(chunk_len(8, 4, 1), 4);
        assert_eq!(chunk_count(0, 4), 0);
    }

    #[tokio::test]
    async fn resume_continues_from_the_confirmed_offset() {
        let d = tmp("resume");
        let bytes = data(10_500);
        let sha = sha256_hex(&bytes);
        let src = Mem::new(bytes.clone(), 1000);
        src.fail_from.store(4, Ordering::SeqCst); // the link drops before piece 4
        let path = d.join("x.part");
        let cancel = AtomicBool::new(false);
        let mut part = PartFile::open(&path, 10_500, &sha, 1000).unwrap();
        let err = pull_file(&src, &mut part, &cancel, &noop).await.unwrap_err();
        assert!(matches!(err, TransferError::Peer(PeerError::Timeout { .. })), "{err:?}");
        assert_eq!(part.received(), 4000);
        // a torn write past the last verified chunk (crash while appending)
        let mut f = OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(&[9u8; 321]).unwrap();
        drop(f);

        let src2 = Mem::new(bytes.clone(), 1000);
        let mut part = PartFile::open(&path, 10_500, &sha, 1000).unwrap();
        assert_eq!(part.received(), 4000, "torn tail dropped");
        pull_file(&src2, &mut part, &cancel, &noop).await.unwrap();
        assert_eq!(*src2.asked.lock().unwrap(), vec![4, 5, 6, 7, 8, 9, 10], "only the missing pieces were requested");
        part.finish().unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        let _ = fs::remove_dir_all(&d);
    }

    #[tokio::test]
    async fn damaged_chunk_is_retried_then_rejected() {
        let d = tmp("hash");
        let bytes = data(3000);
        let sha = sha256_hex(&bytes);
        let cancel = AtomicBool::new(false);
        // one corrupted delivery: the piece is asked again
        let src = Mem::new(bytes.clone(), 1000);
        src.corrupt_once.store(1, Ordering::SeqCst);
        let mut part = PartFile::open(&d.join("a.part"), 3000, &sha, 1000).unwrap();
        pull_file(&src, &mut part, &cancel, &noop).await.unwrap();
        assert_eq!(*src.asked.lock().unwrap(), vec![0, 1, 1, 2]);
        part.finish().unwrap();

        // a direct append with a wrong checksum writes nothing
        let mut part = PartFile::open(&d.join("b.part"), 3000, &sha, 1000).unwrap();
        let e = part.append(0, &bytes[..1000], &sha256_hex(b"other")).unwrap_err();
        assert!(matches!(e, TransferError::ChunkHash(0)));
        assert_eq!(part.received(), 0);
        assert_eq!(fs::metadata(d.join("b.part")).unwrap().len(), 0);
        // wrong offset
        let e = part.append(1000, &bytes[1000..2000], &sha256_hex(&bytes[1000..2000])).unwrap_err();
        assert!(matches!(e, TransferError::Offset { expected: 0, got: 1000 }));
        let _ = fs::remove_dir_all(&d);
    }

    #[tokio::test]
    async fn whole_file_hash_mismatch_discards_the_part() {
        let d = tmp("whole");
        let bytes = data(2500);
        let wrong = sha256_hex(b"not the file");
        let src = Mem::new(bytes, 1000);
        let cancel = AtomicBool::new(false);
        let path = d.join("w.part");
        let mut part = PartFile::open(&path, 2500, &wrong, 1000).unwrap();
        pull_file(&src, &mut part, &cancel, &noop).await.unwrap();
        assert!(matches!(part.finish().unwrap_err(), TransferError::WholeHash));
        assert!(!path.exists(), "a file that fails the whole-file check is deleted");
        let _ = fs::remove_dir_all(&d);
    }

    #[tokio::test]
    async fn cancel_stops_between_chunks() {
        let d = tmp("cancel");
        let bytes = data(5000);
        let src = Mem::new(bytes.clone(), 1000);
        let cancel = AtomicBool::new(false);
        let mut part = PartFile::open(&d.join("c.part"), 5000, &sha256_hex(&bytes), 1000).unwrap();
        let p = |n: u64| {
            if n >= 2000 {
                cancel.store(true, Ordering::SeqCst);
            }
        };
        let e = pull_file(&src, &mut part, &cancel, &p).await.unwrap_err();
        assert!(matches!(e, TransferError::Cancelled));
        assert_eq!(part.received(), 2000);
        let _ = fs::remove_dir_all(&d);
    }

    struct Sink {
        part: Mutex<PartFile>,
        puts: Mutex<Vec<u64>>,
    }

    #[async_trait]
    impl ChunkSink for Sink {
        async fn put(&self, index: u64, bytes: Vec<u8>, sha: &str) -> Result<u64, PeerError> {
            self.puts.lock().unwrap().push(index);
            let mut p = self.part.lock().unwrap();
            let off = index * p.chunk as u64;
            p.append(off, &bytes, sha).map_err(|e| PeerError::Status {
                host: "p".into(),
                status: 409,
                code: "bad_offset".into(),
                message: e.to_string(),
            })
        }
        async fn received(&self) -> Result<u64, PeerError> {
            Ok(self.part.lock().unwrap().received())
        }
    }

    #[tokio::test]
    async fn push_resumes_from_the_receivers_offset() {
        let d = tmp("push");
        let bytes = data(4200);
        let sha = sha256_hex(&bytes);
        fs::write(d.join("src.zip"), &bytes).unwrap();
        let mut part = PartFile::open(&d.join("dst.part"), 4200, &sha, 1000).unwrap();
        part.append(0, &bytes[..1000], &sha256_hex(&bytes[..1000])).unwrap();
        part.append(1000, &bytes[1000..2000], &sha256_hex(&bytes[1000..2000])).unwrap();
        let sink = Sink { part: Mutex::new(part), puts: Mutex::new(vec![]) };
        let cancel = AtomicBool::new(false);
        push_file(&sink, &d.join("src.zip"), 4200, 1000, &cancel, &noop).await.unwrap();
        assert_eq!(*sink.puts.lock().unwrap(), vec![2, 3, 4]);
        sink.part.lock().unwrap().finish().unwrap();
        assert_eq!(fs::read(d.join("dst.part")).unwrap(), bytes);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn store_roundtrip_and_gc() {
        let d = tmp("store");
        let st = Store::new(d.clone(), "part");
        let m = Meta {
            id: "0123456789abcdef".into(),
            peer: "p".into(),
            profile_id: "x".into(),
            name: "n".into(),
            size: 1,
            sha256: "s".into(),
            chunk: 1000,
            content_hash: "h".into(),
            created_at: 100,
            action: None,
            target_id: None,
            proxy: None,
        };
        st.save(&m).unwrap();
        fs::write(st.data_path(&m.id), b"x").unwrap();
        assert_eq!(st.load(&m.id).unwrap().peer, "p");
        assert!(st.load("../etc/passwd").is_none());
        st.gc(150, 1000);
        assert!(st.load(&m.id).is_some());
        st.gc(5000, 1000);
        assert!(st.load(&m.id).is_none() && !st.data_path(&m.id).exists());
        let _ = fs::remove_dir_all(&d);
    }
}
