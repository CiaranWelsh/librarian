//! Slice 006 against real Qdrant: a failed Document writes nothing to the
//! collection. Gated on `LIBRARIAN_QDRANT_URL` (default localhost:6533).

use adapter_chunker_blankline::BlankLineChunker;
use adapter_embedder_stub::StubEmbedder;
use adapter_indexer_qdrant::QdrantIndexer;
use adapter_manifest_mem::MemManifest;
use librarian_domain::{
    AdapterIdentity, ConfigHash, ContentType, Document, Embedder, EmbedderError, ExtractedText,
    Extractor, ManifestStatus, ManifestStore, SourceHash, SourceId, SpanKind, StageVersion,
    TextSpan, Vector,
};
use librarian_runner::{BatchRunner, Pipeline};

struct OkExtractor;
impl AdapterIdentity for OkExtractor {
    fn name(&self) -> &str {
        "ok-ext"
    }
    fn version(&self) -> StageVersion {
        StageVersion("v".into())
    }
    fn config_hash(&self) -> ConfigHash {
        ConfigHash("c".into())
    }
}
#[derive(Debug, thiserror::Error)]
#[error("never")]
struct EErr;
impl Extractor for OkExtractor {
    type Error = EErr;
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

struct FailingEmbedder {
    marker: &'static str,
    inner: StubEmbedder,
}
impl AdapterIdentity for FailingEmbedder {
    fn name(&self) -> &str {
        "fail-emb"
    }
    fn version(&self) -> StageVersion {
        StageVersion("v".into())
    }
    fn config_hash(&self) -> ConfigHash {
        ConfigHash("c".into())
    }
}
impl Embedder for FailingEmbedder {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        if texts.iter().any(|t| t.contains(self.marker)) {
            return Err(EmbedderError::Recoverable("blip".into()));
        }
        self.inner.embed(texts)
    }
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }
}

fn doc(id: &str) -> Document {
    Document {
        source_id: SourceId(id.into()),
        source_hash: SourceHash(format!("h-{id}")),
        content_type: ContentType::Book,
        path: format!("/tmp/{id}").into(),
        work_id: None,
    }
}

fn url() -> String {
    std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6533".into())
}

#[test]
fn embedder_failure_writes_no_chunks_for_failed_doc_in_qdrant() {
    let stub = StubEmbedder::new();
    let dim = stub.dimension() as u64;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let collection = format!("librarian-fault-{nanos}");

    let Ok(ix) = QdrantIndexer::open(&url(), &collection, dim) else {
        eprintln!("skip: no Qdrant");
        return;
    };

    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: OkExtractor,
            chunker: BlankLineChunker::new(),
            embedder: FailingEmbedder {
                marker: "d2",
                inner: stub,
            },
            indexer: ix,
        },
        manifest: MemManifest::new(),
        cache: adapter_cache_mem::MemCache::new(),
        quality: librarian_domain::QualityConfig::default(),
    };

    let _ = runner.ingest_batch(&[doc("d0"), doc("d1"), doc("d2"), doc("d3")]);

    assert_eq!(
        runner
            .pipeline
            .indexer
            .count_by_source(&SourceId("d2".into()))
            .unwrap(),
        0
    );
    assert_eq!(runner.pipeline.indexer.count().unwrap(), 3);

    let failed = runner
        .manifest
        .list_by_status(ManifestStatus::Failed)
        .unwrap();
    assert_eq!(failed, vec![(SourceId("d2".into()), "embed".into())]);
}
