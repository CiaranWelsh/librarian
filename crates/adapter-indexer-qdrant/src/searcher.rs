//! `QdrantSearcher` — async `Searcher` over the async `qdrant-client`. Unlike
//! `QdrantIndexer` (sync, internal runtime, one collection), this is async and
//! takes the collection per call so one daemon fronts all collections.

use std::collections::HashSet;
use std::sync::Arc;

use librarian_domain::{CollectionInfo, ExtractChunk, Hit, SearchError, Searcher, SourceId};
use qdrant_client::qdrant::{
    with_payload_selector, Condition, Filter, PayloadIncludeSelector, ScrollPointsBuilder,
    SearchPointsBuilder,
};
use qdrant_client::Qdrant;

pub struct QdrantSearcher {
    client: Arc<Qdrant>,
}

impl QdrantSearcher {
    pub fn new(client: Arc<Qdrant>) -> Self {
        Self { client }
    }

    pub fn open(url: &str) -> Result<Self, SearchError> {
        let client = Qdrant::from_url(url)
            .build()
            .map_err(|e| SearchError::Unavailable(e.to_string()))?;
        Ok(Self {
            client: Arc::new(client),
        })
    }
}

// Map a qdrant error to our port error.
//
// For gRPC status errors we classify by code (NotFound -> NotFound,
// Unavailable/DeadlineExceeded -> Unavailable, else Backend). Other error
// kinds fall through to string-based heuristics.
fn map_err(e: qdrant_client::QdrantError) -> SearchError {
    use qdrant_client::QdrantError;
    match &e {
        QdrantError::ResponseError { status } => match status.code() {
            tonic::Code::NotFound => return SearchError::NotFound(e.to_string()),
            // ResourceExhausted (429) without retry-after metadata arrives as a
            // bare gRPC status; surface it as Unavailable (503) to match the
            // dedicated ResourceExhaustedError arm below.
            tonic::Code::Unavailable
            | tonic::Code::DeadlineExceeded
            | tonic::Code::ResourceExhausted => return SearchError::Unavailable(e.to_string()),
            _ => return SearchError::Backend(e.to_string()),
        },
        QdrantError::ResourceExhaustedError { .. } => {
            return SearchError::Unavailable(e.to_string())
        }
        _ => {}
    }
    // Fallback for non-status errors: heuristic string match.
    let lower = e.to_string().to_lowercase();
    if lower.contains("not found")
        || lower.contains("doesn't exist")
        || lower.contains("does not exist")
    {
        SearchError::NotFound(e.to_string())
    } else if lower.contains("unavailable")
        || lower.contains("transport error")
        || lower.contains("connect")
        || lower.contains("timeout")
    {
        SearchError::Unavailable(e.to_string())
    } else {
        SearchError::Backend(e.to_string())
    }
}

impl Searcher for QdrantSearcher {
    async fn search(
        &self,
        collection: &str,
        vector: &[f32],
        limit: u64,
        content_type: Option<&str>,
    ) -> Result<Vec<Hit>, SearchError> {
        let mut req = SearchPointsBuilder::new(collection, vector.to_vec(), limit)
            .with_payload(true)
            .vector_name("text");
        if let Some(ct) = content_type {
            req = req.filter(Filter::must([Condition::matches(
                "content_type",
                ct.to_string(),
            )]));
        }
        let r = self.client.search_points(req).await.map_err(map_err)?;
        Ok(r.result
            .into_iter()
            .map(|p| {
                let get = |k: &str| {
                    p.payload
                        .get(k)
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                };
                Hit {
                    score: p.score,
                    source_id: SourceId(get("source_id").unwrap_or_default()),
                    content_type: get("content_type").unwrap_or_default(),
                    chunk_index: p
                        .payload
                        .get("chunk_index")
                        .and_then(|v| v.as_integer())
                        .unwrap_or(0) as u32,
                    text: get("text").unwrap_or_default(),
                }
            })
            .collect())
    }

    async fn get_extract(
        &self,
        collection: &str,
        source_id: &SourceId,
        start: u32,
        end: u32,
    ) -> Result<Vec<ExtractChunk>, SearchError> {
        let filter = Filter::must([Condition::matches("source_id", source_id.0.clone())]);
        let mut all = Vec::new();
        let mut offset = None;
        loop {
            let mut b = ScrollPointsBuilder::new(collection)
                .filter(filter.clone())
                .with_payload(true)
                .limit(4096);
            if let Some(o) = offset.clone() {
                b = b.offset(o);
            }
            let r = self.client.scroll(b).await.map_err(map_err)?;
            all.extend(r.result);
            match r.next_page_offset {
                Some(o) => offset = Some(o),
                None => break,
            }
        }
        let mut out: Vec<ExtractChunk> = all
            .into_iter()
            .filter_map(|p| {
                let idx = p.payload.get("chunk_index").and_then(|v| v.as_integer())? as u32;
                if idx < start || idx >= end {
                    return None;
                }
                let text = p
                    .payload
                    .get("text")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))?;
                Some(ExtractChunk {
                    chunk_index: idx,
                    text,
                })
            })
            .collect();
        out.sort_by_key(|e| e.chunk_index);
        Ok(out)
    }

    async fn list_documents(&self, collection: &str) -> Result<Vec<SourceId>, SearchError> {
        // Reads from qdrant (indexed points). This is an intentional change from the
        // old manifest-based `distinct_ingested_sources` — the daemon has no manifest.
        // Still O(collection size); payload-facet deduplication is a follow-up.
        //
        // We request only the `source_id` field to reduce per-page payload transfer.
        let selector = with_payload_selector::SelectorOptions::Include(PayloadIncludeSelector {
            fields: vec!["source_id".into()],
        });
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        let mut offset = None;
        loop {
            let mut b = ScrollPointsBuilder::new(collection)
                .with_payload(selector.clone())
                .limit(4096);
            if let Some(o) = offset.clone() {
                b = b.offset(o);
            }
            let r = self.client.scroll(b).await.map_err(map_err)?;
            for p in &r.result {
                if let Some(sid) = p.payload.get("source_id").and_then(|v| v.as_str()) {
                    if seen.insert(sid.to_string()) {
                        out.push(SourceId(sid.to_string()));
                    }
                }
            }
            match r.next_page_offset {
                Some(o) => offset = Some(o),
                None => break,
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    async fn list_collections(&self) -> Result<Vec<CollectionInfo>, SearchError> {
        let cols = self.client.list_collections().await.map_err(map_err)?;
        let mut out = Vec::new();
        for c in cols.collections {
            let info = match self.client.collection_info(&c.name).await {
                Ok(i) => i,
                Err(_) => continue, // collection deleted between list and info; skip it
            };
            let points = info.result.and_then(|r| r.points_count).unwrap_or(0);
            out.push(CollectionInfo {
                name: c.name,
                points,
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }
}
