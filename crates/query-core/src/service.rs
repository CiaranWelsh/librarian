//! `QueryService` — the query logic. `search` runs the blocking embedder off the
//! reactor via `spawn_blocking`, bounded by a semaphore (ADR-0005), then delegates
//! to the `Searcher`. `list_documents`/`get_extract`/`list_collections` skip the
//! embedder.

use std::sync::Arc;

use librarian_domain::{
    CollectionInfo, Embedder, EmbedderError, ExtractChunk, Hit, Searcher, SourceId,
};
use tokio::sync::Semaphore;

use crate::error::QueryError;

pub struct QueryService<E, S> {
    embedder: Arc<E>,
    searcher: S,
    embed_permits: Arc<Semaphore>,
}

impl<E, S> QueryService<E, S>
where
    E: Embedder + Send + Sync + 'static,
    S: Searcher,
{
    /// `max_concurrent_embeds` bounds in-flight embed calls (QA-Q1 contention).
    pub fn new(embedder: Arc<E>, searcher: S, max_concurrent_embeds: usize) -> Self {
        Self {
            embedder,
            searcher,
            embed_permits: Arc::new(Semaphore::new(max_concurrent_embeds.max(1))),
        }
    }

    pub async fn search(
        &self,
        collection: &str,
        query: &str,
        limit: u64,
        content_type: Option<&str>,
    ) -> Result<Vec<Hit>, QueryError> {
        if query.trim().is_empty() {
            return Err(QueryError::EmptyQuery);
        }
        let vector = self.embed(query).await?;
        Ok(self
            .searcher
            .search(collection, &vector, limit, content_type)
            .await?)
    }

    pub async fn list_documents(&self, collection: &str) -> Result<Vec<SourceId>, QueryError> {
        Ok(self.searcher.list_documents(collection).await?)
    }

    pub async fn get_extract(
        &self,
        collection: &str,
        source_id: &SourceId,
        start: u32,
        end: u32,
    ) -> Result<Vec<ExtractChunk>, QueryError> {
        Ok(self
            .searcher
            .get_extract(collection, source_id, start, end)
            .await?)
    }

    pub async fn list_collections(&self) -> Result<Vec<CollectionInfo>, QueryError> {
        Ok(self.searcher.list_collections().await?)
    }

    /// Embed one query string off the reactor. The permit is moved into the
    /// closure so it covers the full blocking embed, then dropped on return.
    async fn embed(&self, query: &str) -> Result<Vec<f32>, QueryError> {
        let permit = Arc::clone(&self.embed_permits)
            .acquire_owned()
            .await
            .expect("embed semaphore is never closed");
        let embedder = Arc::clone(&self.embedder);
        let owned = query.to_string();
        let joined = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            embedder.embed(&[owned.as_str()])
        })
        .await;
        match joined {
            Err(_) => Err(QueryError::EmbedPanic),
            Ok(Err(EmbedderError::Recoverable(m))) => Err(QueryError::EmbedRecoverable(m)),
            Ok(Err(EmbedderError::Terminal(m))) => Err(QueryError::EmbedTerminal(m)),
            Ok(Ok(vecs)) => vecs
                .into_iter()
                .next()
                .ok_or_else(|| QueryError::EmbedTerminal("embedder returned no vectors".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_embedder_stub::StubEmbedder;
    use adapter_indexer_mem::MemSearcher;
    use librarian_domain::{BookMeta, Chunk, ChunkId, ChunkPayload, Provenance};

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

    fn service_with_two_docs() -> QueryService<StubEmbedder, MemSearcher> {
        let stub = StubEmbedder::new();
        let mem = MemSearcher::new();
        let apple_vec = stub.embed(&["apple"]).unwrap().remove(0);
        let zebra_vec = stub.embed(&["zebra"]).unwrap().remove(0);
        mem.add("c", book_chunk("apple", 0, "apple"), apple_vec);
        mem.add("c", book_chunk("zebra", 0, "zebra"), zebra_vec);
        QueryService::new(Arc::new(stub), mem, 4)
    }

    #[tokio::test]
    async fn search_returns_nearest_doc_first() {
        let svc = service_with_two_docs();
        let hits = svc.search("c", "apple", 10, None).await.unwrap();
        assert_eq!(hits[0].source_id.0, "apple");
    }

    #[tokio::test]
    async fn empty_query_is_rejected_without_embedding() {
        let svc = service_with_two_docs();
        let err = svc.search("c", "   ", 10, None).await.unwrap_err();
        assert!(matches!(err, QueryError::EmptyQuery));
    }

    #[tokio::test]
    async fn search_propagates_not_found() {
        let svc = service_with_two_docs();
        let err = svc.search("missing", "apple", 5, None).await.unwrap_err();
        assert!(matches!(
            err,
            QueryError::Search(librarian_domain::SearchError::NotFound(_))
        ));
    }
}
