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
