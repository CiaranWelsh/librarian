//! Provenance chain attached to every `Chunk` — one link per pipeline stage
//! that produced it (F-M.6).

use serde::{Deserialize, Serialize};

use crate::ids::{CacheKey, ConfigHash, StageVersion};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceLink {
    pub stage_name: String,
    pub stage_version: StageVersion,
    pub config_hash: ConfigHash,
    pub cache_key: CacheKey,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Provenance(pub Vec<ProvenanceLink>);
