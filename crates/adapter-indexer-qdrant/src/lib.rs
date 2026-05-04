//! Qdrant-backed `Indexer`. Reserves the `text` named vector slot
//! (slice 016/017 will add `code`/`figure` slots later — additive).
//!
//! Sync trait wraps an internal Tokio runtime so the domain stays sync.

use librarian_domain::{
    AdapterIdentity, Chunk, ConfigHash, Indexer, SourceId, StageVersion, Vector,
};
use qdrant_client::qdrant::{
    point_id::PointIdOptions, vectors::VectorsOptions, vectors_config::Config as VectorsCfg,
    Condition, CreateCollectionBuilder, DeletePointsBuilder, Distance, FieldType, Filter,
    NamedVectors, PointId, PointStruct, UpsertPointsBuilder, Value as QValue, Vector as QVector,
    VectorParams, VectorParamsMap, VectorsConfig,
};
use qdrant_client::{Payload, Qdrant};
use std::collections::HashMap;

/// UUID v5 namespace for librarian point IDs (deterministic across runs).
const NAMESPACE: uuid::Uuid = uuid::Uuid::from_bytes([
    0xc0, 0x11, 0xec, 0x70, 0x71, 0xbe, 0x40, 0xa1, 0x95, 0x10, 0xd0, 0xea, 0xd0, 0x70, 0x6e, 0x73,
]);

/// Deterministic point ID from `(source_id, chunk_index)`.
pub fn point_id(source_id: &SourceId, chunk_index: u32) -> uuid::Uuid {
    uuid::Uuid::new_v5(
        &NAMESPACE,
        format!("{}#{}", source_id.0, chunk_index).as_bytes(),
    )
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub score: f32,
    pub source_id: String,
    pub chunk_index: u32,
    pub text: String,
    pub content_type: String,
}

pub struct QdrantIndexer {
    rt: tokio::runtime::Runtime,
    client: Qdrant,
    collection: String,
    dim: u64,
}

