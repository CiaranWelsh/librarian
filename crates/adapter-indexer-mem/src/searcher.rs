//! `MemSearcher` — an in-memory `Searcher` for testing `query-core` with no
//! network. Stores points per collection; `search` ranks by cosine similarity.

use std::collections::HashMap;
use std::sync::Mutex;

use librarian_domain::{
    Chunk, ChunkPayload, CollectionInfo, ExtractChunk, Hit, SearchError, Searcher, SourceId, Vector,
};

#[derive(Default)]
pub struct MemSearcher {
    points: Mutex<HashMap<String, Vec<(Chunk, Vector)>>>,
}

impl MemSearcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Test helper: insert a point into `collection`.
    pub fn add(&self, collection: &str, chunk: Chunk, vector: Vector) {
        self.points
            .lock()
            .unwrap()
            .entry(collection.to_string())
            .or_default()
            .push((chunk, vector));
    }
}

fn content_type_str(p: &ChunkPayload) -> &'static str {
    match p {
        ChunkPayload::Book(_) => "book",
        ChunkPayload::Paper(_) => "paper",
        ChunkPayload::Code(_) => "code",
        ChunkPayload::Figure(_) => "figure",
    }
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

impl Searcher for MemSearcher {
    async fn search(
        &self,
        collection: &str,
        vector: &[f32],
        limit: u64,
        content_type: Option<&str>,
    ) -> Result<Vec<Hit>, SearchError> {
        let guard = self.points.lock().unwrap();
        let pts = guard
            .get(collection)
            .ok_or_else(|| SearchError::NotFound(collection.to_string()))?;
        let mut scored: Vec<Hit> = pts
            .iter()
            .filter(|(c, _)| content_type.map_or(true, |ct| content_type_str(&c.payload) == ct))
            .map(|(c, v)| Hit {
                score: cosine(vector, v),
                source_id: c.source_id.clone(),
                content_type: content_type_str(&c.payload).to_string(),
                chunk_index: c.chunk_index,
                text: c.text.clone(),
            })
            .collect();
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(limit as usize);
        Ok(scored)
    }

    async fn list_documents(&self, collection: &str) -> Result<Vec<SourceId>, SearchError> {
        let guard = self.points.lock().unwrap();
        let pts = guard
            .get(collection)
            .ok_or_else(|| SearchError::NotFound(collection.to_string()))?;
        let mut seen = Vec::new();
        for (c, _) in pts {
            if !seen.contains(&c.source_id) {
                seen.push(c.source_id.clone());
            }
        }
        Ok(seen)
    }

    async fn get_extract(
        &self,
        collection: &str,
        source_id: &SourceId,
        start: u32,
        end: u32,
    ) -> Result<Vec<ExtractChunk>, SearchError> {
        let guard = self.points.lock().unwrap();
        let pts = guard
            .get(collection)
            .ok_or_else(|| SearchError::NotFound(collection.to_string()))?;
        let mut out: Vec<ExtractChunk> = pts
            .iter()
            .filter(|(c, _)| {
                &c.source_id == source_id && c.chunk_index >= start && c.chunk_index < end
            })
            .map(|(c, _)| ExtractChunk {
                chunk_index: c.chunk_index,
                text: c.text.clone(),
            })
            .collect();
        out.sort_by_key(|e| e.chunk_index);
        Ok(out)
    }

    async fn list_collections(&self) -> Result<Vec<CollectionInfo>, SearchError> {
        let guard = self.points.lock().unwrap();
        let mut out: Vec<CollectionInfo> = guard
            .iter()
            .map(|(name, pts)| CollectionInfo {
                name: name.clone(),
                points: pts.len() as u64,
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::{BookMeta, ChunkId, PaperMeta, Provenance};

    fn book_chunk(source: &str, idx: u32, text: &str) -> Chunk {
        Chunk {
            chunk_id: ChunkId(format!("{source}#{idx}")),
            source_id: SourceId(source.into()),
            chunk_index: idx,
            text: text.into(),
            payload: ChunkPayload::Book(BookMeta {
                title: source.into(),
                author: None,
                chapter: None,
                section: None,
                page: None,
            }),
            provenance: Provenance::default(),
        }
    }

    fn paper_chunk(source: &str, idx: u32, text: &str) -> Chunk {
        Chunk {
            chunk_id: ChunkId(format!("{source}#{idx}")),
            source_id: SourceId(source.into()),
            chunk_index: idx,
            text: text.into(),
            payload: ChunkPayload::Paper(PaperMeta {
                title: source.into(),
                authors: vec![],
                section: None,
                page_start: None,
                page_end: None,
            }),
            provenance: Provenance::default(),
        }
    }

    #[tokio::test]
    async fn search_ranks_nearest_vector_first() {
        let s = MemSearcher::new();
        s.add(
            "c",
            book_chunk("apple", 0, "apple text"),
            vec![1.0, 0.0, 0.0],
        );
        s.add(
            "c",
            book_chunk("zebra", 0, "zebra text"),
            vec![0.0, 1.0, 0.0],
        );

        let hits = s.search("c", &[0.9, 0.1, 0.0], 10, None).await.unwrap();

        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].source_id.0, "apple");
        assert!(hits[0].score > hits[1].score);
    }

    #[tokio::test]
    async fn search_unknown_collection_is_not_found() {
        let s = MemSearcher::new();
        let err = s.search("missing", &[1.0], 5, None).await.unwrap_err();
        assert!(matches!(err, SearchError::NotFound(_)));
    }

    #[tokio::test]
    async fn extract_returns_half_open_range_sorted() {
        let s = MemSearcher::new();
        s.add("c", book_chunk("d", 2, "two"), vec![1.0]);
        s.add("c", book_chunk("d", 0, "zero"), vec![1.0]);
        s.add("c", book_chunk("d", 1, "one"), vec![1.0]);

        let chunks = s
            .get_extract("c", &SourceId("d".into()), 0, 2)
            .await
            .unwrap();

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[1].chunk_index, 1);
    }

    #[tokio::test]
    async fn list_documents_dedupes_source_ids() {
        let s = MemSearcher::new();
        s.add("c", book_chunk("d", 0, "a"), vec![1.0]);
        s.add("c", book_chunk("d", 1, "b"), vec![1.0]);
        let docs = s.list_documents("c").await.unwrap();
        assert_eq!(docs, vec![SourceId("d".into())]);
    }

    #[tokio::test]
    async fn search_filters_by_content_type() {
        let s = MemSearcher::new();
        s.add("c", book_chunk("the-book", 0, "book text"), vec![1.0, 0.0]);
        s.add(
            "c",
            paper_chunk("the-paper", 0, "paper text"),
            vec![1.0, 0.0],
        );

        let hits = s.search("c", &[1.0, 0.0], 10, Some("paper")).await.unwrap();

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].source_id.0, "the-paper");
        assert_eq!(hits[0].content_type, "paper");
    }
}
