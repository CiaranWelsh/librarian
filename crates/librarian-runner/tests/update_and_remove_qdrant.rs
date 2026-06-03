//! Slice 008 against real Qdrant — update drops orphans, remove drops everything.

use adapter_cache_mem::MemCache;
use adapter_chunker_blankline::BlankLineChunker;
use adapter_embedder_stub::StubEmbedder;
use adapter_indexer_qdrant::QdrantIndexer;
use adapter_manifest_mem::MemManifest;
use librarian_domain::{
    AdapterIdentity, ConfigHash, ContentType, Document, Embedder, ExtractedText, Extractor,
    SourceHash, SourceId, SpanKind, StageVersion, TextSpan,
};
use librarian_runner::{BatchRunner, Pipeline};
use std::cell::RefCell;

struct ScriptedExtractor {
    next: RefCell<Vec<&'static str>>,
}
impl AdapterIdentity for ScriptedExtractor {
    fn name(&self) -> &str {
        "scripted"
    }
    fn version(&self) -> StageVersion {
        StageVersion("v1".into())
    }
    fn config_hash(&self) -> ConfigHash {
        ConfigHash("c".into())
    }
}
#[derive(Debug, thiserror::Error)]
#[error("never")]
struct EErr;
impl Extractor for ScriptedExtractor {
    type Error = EErr;
    fn extract(&self, _: &Document) -> Result<ExtractedText, Self::Error> {
        let body = self.next.borrow_mut().remove(0);
        Ok(ExtractedText {
            spans: vec![TextSpan {
                kind: SpanKind::Paragraph,
                text: body.into(),
                page: None,
                byte_range: 0..body.len(),
            }],
        })
    }
}

fn doc(id: &str, hash: &str) -> Document {
    Document {
        source_id: SourceId(id.into()),
        source_hash: SourceHash(hash.into()),
        content_type: ContentType::Book,
        path: format!("/tmp/{id}").into(),
        work_id: None,
    }
}

fn url() -> String {
    std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6533".into())
}

fn unique(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("librarian-{label}-{nanos}")
}

#[test]
fn update_drops_orphan_chunks_in_qdrant() {
    let stub = StubEmbedder::new();
    let dim = stub.dimension() as u64;
    let Ok(ix) = QdrantIndexer::open(&url(), &unique("update"), dim) else {
        eprintln!("skip: no Qdrant");
        return;
    };

    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: ScriptedExtractor {
                next: RefCell::new(vec!["p0\n\np1\n\np2\n\np3\n\np4", "p0\n\np1\n\np2"]),
            },
            chunker: BlankLineChunker::new(),
            embedder: stub,
            indexer: ix,
        },
        manifest: MemManifest::new(),
        cache: MemCache::new(),
        quality: librarian_domain::QualityConfig::default(),
    };

    runner.ingest_batch(&[doc("d0", "h-original")]);
    assert_eq!(
        runner
            .pipeline
            .indexer
            .count_by_source(&SourceId("d0".into()))
            .unwrap(),
        5
    );

    runner.ingest_batch(&[doc("d0", "h-edited")]);
    assert_eq!(
        runner
            .pipeline
            .indexer
            .count_by_source(&SourceId("d0".into()))
            .unwrap(),
        3
    );
    assert_eq!(runner.pipeline.indexer.count().unwrap(), 3);
}

#[test]
fn explicit_remove_drops_all_chunks_in_qdrant() {
    let stub = StubEmbedder::new();
    let dim = stub.dimension() as u64;
    let Ok(ix) = QdrantIndexer::open(&url(), &unique("remove"), dim) else {
        eprintln!("skip: no Qdrant");
        return;
    };

    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: ScriptedExtractor {
                next: RefCell::new(vec!["p0\n\np1\n\np2"]),
            },
            chunker: BlankLineChunker::new(),
            embedder: stub,
            indexer: ix,
        },
        manifest: MemManifest::new(),
        cache: MemCache::new(),
        quality: librarian_domain::QualityConfig::default(),
    };

    runner.ingest_batch(&[doc("d0", "h0")]);
    assert_eq!(
        runner
            .pipeline
            .indexer
            .count_by_source(&SourceId("d0".into()))
            .unwrap(),
        3
    );

    runner.remove(&SourceId("d0".into())).expect("remove");
    assert_eq!(
        runner
            .pipeline
            .indexer
            .count_by_source(&SourceId("d0".into()))
            .unwrap(),
        0
    );
    assert_eq!(runner.pipeline.indexer.count().unwrap(), 0);
}
