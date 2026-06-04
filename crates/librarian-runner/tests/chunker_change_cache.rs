//! Regression (issue 028 cache bug): changing the chunker must invalidate the embed cache.
//! Otherwise a re-ingest re-chunks (new count) but the embed stage serves stale vectors (old
//! count), and the index stage fails with a length mismatch — observed live as
//! "900 chunks vs 5037 vectors" when switching blankline → recursive.

use adapter_cache_mem::MemCache;
use adapter_indexer_mem::MemIndexer;
use adapter_manifest_mem::MemManifest;
use librarian_domain::{
    AdapterIdentity, BookMeta, Chunk, ChunkId, ChunkPayload, Chunker, ConfigHash, ContentType,
    Document, Embedder, EmbedderError, ExtractedText, Extractor, Provenance, SourceHash, SourceId,
    SpanKind, StageVersion, TextSpan, Vector,
};
use librarian_runner::{BatchRunner, Pipeline};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Ext;
#[derive(Debug, thiserror::Error)]
#[error("never")]
struct NeverErr;
impl AdapterIdentity for Ext {
    fn name(&self) -> &str {
        "ext"
    }
    fn version(&self) -> StageVersion {
        StageVersion("v1".into())
    }
    fn config_hash(&self) -> ConfigHash {
        ConfigHash("c".into())
    }
}
impl Extractor for Ext {
    type Error = NeverErr;
    fn extract(&self, doc: &Document) -> Result<ExtractedText, Self::Error> {
        Ok(ExtractedText {
            spans: vec![TextSpan {
                kind: SpanKind::Paragraph,
                text: format!("body-{}", doc.source_id.0),
                page: None,
                byte_range: 0..1,
            }],
        })
    }
}

/// Chunker producing exactly `n` chunks, with a configurable `config_hash`.
struct VarChunker {
    n: usize,
    cfg: ConfigHash,
}
impl VarChunker {
    fn new(n: usize, cfg: &str) -> Self {
        Self {
            n,
            cfg: ConfigHash(cfg.into()),
        }
    }
}
impl AdapterIdentity for VarChunker {
    fn name(&self) -> &str {
        "var-chunker"
    }
    fn version(&self) -> StageVersion {
        StageVersion("v1".into())
    }
    fn config_hash(&self) -> ConfigHash {
        self.cfg.clone()
    }
}
impl Chunker for VarChunker {
    type Error = NeverErr;
    fn chunk(&self, doc: &Document, _t: ExtractedText) -> Result<Vec<Chunk>, Self::Error> {
        Ok((0..self.n)
            .map(|i| Chunk {
                chunk_id: ChunkId(format!("{}#{i}", doc.source_id.0)),
                source_id: doc.source_id.clone(),
                chunk_index: i as u32,
                text: format!("chunk-{i}"),
                payload: ChunkPayload::Book(BookMeta {
                    title: "t".into(),
                    author: None,
                    chapter: None,
                    section: None,
                    page: None,
                }),
                provenance: Provenance::default(),
            })
            .collect())
    }
}

struct Emb {
    calls: AtomicUsize,
}
impl Emb {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }
    fn count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}
impl AdapterIdentity for Emb {
    fn name(&self) -> &str {
        "emb"
    }
    fn version(&self) -> StageVersion {
        StageVersion("v1".into())
    }
    fn config_hash(&self) -> ConfigHash {
        ConfigHash("e".into())
    }
}
impl Embedder for Emb {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(texts
            .iter()
            .map(|t| vec![t.len() as f32, 0.0, 0.0, 0.0])
            .collect())
    }
    fn dimension(&self) -> usize {
        4
    }
}

fn doc() -> Document {
    Document {
        source_id: SourceId("d0".into()),
        source_hash: SourceHash("h0".into()),
        content_type: ContentType::Book,
        path: "/tmp/d0".into(),
        work_id: None,
    }
}

#[test]
fn chunker_change_invalidates_embed_cache() {
    let cache = MemCache::new();
    let manifest = MemManifest::new();

    // First run: chunker producing 2 chunks → 2 vectors cached.
    {
        let r = BatchRunner {
            pipeline: Pipeline {
                extractor: Ext,
                chunker: VarChunker::new(2, "A"),
                embedder: Emb::new(),
                indexer: MemIndexer::new(),
            },
            manifest: &manifest,
            cache: &cache,
            quality: librarian_domain::QualityConfig::default(),
        };
        let out = r.ingest_batch(&[doc()]);
        assert!(out[0].is_success(), "first run ok: {:?}", out[0]);
        assert_eq!(r.pipeline.embedder.count(), 1);
    }

    // Second run: a DIFFERENT chunker producing 3 chunks, sharing the cache. The embed cache
    // must invalidate (the chunk-set changed) — otherwise we'd index 3 chunks against 2 stale
    // vectors and fail with a length mismatch.
    let r2 = BatchRunner {
        pipeline: Pipeline {
            extractor: Ext,
            chunker: VarChunker::new(3, "B"),
            embedder: Emb::new(),
            indexer: MemIndexer::new(),
        },
        manifest: &manifest,
        cache: &cache,
        quality: librarian_domain::QualityConfig::default(),
    };
    let out2 = r2.ingest_batch(&[doc()]);
    assert!(
        out2[0].is_success(),
        "chunker change must not cause a length mismatch: {:?}",
        out2[0]
    );
    assert_eq!(
        r2.pipeline.embedder.count(),
        1,
        "embed must re-run after the chunker changed (stale vectors otherwise)"
    );
}
