//! Filesystem-backed `Cache`.

mod cache;
mod error;

pub use cache::FsCache;
pub use error::FsCacheError;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::tmp_sibling;
    use librarian_domain::{Cache, CacheKey};
    use tempfile::tempdir;

    fn key(s: &str) -> CacheKey {
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
        let dir = tempdir().unwrap();
        let c = FsCache::open(dir.path()).unwrap();

        let final_path = c.path_for(&key("aabb"));
        std::fs::create_dir_all(final_path.parent().unwrap()).unwrap();
        let tmp = tmp_sibling(&final_path);
        std::fs::write(&tmp, b"partial").unwrap();

        assert_eq!(c.get(&key("aabb")).unwrap(), None);
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
