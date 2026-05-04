//! Integration: cache survives a fresh `FsCache` open against the same root —
//! the across-process-restart guarantee in slice-003 AC.

use adapter_cache_fs::FsCache;
use librarian_domain::{Cache, CacheKey};
use tempfile::tempdir;

#[test]
fn write_in_one_open_read_in_another() {
    let dir = tempdir().unwrap();
    let key = CacheKey("a".repeat(64));

    {
        let c = FsCache::open(dir.path()).unwrap();
        c.put(&key, b"persisted").unwrap();
    } // drop — this open is gone

    let c2 = FsCache::open(dir.path()).unwrap();
    assert_eq!(c2.get(&key).unwrap(), Some(b"persisted".to_vec()));
}