impl QdrantIndexer {
    /// Open a connection and ensure the collection exists with a `text` named
    /// vector slot of dimension `dim`. Idempotent.
    pub fn open(url: &str, collection: &str, dim: u64) -> Result<Self, QdrantError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(QdrantError::Runtime)?;
        let client = Qdrant::from_url(url).build().map_err(QdrantError::client)?;
        let me = Self {
            rt,
            client,
            collection: collection.to_string(),
            dim,
        };
        me.ensure_collection()?;
        Ok(me)
    }

    fn ensure_collection(&self) -> Result<(), QdrantError> {
        self.rt.block_on(async {
            let exists = self
                .client
                .collection_exists(&self.collection)
                .await
                .map_err(QdrantError::client)?;
            if !exists {
                let mut params = HashMap::new();
                params.insert(
                    "text".to_string(),
                    VectorParams {
                        size: self.dim,
                        distance: Distance::Cosine.into(),
                        ..Default::default()
                    },
                );
                let cfg = VectorsConfig {
                    config: Some(VectorsCfg::ParamsMap(VectorParamsMap { map: params })),
                };
                self.client
                    .create_collection(
                        CreateCollectionBuilder::new(&self.collection).vectors_config(cfg),
                    )
                    .await
                    .map_err(QdrantError::client)?;
                // Indexed payload fields per F-M.4.
                for field in ["content_type", "work_id", "source_id"] {
                    let _ = self
                        .client
                        .create_field_index(
                            qdrant_client::qdrant::CreateFieldIndexCollectionBuilder::new(
                                &self.collection,
                                field,
                                FieldType::Keyword,
                            ),
                        )
                        .await;
                }
            }
            Ok::<_, QdrantError>(())
        })
    }

    /// Semantic search using the `text` named vector. Returns up to `k` hits
    /// ordered by cosine similarity. `filter_content_type` narrows by F-M.4.
    pub fn search(
        &self,
        query: &[f32],
        k: u64,
        filter_content_type: Option<&str>,
    ) -> Result<Vec<SearchHit>, QdrantError> {
        self.rt.block_on(async {
            use qdrant_client::qdrant::SearchPointsBuilder;
            let mut req = SearchPointsBuilder::new(&self.collection, query.to_vec(), k)
                .with_payload(true)
                .vector_name("text");
            if let Some(ct) = filter_content_type {
                req = req.filter(Filter::must([Condition::matches("content_type", ct.to_string())]));
            }
            let r = self.client.search_points(req).await.map_err(QdrantError::client)?;
            Ok(r.result.into_iter().map(|p| {
                let payload_get = |k: &str| -> Option<String> {
                    p.payload.get(k).and_then(|v| v.as_str().map(|s| s.to_string()))
                };
                SearchHit {
                    score: p.score,
                    source_id: payload_get("source_id").unwrap_or_default(),
                    chunk_index: p.payload.get("chunk_index")
                        .and_then(|v| v.as_integer())
                        .unwrap_or(0) as u32,
                    text: payload_get("text").unwrap_or_default(),
                    content_type: payload_get("content_type").unwrap_or_default(),
                }
            }).collect())
        })
    }

    /// Scoped retrieval: chunks of `source_id` with `chunk_index` in
    /// `[start, end)` (half-open). Returns `(chunk_index, text)` ordered.
    pub fn get_extract(
        &self,
        source_id: &SourceId,
        start: u32,
        end: u32,
    ) -> Result<Vec<(u32, String)>, QdrantError> {
        self.rt.block_on(async {
            use qdrant_client::qdrant::ScrollPointsBuilder;
            let filter = Filter::must([Condition::matches("source_id", source_id.0.clone())]);
            let r = self.client
                .scroll(ScrollPointsBuilder::new(&self.collection).filter(filter).with_payload(true).limit(1024))
                .await
                .map_err(QdrantError::client)?;
            let mut hits: Vec<(u32, String)> = r.result.into_iter().filter_map(|p| {
                let idx = p.payload.get("chunk_index").and_then(|v| v.as_integer())? as u32;
                if idx < start || idx >= end { return None; }
                let text = p.payload.get("text").and_then(|v| v.as_str().map(|s| s.to_string()))?;
                Some((idx, text))
            }).collect();
            hits.sort_by_key(|(i, _)| *i);
            Ok(hits)
        })
    }

    /// Test/observability helper: total points in the collection.
    pub fn count(&self) -> Result<u64, QdrantError> {
        self.rt.block_on(async {
            let r = self
                .client
                .count(qdrant_client::qdrant::CountPointsBuilder::new(&self.collection).exact(true))
                .await
                .map_err(QdrantError::client)?;
            Ok(r.result.map(|c| c.count).unwrap_or(0))
        })
    }

    /// Test/observability helper: points carrying `source_id`.
    pub fn count_by_source(&self, source_id: &SourceId) -> Result<u64, QdrantError> {
        self.rt.block_on(async {
            let filter = Filter::must([Condition::matches("source_id", source_id.0.clone())]);
            let r = self
                .client
                .count(
                    qdrant_client::qdrant::CountPointsBuilder::new(&self.collection)
                        .filter(filter)
                        .exact(true),
                )
                .await
                .map_err(QdrantError::client)?;
            Ok(r.result.map(|c| c.count).unwrap_or(0))
        })
    }

    fn build_points(&self, chunks: &[Chunk], vectors: &[Vector]) -> Vec<PointStruct> {
        chunks
            .iter()
            .zip(vectors.iter())
            .map(|(c, v)| {
                let id = PointId {
                    point_id_options: Some(PointIdOptions::Uuid(
                        point_id(&c.source_id, c.chunk_index).to_string(),
                    )),
                };
                let mut named = HashMap::new();
                named.insert("text".to_string(), QVector::from(v.clone()));
                let vectors = qdrant_client::qdrant::Vectors {
                    vectors_options: Some(VectorsOptions::Vectors(NamedVectors { vectors: named })),
                };

                let payload = build_payload(c);
                PointStruct {
                    id: Some(id),
                    vectors: Some(vectors),
                    payload: payload.into(),
                }
            })
            .collect()
    }
}

