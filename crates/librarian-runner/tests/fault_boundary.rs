//! Slice 006: per-document fault boundary. One bad Document never halts the batch.

use adapter_chunker_blankline::BlankLineChunker;
use adapter_embedder_stub::StubEmbedder;
use adapter_indexer_mem::MemIndexer;
use adapter_manifest_mem::MemManifest;
use librarian_domain::{
    AdapterIdentity, ConfigHash, ContentType, Document, EmbedderError, Embedder, Extractor,
    ExtractedText, ManifestStatus, ManifestStore, SourceHash, SourceId, SpanKind, StageVersion,
    TextSpan, Vector,
};
use librarian_runner::{BatchRunner, Outcome, Pipeline};

/// Extractor that fails on a configurable subset of source_ids and otherwise
/// returns one paragraph span. Real adapter shape so the rest of the pipeline runs.
struct PartialFailExtractor {
    fail_on: Vec<&'static str>,
}
impl AdapterIdentity for PartialFailExtractor {
    fn name(&self) -> &str { "partial-fail-extract" }
    fn version(&self) -> StageVersion { StageVersion("v".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash("c".into()) }
}
#[derive(Debug, thiserror::Error)] #[error("extract bomb on {0}")] struct EErr(String);
impl Extractor for PartialFailExtractor {
    type Error = EErr;
    fn extract(&self, doc: &Document) -> Result<ExtractedText, Self::Error> {
        if self.fail_on.iter().any(|s| *s == doc.source_id.0) {
            return Err(EErr(doc.source_id.0.clone()));
        }
        Ok(ExtractedText { spans: vec![TextSpan {
            kind: SpanKind::Paragraph, text: format!("body-{}", doc.source_id.0), page: None, byte_range: 0..1,
        }]})
    }
}

/// Embedder that returns Recoverable for configured source_ids' chunks.
/// Slightly tricky: the embedder sees text, not source_id, so we encode the
/// source_id into the chunk text via the extractor above (`body-<id>`).
struct PartialFailEmbedder {
    fail_marker: &'static str,
    inner: StubEmbedder,
}
impl AdapterIdentity for PartialFailEmbedder {
    fn name(&self) -> &str { "partial-fail-embed" }
    fn version(&self) -> StageVersion { StageVersion("v".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash("c".into()) }
}
impl Embedder for PartialFailEmbedder {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        if texts.iter().any(|t| t.contains(self.fail_marker)) {
            return Err(EmbedderError::Recoverable("simulated network blip".into()));
        }
        self.inner.embed(texts)
    }
    fn dimension(&self) -> usize { self.inner.dimension() }
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
fn extractor_failure_on_one_doc_does_not_halt_batch() {
    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: PartialFailExtractor { fail_on: vec!["d2"] },
            chunker: BlankLineChunker::new(),
            embedder: StubEmbedder::new(),
            indexer: MemIndexer::new(),
        },
        manifest: MemManifest::new(),
    };

    let outcomes = runner.ingest_batch(&[doc("d0"), doc("d1"), doc("d2"), doc("d3"), doc("d4")]);

    let succ: Vec<_> = outcomes.iter().filter(|o| o.is_success()).collect();
    let fail: Vec<_> = outcomes.iter().filter(|o| !o.is_success()).collect();
    assert_eq!(succ.len(), 4);
    assert_eq!(fail.len(), 1);

    if let Outcome::Failed { source_id, stage, error } = fail[0] {
        assert_eq!(source_id.0, "d2");
        assert_eq!(*stage, "extract");
        assert!(error.contains("d2"), "error preserves stage detail: {error}");
    } else { unreachable!() }

    // Manifest reflects the same 4-success / 1-failure split.
    assert_eq!(runner.manifest.list_by_status(ManifestStatus::Success).unwrap().len(), 4 * 4); // 4 docs × 4 stages
    assert_eq!(runner.manifest.list_by_status(ManifestStatus::Failed).unwrap().len(), 1);

    // Indexer has 4 docs' chunks (1 each from the trivial extractor).
    assert_eq!(runner.pipeline.indexer.count(), 4);
}

#[test]
fn embedder_failure_does_not_write_to_indexer_for_failed_doc() {
    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: PartialFailExtractor { fail_on: vec![] },
            chunker: BlankLineChunker::new(),
            embedder: PartialFailEmbedder { fail_marker: "d2", inner: StubEmbedder::new() },
            indexer: MemIndexer::new(),
        },
        manifest: MemManifest::new(),
    };

    let _ = runner.ingest_batch(&[doc("d0"), doc("d1"), doc("d2"), doc("d3")]);

    // d2 chunks must not appear in the indexer.
    assert_eq!(runner.pipeline.indexer.by_source(&SourceId("d2".into())).len(), 0);
    assert_eq!(runner.pipeline.indexer.count(), 3);

    // Manifest: failure is recorded against the embed stage for d2.
    let failed = runner.manifest.list_by_status(ManifestStatus::Failed).unwrap();
    assert_eq!(failed, vec![(SourceId("d2".into()), "embed".into())]);
}

#[test]
fn empty_batch_is_a_noop_with_no_outcomes() {
    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: PartialFailExtractor { fail_on: vec![] },
            chunker: BlankLineChunker::new(),
            embedder: StubEmbedder::new(),
            indexer: MemIndexer::new(),
        },
        manifest: MemManifest::new(),
    };
    assert!(runner.ingest_batch(&[]).is_empty());
    assert_eq!(runner.pipeline.indexer.count(), 0);
}

#[test]
fn outcomes_preserve_input_order() {
    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: PartialFailExtractor { fail_on: vec!["d1", "d3"] },
            chunker: BlankLineChunker::new(),
            embedder: StubEmbedder::new(),
            indexer: MemIndexer::new(),
        },
        manifest: MemManifest::new(),
    };
    let outcomes = runner.ingest_batch(&[doc("d0"), doc("d1"), doc("d2"), doc("d3")]);
    let ids: Vec<_> = outcomes.iter().map(|o| o.source_id().0.clone()).collect();
    assert_eq!(ids, vec!["d0", "d1", "d2", "d3"]);
    assert_eq!(outcomes[0].is_success(), true);
    assert_eq!(outcomes[1].is_success(), false);
    assert_eq!(outcomes[2].is_success(), true);
    assert_eq!(outcomes[3].is_success(), false);
}
