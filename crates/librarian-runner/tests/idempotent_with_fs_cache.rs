//! Slice 007 against the FS cache — cache hits survive across runner instances
//! (process-restart equivalent).

use adapter_cache_fs::FsCache;
use adapter_chunker_blankline::BlankLineChunker;
use adapter_indexer_mem::MemIndexer;
use adapter_manifest_mem::MemManifest;
use librarian_domain::{
    AdapterIdentity, ConfigHash, ContentType, Document, EmbedderError, Embedder, ExtractedText,
    Extractor, SourceHash, SourceId, SpanKind, StageVersion, TextSpan, Vector,
};
use librarian_runner::{BatchRunner, Pipeline};
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::tempdir;

struct CountingExtractor { calls: AtomicUsize }
impl AdapterIdentity for CountingExtractor {
    fn name(&self) -> &str { "ce" }
    fn version(&self) -> StageVersion { StageVersion("v".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash("c".into()) }
}
#[derive(Debug, thiserror::Error)] #[error("never")] struct EErr;
impl Extractor for CountingExtractor {
    type Error = EErr;
    fn extract(&self, doc: &Document) -> Result<ExtractedText, Self::Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ExtractedText { spans: vec![TextSpan {
            kind: SpanKind::Paragraph, text: format!("body-{}", doc.source_id.0),
            page: None, byte_range: 0..1,
        }]})
    }
}

struct CountingEmbedder { calls: AtomicUsize }
impl AdapterIdentity for CountingEmbedder {
    fn name(&self) -> &str { "ce-emb" }
    fn version(&self) -> StageVersion { StageVersion("v".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash("c".into()) }
}
impl Embedder for CountingEmbedder {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(texts.iter().map(|t| vec![t.len() as f32, 0.0]).collect())
    }
    fn dimension(&self) -> usize { 2 }
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

#[test]
fn fs_cache_persists_outputs_across_runner_instances() {
    let dir = tempdir().unwrap();

    // First runner — populates FsCache.
    {
        let r = BatchRunner {
            pipeline: Pipeline {
                extractor: CountingExtractor { calls: AtomicUsize::new(0) },
                chunker: BlankLineChunker::new(),
                embedder: CountingEmbedder { calls: AtomicUsize::new(0) },
                indexer: MemIndexer::new(),
            },
            manifest: MemManifest::new(),
            cache: FsCache::open(dir.path()).unwrap(),
        };
        r.ingest_batch(&[doc("d0"), doc("d1")]);
        assert_eq!(r.pipeline.extractor.calls.load(Ordering::SeqCst), 2);
        assert_eq!(r.pipeline.embedder.calls.load(Ordering::SeqCst), 2);
    }

    // Second runner — fresh adapters, fresh manifest, but the SAME FsCache root.
    let r2 = BatchRunner {
        pipeline: Pipeline {
            extractor: CountingExtractor { calls: AtomicUsize::new(0) },
            chunker: BlankLineChunker::new(),
            embedder: CountingEmbedder { calls: AtomicUsize::new(0) },
            indexer: MemIndexer::new(),
        },
        manifest: MemManifest::new(),
        cache: FsCache::open(dir.path()).unwrap(),
    };
    r2.ingest_batch(&[doc("d0"), doc("d1")]);
    assert_eq!(r2.pipeline.extractor.calls.load(Ordering::SeqCst), 0, "extract cache survived");
    assert_eq!(r2.pipeline.embedder.calls.load(Ordering::SeqCst), 0, "embed cache survived");
    // Indexer is always called and is idempotent.
    assert_eq!(r2.pipeline.indexer.count(), 2);
}
