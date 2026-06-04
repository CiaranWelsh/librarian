//! `QdrantIndexer` — sync `Indexer` wrapping an internal Tokio runtime so the
//! domain stays sync. Reserves the `text` named vector slot at collection-init
//! time; slices 016/017 may register extra slots (`code`, `figure`).

use librarian_domain::{
    AdapterIdentity, Chunk, ConfigHash, Indexer, SourceId, StageVersion, Vector,
};
use qdrant_client::qdrant::{
    point_id::PointIdOptions, vectors::VectorsOptions, vectors_config::Config as VectorsCfg,
    Condition, CreateCollectionBuilder, DeletePointsBuilder, Distance, FieldType, Filter,
    NamedVectors, PointId, PointStruct, UpsertPointsBuilder, Vector as QVector, VectorParams,
    VectorParamsMap, VectorsConfig,
};
use qdrant_client::Qdrant;
use std::collections::HashMap;

use crate::error::QdrantError;
use crate::payload::build_payload;
use crate::point_id::point_id;
use crate::search::SearchHit;

pub struct QdrantIndexer {
    rt: tokio::runtime::Runtime,
    client: Qdrant,
    collection: String,
    dim: u64,
    /// Additional named vector slots ("code", "figure", etc.) reserved at
    /// collection-init time. `upsert_named` may populate them per chunk.
    extra_slots: Vec<(String, u64)>,
}

impl QdrantIndexer {
    /// Open a connection and ensure the collection exists with a `text` named
    /// vector slot of dimension `dim`. Idempotent.
    pub fn open(url: &str, collection: &str, dim: u64) -> Result<Self, QdrantError> {
        Self::open_with_slots(url, collection, dim, vec![])
    }

    /// Open with a single extra slot — convenience for slice 016 callers.
    pub fn open_with_extra_slot(
        url: &str,
        collection: &str,
        dim: u64,
        extra_slot: Option<(String, u64)>,
    ) -> Result<Self, QdrantError> {
        Self::open_with_slots(url, collection, dim, extra_slot.into_iter().collect())
    }

    /// Open with multiple extra named-vector slots (e.g. `[("code", 1024), ("figure", 512)]`).
    /// Slots are created at collection-init time; `upsert_named` populates them.
    pub fn open_with_slots(
        url: &str,
        collection: &str,
        dim: u64,
        extra_slots: Vec<(String, u64)>,
    ) -> Result<Self, QdrantError> {
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
            extra_slots,
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
                for (name, dim) in &self.extra_slots {
                    params.insert(
                        name.clone(),
                        VectorParams {
                            size: *dim,
                            distance: Distance::Cosine.into(),
                            ..Default::default()
                        },
                    );
                }
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
                req = req.filter(Filter::must([Condition::matches(
                    "content_type",
                    ct.to_string(),
                )]));
            }
            let r = self
                .client
                .search_points(req)
                .await
                .map_err(QdrantError::client)?;
            Ok(r.result
                .into_iter()
                .map(|p| {
                    let payload_get = |k: &str| -> Option<String> {
                        p.payload
                            .get(k)
                            .and_then(|v| v.as_str().map(|s| s.to_string()))
                    };
                    SearchHit {
                        score: p.score,
                        source_id: payload_get("source_id").unwrap_or_default(),
                        chunk_index: p
                            .payload
                            .get("chunk_index")
                            .and_then(|v| v.as_integer())
                            .unwrap_or(0) as u32,
                        text: payload_get("text").unwrap_or_default(),
                        content_type: payload_get("content_type").unwrap_or_default(),
                    }
                })
                .collect())
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
            let r = self
                .client
                .scroll(
                    ScrollPointsBuilder::new(&self.collection)
                        .filter(filter)
                        .with_payload(true)
                        .limit(1024),
                )
                .await
                .map_err(QdrantError::client)?;
            let mut hits: Vec<(u32, String)> = r
                .result
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
                    Some((idx, text))
                })
                .collect();
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

    /// Upsert chunks with multiple named vectors per chunk. `named_vectors` maps
    /// slot name (e.g. `"text"`, `"code"`) to a `Vec<Vector>` of equal length.
    /// All slots must have len == chunks.len(). Use `Vec<f32>::new()` to skip
    /// a slot for one chunk — Qdrant will still record the point.
    pub fn upsert_named(
        &self,
        chunks: &[Chunk],
        named_vectors: std::collections::BTreeMap<String, Vec<Vector>>,
    ) -> Result<(), QdrantError> {
        if chunks.is_empty() {
            return Ok(());
        }
        for (slot, vs) in &named_vectors {
            if vs.len() != chunks.len() {
                return Err(QdrantError::LengthMismatch {
                    chunks: chunks.len(),
                    vectors: vs.len(),
                });
            }
            let _ = slot;
        }
        let points: Vec<PointStruct> = chunks
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let id = PointId {
                    point_id_options: Some(PointIdOptions::Uuid(
                        point_id(&c.source_id, c.chunk_index).to_string(),
                    )),
                };
                let mut named = HashMap::new();
                for (slot, vs) in &named_vectors {
                    if !vs[i].is_empty() {
                        named.insert(slot.clone(), QVector::from(vs[i].clone()));
                    }
                }
                let vectors = qdrant_client::qdrant::Vectors {
                    vectors_options: Some(VectorsOptions::Vectors(NamedVectors { vectors: named })),
                };
                PointStruct {
                    id: Some(id),
                    vectors: Some(vectors),
                    payload: build_payload(c).into(),
                }
            })
            .collect();

        self.rt.block_on(async {
            self.client
                .upsert_points(UpsertPointsBuilder::new(&self.collection, points).wait(true))
                .await
                .map_err(QdrantError::client)?;
            Ok::<_, QdrantError>(())
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

impl AdapterIdentity for QdrantIndexer {
    fn name(&self) -> &str {
        "qdrant-indexer"
    }
    fn version(&self) -> StageVersion {
        StageVersion("0.1.0".into())
    }
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
