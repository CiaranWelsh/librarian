use librarian_domain::{Cache, CacheKey};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct MemCache {
    inner: Mutex<HashMap<String, Vec<u8>>>,
}

impl MemCache {
    pub fn new() -> Self { Self::default() }
}

#[derive(Debug, thiserror::Error)]
#[error("mem-cache poisoned")]
pub struct MemCacheError;

impl Cache for MemCache {
    type Error = MemCacheError;

    fn get(&self, key: &CacheKey) -> Result<Option<Vec<u8>>, Self::Error> {
        let g = self.inner.lock().map_err(|_| MemCacheError)?;
        Ok(g.get(&key.0).cloned())
    }

    fn put(&self, key: &CacheKey, value: &[u8]) -> Result<(), Self::Error> {
        let mut g = self.inner.lock().map_err(|_| MemCacheError)?;
        g.insert(key.0.clone(), value.to_vec());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let c = MemCache::new();
        let k = CacheKey("k".into());
        c.put(&k, b"v").unwrap();
        assert_eq!(c.get(&k).unwrap(), Some(b"v".to_vec()));
    }

    #[test]
    fn missing_key_is_none_not_error() {
        let c = MemCache::new();
        assert_eq!(c.get(&CacheKey("nope".into())).unwrap(), None);
    }

    #[test]
    fn put_overwrites() {
        let c = MemCache::new();
        let k = CacheKey("k".into());
        c.put(&k, b"a").unwrap();
        c.put(&k, b"b").unwrap();
        assert_eq!(c.get(&k).unwrap(), Some(b"b".to_vec()));
    }
}
