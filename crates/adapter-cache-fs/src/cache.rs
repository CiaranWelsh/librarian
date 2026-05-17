//! Filesystem-backed Cache. Atomic put via tmp-write + fsync + rename.

use librarian_domain::{Cache, CacheKey};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::FsCacheError;

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
    pub(crate) fn path_for(&self, key: &CacheKey) -> PathBuf {
        let k = &key.0;
        let (shard, rest) = if k.len() >= 2 { (&k[..2], &k[2..]) } else { ("__", k.as_str()) };
        self.root.join(shard).join(rest)
    }
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

pub(crate) fn tmp_sibling(path: &Path) -> PathBuf {
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
