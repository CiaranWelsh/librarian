//! Filesystem-backed Cache. Atomic put via tmp-write + fsync + rename.

use librarian_domain::{Cache, CacheKey};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct FsCache {
    root: PathBuf,
}

impl FsCache {
    /// Open (or create) a cache rooted at `root`. The directory is created if missing.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, FsCacheError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(FsCacheError::Io)?;
        Ok(Self { root })
    }

    /// Cache keys are 64-char lowercase hex (per `cache_key::derive`).
    /// Shard by the first two chars to keep directory fanout reasonable.
    fn path_for(&self, key: &CacheKey) -> PathBuf {
        let k = &key.0;
        let (shard, rest) = if k.len() >= 2 { (&k[..2], &k[2..]) } else { ("__", k.as_str()) };
        self.root.join(shard).join(rest)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FsCacheError {
    #[error("io: {0}")]
    Io(#[source] std::io::Error),
}

impl Cache for FsCache {
    type Error = FsCacheError;

    fn get(&self, key: &CacheKey) -> Result<Option<Vec<u8>>, Self::Error> {
        let p = self.path_for(key);
        match fs::read(&p) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(FsCacheError::Io(e)),
        }
    }

    fn put(&self, key: &CacheKey, value: &[u8]) -> Result<(), Self::Error> {
        let final_path = self.path_for(key);
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent).map_err(FsCacheError::Io)?;
        }
        let tmp = tmp_sibling(&final_path);
        write_then_fsync(&tmp, value).map_err(FsCacheError::Io)?;
        fs::rename(&tmp, &final_path).map_err(FsCacheError::Io)?;
        Ok(())
    }
}

fn tmp_sibling(path: &Path) -> PathBuf {
    let mut name = path.file_name().expect("non-root path").to_os_string();
    name.push(format!(".tmp.{}", std::process::id()));
    path.with_file_name(name)
}

fn write_then_fsync(path: &Path, value: &[u8]) -> std::io::Result<()> {
    let mut f = fs::File::create(path)?;
    f.write_all(value)?;
    f.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn key(s: &str) -> CacheKey {
        // Pad to ≥2 chars so sharding works in tests.
        let mut s = s.to_string();
        while s.len() < 2 { s.push('0'); }
        CacheKey(s)
    }

    #[test]
    fn round_trip() {
        let dir = tempdir().unwrap();
        let c = FsCache::open(dir.path()).unwrap();
        c.put(&key("aabb"), b"value").unwrap();
        assert_eq!(c.get(&key("aabb")).unwrap(), Some(b"value".to_vec()));
    }

    #[test]
    fn missing_key_is_none_not_error() {
        let dir = tempdir().unwrap();
        let c = FsCache::open(dir.path()).unwrap();
        assert_eq!(c.get(&key("nope")).unwrap(), None);
    }

    #[test]
    fn put_overwrites_atomically() {
        let dir = tempdir().unwrap();
        let c = FsCache::open(dir.path()).unwrap();
        c.put(&key("aabb"), b"v1").unwrap();
        c.put(&key("aabb"), b"v2").unwrap();
        assert_eq!(c.get(&key("aabb")).unwrap(), Some(b"v2".to_vec()));
    }

    #[test]
    fn orphan_tmp_is_invisible_to_get_simulated_crash() {
        // Simulate: a partial write left a `.tmp` file behind. `get` must
        // still return None for the real key, never returning partial bytes.
        let dir = tempdir().unwrap();
        let c = FsCache::open(dir.path()).unwrap();

        let final_path = c.path_for(&key("aabb"));
        std::fs::create_dir_all(final_path.parent().unwrap()).unwrap();
        let tmp = tmp_sibling(&final_path);
        std::fs::write(&tmp, b"partial").unwrap();

        assert_eq!(c.get(&key("aabb")).unwrap(), None);
        // The orphan file remains on disk (cleanup is the operator's call), but
        // it does not corrupt cache reads.
        assert!(tmp.exists());
    }

    #[test]
    fn open_creates_root_if_missing() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("a/b/c");
        let c = FsCache::open(&nested).unwrap();
        c.put(&key("aabb"), b"x").unwrap();
        assert!(nested.exists());
    }

    #[test]
    fn keys_with_same_shard_prefix_do_not_collide() {
        let dir = tempdir().unwrap();
        let c = FsCache::open(dir.path()).unwrap();
        c.put(&key("aabb"), b"first").unwrap();
        c.put(&key("aacc"), b"second").unwrap();
        assert_eq!(c.get(&key("aabb")).unwrap(), Some(b"first".to_vec()));
        assert_eq!(c.get(&key("aacc")).unwrap(), Some(b"second".to_vec()));
    }
}
