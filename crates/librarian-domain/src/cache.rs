//! Cache port and the `CacheKey` derivation function (ADR-0001 §4).
//!
//! Key formula: `sha256(source_hash ‖ 0x1F ‖ stage_name ‖ 0x1F ‖ stage_version ‖ 0x1F ‖ config_hash)`.
//! 0x1F (Unit Separator) prevents adversarial concatenation collisions.
//!
//! The runner derives keys — adapters never see `CacheKey`. They expose
//! `AdapterIdentity` and the runner does the hashing. Single source of truth.

use crate::ids::{CacheKey, ConfigHash, SourceHash, StageVersion};

pub mod cache_key {
    use super::*;
    use sha2::{Digest, Sha256};

    pub fn derive(
        source_hash: &SourceHash,
        stage_name: &str,
        stage_version: &StageVersion,
        config_hash: &ConfigHash,
    ) -> CacheKey {
        let sep = [0x1Fu8];
        let mut h = Sha256::new();
        h.update(source_hash.0.as_bytes());
        h.update(sep);
        h.update(stage_name.as_bytes());
        h.update(sep);
        h.update(stage_version.0.as_bytes());
        h.update(sep);
        h.update(config_hash.0.as_bytes());
        CacheKey(hex::encode(h.finalize()))
    }
}

pub trait Cache {
    type Error: std::error::Error + Send + Sync + 'static;
    fn get(&self, key: &CacheKey) -> Result<Option<Vec<u8>>, Self::Error>;
    fn put(&self, key: &CacheKey, value: &[u8]) -> Result<(), Self::Error>;
}

impl<T: Cache + ?Sized> Cache for &T {
    type Error = T::Error;
    fn get(&self, key: &CacheKey) -> Result<Option<Vec<u8>>, Self::Error> { (**self).get(key) }
    fn put(&self, key: &CacheKey, value: &[u8]) -> Result<(), Self::Error> { (**self).put(key, value) }
}
