use librarian_domain::{
    AdapterIdentity, Chunk, ConfigHash, Indexer, SourceId, StageVersion, Vector,
};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Point {
    pub chunk: Chunk,
    pub vector: Vector,
}

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

#[derive(Debug, thiserror::Error)]
pub enum MemIndexerError {
    #[error("length mismatch: {chunks} chunks vs {vectors} vectors")]
    LengthMismatch { chunks: usize, vectors: usize },
    #[error("poisoned")]
    Poisoned,
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

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::{
        BookMeta, ChunkId, ChunkPayload, Provenance,
    };

    fn chunk(sid: &str, idx: u32) -> Chunk {
        Chunk {
            chunk_id: ChunkId(format!("{sid}#{idx}")),
            source_id: SourceId(sid.into()),
            chunk_index: idx,
            text: format!("text-{idx}"),
            payload: ChunkPayload::Book(BookMeta {
                title: "t".into(), author: None, chapter: None, section: None, page: None,
            }),
            provenance: Provenance::default(),
        }
    }

    #[test]
    fn upsert_length_mismatch_errors() {
        let ix = MemIndexer::new();
        let r = ix.upsert(&[chunk("a", 0)], &[]);
        assert!(matches!(r, Err(MemIndexerError::LengthMismatch { chunks: 1, vectors: 0 })));
    }

    #[test]
    fn upsert_is_idempotent_on_chunk_id() {
        let ix = MemIndexer::new();
        ix.upsert(&[chunk("a", 0)], &[vec![0.0]]).unwrap();
        ix.upsert(&[chunk("a", 0)], &[vec![1.0]]).unwrap();
        assert_eq!(ix.count(), 1);
    }

    #[test]
    fn replace_removes_orphans() {
        let ix = MemIndexer::new();
        ix.upsert(
            &[chunk("a", 0), chunk("a", 1), chunk("a", 2)],
            &[vec![0.0], vec![0.0], vec![0.0]],
        ).unwrap();
        assert_eq!(ix.count(), 3);
        ix.replace(&SourceId("a".into()), &[chunk("a", 0)], &[vec![0.0]]).unwrap();
        assert_eq!(ix.count(), 1);
    }

    #[test]
    fn delete_by_source_id_is_idempotent() {
        let ix = MemIndexer::new();
        ix.delete_by_source_id(&SourceId("nope".into())).unwrap();
        assert_eq!(ix.count(), 0);
    }

    #[test]
    fn by_source_filters() {
        let ix = MemIndexer::new();
        ix.upsert(&[chunk("a", 0), chunk("b", 0)], &[vec![0.0], vec![0.0]]).unwrap();
        assert_eq!(ix.by_source(&SourceId("a".into())).len(), 1);
    }
}