fn build_payload(c: &Chunk) -> Payload {
    let mut map: HashMap<String, QValue> = HashMap::new();
    map.insert("source_id".into(), c.source_id.0.clone().into());
    map.insert("chunk_index".into(), (c.chunk_index as i64).into());
    map.insert("text".into(), c.text.clone().into());
    let content_type = match &c.payload {
        librarian_domain::ChunkPayload::Book(_) => "book",
        librarian_domain::ChunkPayload::Paper(_) => "paper",
        librarian_domain::ChunkPayload::Code(_) => "code",
    };
    map.insert("content_type".into(), content_type.into());
    // Store the typed payload as a serialized JSON string (sufficient for v1
    // retrieval; future filters on inner fields can decode or move to a
    // structured payload then).
    if let Ok(s) = serde_json::to_string(&c.payload) {
        map.insert("payload_json".into(), s.into());
    }
    Payload::from(map)
}

#[derive(Debug, thiserror::Error)]
pub enum QdrantError {
    #[error("runtime: {0}")]
    Runtime(#[source] std::io::Error),
    #[error("client: {0}")]
    Client(String),
    #[error("length mismatch: {chunks} chunks vs {vectors} vectors")]
    LengthMismatch { chunks: usize, vectors: usize },
}

impl QdrantError {
    fn client<E: std::fmt::Display>(e: E) -> Self {
        QdrantError::Client(e.to_string())
    }
}

impl AdapterIdentity for QdrantIndexer {
    fn name(&self) -> &str { "qdrant-indexer" }
    fn version(&self) -> StageVersion { StageVersion("0.1.0".into()) }
    fn config_hash(&self) -> ConfigHash {
        ConfigHash(format!("col={};dim={}", self.collection, self.dim))
    }
}

impl Indexer for QdrantIndexer {
    type Error = QdrantError;

    fn upsert(&self, chunks: &[Chunk], vectors: &[Vector]) -> Result<(), Self::Error> {
        if chunks.len() != vectors.len() {
            return Err(QdrantError::LengthMismatch {
                chunks: chunks.len(),
                vectors: vectors.len(),
            });
        }
        if chunks.is_empty() {
            return Ok(());
        }
        let points = self.build_points(chunks, vectors);
        self.rt.block_on(async {
            self.client
                .upsert_points(UpsertPointsBuilder::new(&self.collection, points).wait(true))
                .await
                .map_err(QdrantError::client)?;
            Ok::<_, QdrantError>(())
        })
    }

    fn replace(
        &self,
        source_id: &SourceId,
        chunks: &[Chunk],
        vectors: &[Vector],
    ) -> Result<(), Self::Error> {
        self.delete_by_source_id(source_id)?;
        if !chunks.is_empty() {
            self.upsert(chunks, vectors)?;
        }
        Ok(())
    }

    fn delete_by_source_id(&self, source_id: &SourceId) -> Result<(), Self::Error> {
        self.rt.block_on(async {
            let filter = Filter::must([Condition::matches("source_id", source_id.0.clone())]);
            self.client
                .delete_points(
                    DeletePointsBuilder::new(&self.collection)
                        .points(filter)
                        .wait(true),
                )
                .await
                .map_err(QdrantError::client)?;
            Ok::<_, QdrantError>(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_id_is_deterministic() {
        let a = point_id(&SourceId("doc-a".into()), 0);
        let b = point_id(&SourceId("doc-a".into()), 0);
        assert_eq!(a, b);
    }

    #[test]
    fn point_id_distinguishes_index() {
        let a = point_id(&SourceId("doc-a".into()), 0);
        let b = point_id(&SourceId("doc-a".into()), 1);
        assert_ne!(a, b);
    }

    #[test]
    fn point_id_distinguishes_source() {
        let a = point_id(&SourceId("doc-a".into()), 0);
        let b = point_id(&SourceId("doc-b".into()), 0);
        assert_ne!(a, b);
    }
}
