use librarian_domain::{
    AdapterIdentity, Chunk, ConfigHash, Indexer, SourceId, StageVersion, Vector,
};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::MemIndexerError;
use crate::point::Point;

#[derive(Default)]
pub struct MemIndexer {
    points: Mutex<HashMap<String, Point>>, // chunk_id -> point
}

impl MemIndexer {
    pub fn new() -> Self { Self::default() }
    pub fn points(&self) -> Vec<Point> { self.points.lock().unwrap().values().cloned().collect() }
    pub fn count(&self) -> usize { self.points.lock().unwrap().len() }
    pub fn by_source(&self, source_id: &SourceId) -> Vec<Point> {
        self.points
            .lock()
            .unwrap()
            .values()
            .filter(|p| p.chunk.source_id == *source_id)
            .cloned()
            .collect()
    }
}

impl AdapterIdentity for MemIndexer {
    fn name(&self) -> &str { "mem-indexer" }
    fn version(&self) -> StageVersion { StageVersion("0.1.0".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash("default".into()) }
}

impl Indexer for MemIndexer {
    type Error = MemIndexerError;

    fn upsert(&self, chunks: &[Chunk], vectors: &[Vector]) -> Result<(), Self::Error> {
        if chunks.len() != vectors.len() {
            return Err(MemIndexerError::LengthMismatch {
                chunks: chunks.len(),
                vectors: vectors.len(),
            });
        }
        let mut g = self.points.lock().map_err(|_| MemIndexerError::Poisoned)?;
        for (c, v) in chunks.iter().zip(vectors.iter()) {
            g.insert(
                c.chunk_id.0.clone(),
                Point { chunk: c.clone(), vector: v.clone() },
            );
        }
        Ok(())
    }

    fn replace(
        &self,
        source_id: &SourceId,
        chunks: &[Chunk],
        vectors: &[Vector],
    ) -> Result<(), Self::Error> {
        if chunks.len() != vectors.len() {
            return Err(MemIndexerError::LengthMismatch {
                chunks: chunks.len(),
                vectors: vectors.len(),
            });
        }
        let mut g = self.points.lock().map_err(|_| MemIndexerError::Poisoned)?;
        g.retain(|_, p| p.chunk.source_id != *source_id);
        for (c, v) in chunks.iter().zip(vectors.iter()) {
            g.insert(
                c.chunk_id.0.clone(),
                Point { chunk: c.clone(), vector: v.clone() },
            );
        }
        Ok(())
    }

    fn delete_by_source_id(&self, source_id: &SourceId) -> Result<(), Self::Error> {
        let mut g = self.points.lock().map_err(|_| MemIndexerError::Poisoned)?;
        g.retain(|_, p| p.chunk.source_id != *source_id);
        Ok(())
    }
}
