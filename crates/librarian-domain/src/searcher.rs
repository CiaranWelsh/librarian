//! `Searcher` — read-only outbound port for vector search + metadata scroll.
//!
//! Per ADR-0004/0005: the query side depends on this port (not on Qdrant).
//! Methods return `-> impl Future<Output=…> + Send` (stable RPITIT) so generic
//! callers — e.g. axum handlers, which require `Send` futures — compile without
//! the `async-trait` crate and without `Box<dyn>`.

use crate::ids::SourceId;
use std::future::Future;

/// A single search result (the ADR hit shape). `content_type` is the raw
/// payload string ("book"/"paper"/"code"/"figure") — kept as `String` so the
/// port never loses values the `ContentType` enum doesn't model (e.g. "figure").
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub score: f32,
    pub source_id: SourceId,
    pub content_type: String,
    pub chunk_index: u32,
    pub text: String,
}

/// One chunk of a scoped document extract.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractChunk {
    pub chunk_index: u32,
    pub text: String,
}

/// Summary of one collection.
#[derive(Debug, Clone, PartialEq)]
pub struct CollectionInfo {
    pub name: String,
    pub points: u64,
}

/// Failure modes a `Searcher` distinguishes so the daemon can map them to the
/// right HTTP status (ADR-0005 error table).
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("collection not found: {0}")]
    NotFound(String),
    #[error("search backend unavailable: {0}")]
    Unavailable(String),
    #[error("search backend error: {0}")]
    Backend(String),
}

/// Read-only port for vector search and metadata scroll.
///
/// Implementors must ensure the futures returned by each method are `Send`.
/// Concretely: do not hold a non-`Send` value (e.g. a `MutexGuard`) across an
/// `.await` point — drop the guard before the first `.await`.
pub trait Searcher {
    /// Vector search over the collection's `text` named vector. Up to `limit`
    /// hits ordered by descending similarity. `content_type` narrows by payload.
    fn search(
        &self,
        collection: &str,
        vector: &[f32],
        limit: u64,
        content_type: Option<&str>,
    ) -> impl Future<Output = Result<Vec<Hit>, SearchError>> + Send;

    /// Distinct `source_id`s present in the collection.
    fn list_documents(
        &self,
        collection: &str,
    ) -> impl Future<Output = Result<Vec<SourceId>, SearchError>> + Send;

    /// Chunks of `source_id` whose `chunk_index` is in `[start, end)`, ordered.
    fn get_extract(
        &self,
        collection: &str,
        source_id: &SourceId,
        start: u32,
        end: u32,
    ) -> impl Future<Output = Result<Vec<ExtractChunk>, SearchError>> + Send;

    /// All collections with their point counts.
    fn list_collections(
        &self,
    ) -> impl Future<Output = Result<Vec<CollectionInfo>, SearchError>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_carries_the_adr_fields() {
        let h = Hit {
            score: 0.5,
            source_id: SourceId("doc".into()),
            content_type: "book".into(),
            chunk_index: 3,
            text: "hello".into(),
        };
        assert_eq!(h.source_id.0, "doc");
        assert_eq!(h.chunk_index, 3);
    }
}
