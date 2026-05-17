use crate::adapter_identity::AdapterIdentity;
use crate::chunk::{Chunk, Vector};
use crate::ids::SourceId;

pub trait Indexer: AdapterIdentity {
    type Error: std::error::Error + Send + Sync + 'static;
    fn upsert(&self, chunks: &[Chunk], vectors: &[Vector]) -> Result<(), Self::Error>;
    fn replace(
        &self,
        source_id: &SourceId,
        chunks: &[Chunk],
        vectors: &[Vector],
    ) -> Result<(), Self::Error>;
    fn delete_by_source_id(&self, source_id: &SourceId) -> Result<(), Self::Error>;
}
