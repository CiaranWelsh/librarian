//! Supertrait every stage adapter implements. The runner uses these three
//! values to derive a `CacheKey` (see `cache::cache_key::derive`) — adapters
//! never compute keys themselves.

use crate::ids::{ConfigHash, StageVersion};

pub trait AdapterIdentity {
    fn name(&self) -> &str;
    fn version(&self) -> StageVersion;
    fn config_hash(&self) -> ConfigHash;
}
