use serde::{Deserialize, Serialize};

use crate::ids::{ChunkId, SourceId};
use crate::payload::ChunkPayload;
use crate::provenance::Provenance;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub chunk_id: ChunkId,
    pub source_id: SourceId,
    pub chunk_index: u32,
    pub text: String,
    pub payload: ChunkPayload,
    pub provenance: Provenance,
}

pub type Vector = Vec<f32>;
