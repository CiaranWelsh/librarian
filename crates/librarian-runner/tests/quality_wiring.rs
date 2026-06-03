//! ADR-0006: ingest-quality wiring through `BatchRunner`.
//! F-EQ.1 — low-value sections are skipped before extraction and not indexed.
//! F-EQ.2 — the garble signal is advisory: a flagged document is still indexed.

use adapter_cache_mem::MemCache;
use adapter_chunker_blankline::BlankLineChunker;
use adapter_embedder_stub::StubEmbedder;
use adapter_indexer_mem::MemIndexer;
use adapter_manifest_mem::MemManifest;
use librarian_domain::{
    AdapterIdentity, ConfigHash, ContentType, Document, ExtractedText, Extractor, GarbleConfig,
    ManifestStatus, ManifestStore, QualityConfig, SectionConfig, SourceHash, SourceId, SpanKind,
    StageVersion, TextSpan,
};
use librarian_runner::{BatchRunner, Outcome, Pipeline};

/// Extractor that always returns one fixed body — lets us inject clean,
/// mojibake, or letter-spaced text without touching the filesystem.
struct FixedExtractor {
    body: &'static str,
}
impl AdapterIdentity for FixedExtractor {
    fn name(&self) -> &str {
        "fixed"
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
impl Extractor for FixedExtractor {
    type Error = EErr;
    fn extract(&self, _doc: &Document) -> Result<ExtractedText, Self::Error> {
        Ok(ExtractedText {
            spans: vec![TextSpan {
                kind: SpanKind::Paragraph,
                text: self.body.into(),
                page: None,
                byte_range: 0..self.body.len(),
            }],
        })
    }
}

/// The file stem drives the section filter (F-EQ.1).
fn doc(stem: &str) -> Document {
    Document {
        source_id: SourceId(stem.into()),
        source_hash: SourceHash("h0".into()),
        content_type: ContentType::Book,
        path: format!("/tmp/{stem}.pdf").into(),
        work_id: None,
    }
}

type TestRunner =
    BatchRunner<FixedExtractor, BlankLineChunker, StubEmbedder, MemIndexer, MemManifest, MemCache>;

fn runner(body: &'static str, quality: QualityConfig) -> TestRunner {
    BatchRunner {
        pipeline: Pipeline {
            extractor: FixedExtractor { body },
            chunker: BlankLineChunker::new(),
            embedder: StubEmbedder::new(),
            indexer: MemIndexer::new(),
        },
        manifest: MemManifest::new(),
        cache: MemCache::new(),
        quality,
    }
}

fn quality() -> QualityConfig {
    QualityConfig {
        sections: SectionConfig {
            exclude: vec!["index".into()],
            keep: vec![],
        },
        garble: GarbleConfig { flag_above: 1.0 },
    }
}

#[test]
fn low_value_section_is_skipped_and_not_indexed() {
    let r = runner("clean prose here", quality());
    let outcomes = r.ingest_batch(&[doc("Index-of-Terms")]);

    assert!(matches!(outcomes[0], Outcome::Skipped { .. }));
    assert_eq!(
        r.pipeline.indexer.count(),
        0,
        "skipped section is not indexed"
    );
    assert_eq!(
        r.manifest.list_by_status(ManifestStatus::Skipped).unwrap(),
        vec![(SourceId("Index-of-Terms".into()), "section".into())],
    );
}

#[test]
fn garbled_doc_is_flagged_but_still_indexed() {
    // Mojibake lifts the garble signal above the threshold. The flag is
    // advisory, so the document is still ingested (QA-EQ2 / F-EQ.2).
    let r = runner(
        "text \u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD} garbled",
        quality(),
    );
    let outcomes = r.ingest_batch(&[doc("Chapter-01")]);

    assert!(
        outcomes[0].is_success(),
        "garble is advisory; ingestion still succeeds"
    );
    assert!(
        r.pipeline.indexer.count() > 0,
        "flagged doc is still indexed"
    );
    assert_eq!(
        r.manifest.list_by_status(ManifestStatus::Flagged).unwrap(),
        vec![(SourceId("Chapter-01".into()), "quality".into())],
    );
}

#[test]
fn clean_doc_succeeds_without_a_flag() {
    let r = runner("a perfectly clean paragraph of prose", quality());
    let outcomes = r.ingest_batch(&[doc("Chapter-02")]);

    assert!(outcomes[0].is_success());
    assert!(r.pipeline.indexer.count() > 0);
    assert!(
        r.manifest
            .list_by_status(ManifestStatus::Flagged)
            .unwrap()
            .is_empty(),
        "clean extraction is not flagged"
    );
}
