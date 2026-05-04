//! Slice 011 integration: the runner records `RecoveredViaFallback` when a
//! `FallbackEmbedder` recovers, and combined-error `Failed` when both fail.

use adapter_cache_mem::MemCache;
use adapter_chunker_blankline::BlankLineChunker;
use adapter_embedder_fallback::FallbackEmbedder;
use adapter_indexer_mem::MemIndexer;
use adapter_manifest_mem::MemManifest;
use librarian_domain::{
    AdapterIdentity, ConfigHash, ContentType, Document, EmbedderError, Embedder, ExtractedText,
    Extractor, ManifestStatus, ManifestStore, SourceHash, SourceId, SpanKind, StageVersion,
    TextSpan, Vector,
};
use librarian_runner::{BatchRunner, Pipeline};
use std::cell::Cell;

struct OkExt;
impl AdapterIdentity for OkExt {
    fn name(&self) -> &str { "ok" }
    fn version(&self) -> StageVersion { StageVersion("v".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash("c".into()) }
}
#[derive(Debug, thiserror::Error)] #[error("never")] struct EE;
impl Extractor for OkExt {
    type Error = EE;
    fn extract(&self, doc: &Document) -> Result<ExtractedText, Self::Error> {
        Ok(ExtractedText { spans: vec![TextSpan {
            kind: SpanKind::Paragraph, text: format!("body-{}", doc.source_id.0),
            page: None, byte_range: 0..1,
        }]})
    }
}

struct StubEmbedderOnce {
    name: &'static str,
    next: Cell<Option<Result<Vec<Vector>, EmbedderError>>>,
}
impl StubEmbedderOnce {
    fn ok_unit(n: &'static str) -> Self {
        Self { name: n, next: Cell::new(Some(Ok(vec![vec![1.0; 4]]))) }
    }
    fn err(n: &'static str, e: EmbedderError) -> Self {
        Self { name: n, next: Cell::new(Some(Err(e))) }
    }
}
impl AdapterIdentity for StubEmbedderOnce {
    fn name(&self) -> &str { self.name }
    fn version(&self) -> StageVersion { StageVersion("v".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash("c".into()) }
}
impl Embedder for StubEmbedderOnce {
    fn embed(&self, _: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        self.next.replace(None).expect("stub configured for one call")
    }
    fn dimension(&self) -> usize { 4 }
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
fn manifest_records_recovered_via_fallback_when_primary_recoverable() {
    let primary = StubEmbedderOnce::err("primary", EmbedderError::Recoverable("rate-limit".into()));
    let fallback = StubEmbedderOnce::ok_unit("fallback");
    let combined = FallbackEmbedder::new(primary, fallback);

    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: OkExt, chunker: BlankLineChunker::new(),
            embedder: combined, indexer: MemIndexer::new(),
        },
        manifest: MemManifest::new(), cache: MemCache::new(),
    };

    runner.ingest_batch(&[doc("d0")]);

    let recovered = runner.manifest.list_by_status(ManifestStatus::RecoveredViaFallback).unwrap();
    assert_eq!(recovered, vec![(SourceId("d0".into()), "embed".into())],
               "manifest reflects recovery");

    // Vector still landed in the indexer.
    assert_eq!(runner.pipeline.indexer.count(), 1);

    // Embed stage row carries the primary error message.
    let rows = runner.manifest.rows();
    let embed_row = rows.iter().find(|r| r.stage == "embed").unwrap();
    assert_eq!(embed_row.status, ManifestStatus::RecoveredViaFallback);
    assert!(embed_row.error.as_deref().unwrap().contains("rate-limit"));
}

#[test]
fn manifest_records_failed_with_both_errors_when_fallback_also_terminal() {
    let primary = StubEmbedderOnce::err("primary", EmbedderError::Recoverable("rate-limit".into()));
    let fallback = StubEmbedderOnce::err("fallback", EmbedderError::Terminal("auth fail".into()));
    let combined = FallbackEmbedder::new(primary, fallback);

    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: OkExt, chunker: BlankLineChunker::new(),
            embedder: combined, indexer: MemIndexer::new(),
        },
        manifest: MemManifest::new(), cache: MemCache::new(),
    };

    runner.ingest_batch(&[doc("d0")]);
    assert_eq!(runner.pipeline.indexer.count(), 0, "no chunks indexed on terminal failure");

    let failed = runner.manifest.list_by_status(ManifestStatus::Failed).unwrap();
    assert_eq!(failed, vec![(SourceId("d0".into()), "embed".into())]);

    let row = runner.manifest.rows().into_iter().find(|r| r.stage == "embed").unwrap();
    let err = row.error.expect("error message");
    assert!(err.contains("rate-limit"), "primary message preserved: {err}");
    assert!(err.contains("auth fail"), "fallback message preserved: {err}");
}

#[test]
fn primary_terminal_skips_fallback_and_records_simple_failed() {
    let primary = StubEmbedderOnce::err("primary", EmbedderError::Terminal("invariant".into()));
    let fallback = StubEmbedderOnce::ok_unit("fallback"); // would succeed, but should be skipped
    let combined = FallbackEmbedder::new(primary, fallback);

    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: OkExt, chunker: BlankLineChunker::new(),
            embedder: combined, indexer: MemIndexer::new(),
        },
        manifest: MemManifest::new(), cache: MemCache::new(),
    };

    runner.ingest_batch(&[doc("d0")]);

    let failed = runner.manifest.list_by_status(ManifestStatus::Failed).unwrap();
    assert_eq!(failed, vec![(SourceId("d0".into()), "embed".into())]);
    let row = runner.manifest.rows().into_iter().find(|r| r.stage == "embed").unwrap();
    let err = row.error.expect("error");
    assert!(err.contains("invariant"));
    assert!(!err.contains("primary:"), "no fallback occurred so no combined message");
}
