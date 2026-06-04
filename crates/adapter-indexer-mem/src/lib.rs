//! In-memory `Indexer` — used by tests and the walking skeleton.

mod error;
mod indexer;
mod point;
pub mod searcher;

pub use error::MemIndexerError;
pub use indexer::MemIndexer;
pub use point::Point;
pub use searcher::MemSearcher;

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::{BookMeta, Chunk, ChunkId, ChunkPayload, Indexer, Provenance, SourceId};

    fn chunk(sid: &str, idx: u32) -> Chunk {
        Chunk {
            chunk_id: ChunkId(format!("{sid}#{idx}")),
            source_id: SourceId(sid.into()),
            chunk_index: idx,
            text: format!("text-{idx}"),
            payload: ChunkPayload::Book(BookMeta {
                title: "t".into(),
                author: None,
                chapter: None,
                section: None,
                page: None,
            }),
            provenance: Provenance::default(),
        }
    }

    #[test]
    fn upsert_length_mismatch_errors() {
        let ix = MemIndexer::new();
        let r = ix.upsert(&[chunk("a", 0)], &[]);
        assert!(matches!(
            r,
            Err(MemIndexerError::LengthMismatch {
                chunks: 1,
                vectors: 0
            })
        ));
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
        )
        .unwrap();
        assert_eq!(ix.count(), 3);
        ix.replace(&SourceId("a".into()), &[chunk("a", 0)], &[vec![0.0]])
            .unwrap();
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
        ix.upsert(&[chunk("a", 0), chunk("b", 0)], &[vec![0.0], vec![0.0]])
            .unwrap();
        assert_eq!(ix.by_source(&SourceId("a".into())).len(), 1);
    }
}
