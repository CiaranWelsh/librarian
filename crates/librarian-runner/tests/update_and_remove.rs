//! Slice 008: atomic update (no orphans) and explicit `remove`.

use adapter_cache_mem::MemCache;
use adapter_chunker_blankline::BlankLineChunker;
use adapter_embedder_stub::StubEmbedder;
use adapter_indexer_mem::MemIndexer;
use adapter_manifest_mem::MemManifest;
use librarian_domain::{
    AdapterIdentity, ConfigHash, ContentType, Document, ExtractedText, Extractor, ManifestStatus,
    ManifestStore, SourceHash, SourceId, SpanKind, StageVersion, TextSpan,
};
use librarian_runner::{BatchRunner, Pipeline};
use std::cell::RefCell;

/// Extractor whose output for a given source_id is controlled per call.
/// Lets us simulate "the file changed" without involving the real fs.
struct ScriptedExtractor {
    next: RefCell<Vec<&'static str>>,
}
impl ScriptedExtractor {
    fn new(scripts: Vec<&'static str>) -> Self {
        Self {
            next: RefCell::new(scripts),
        }
    }
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
    fn extract(&self, _doc: &Document) -> Result<ExtractedText, Self::Error> {
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

#[test]
fn update_with_fewer_chunks_drops_orphans() {
    // 5 paragraphs → 3 paragraphs after edit. The two trailing chunks must vanish.
    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: ScriptedExtractor::new(vec!["p0\n\np1\n\np2\n\np3\n\np4", "p0\n\np1\n\np2"]),
            chunker: BlankLineChunker::new(),
            embedder: StubEmbedder::new(),
            indexer: MemIndexer::new(),
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
            .by_source(&SourceId("d0".into()))
            .len(),
        5
    );

    // Source content changed → new source_hash. Cache for prior hash doesn't apply.
    runner.ingest_batch(&[doc("d0", "h-edited")]);
    assert_eq!(
        runner
            .pipeline
            .indexer
            .by_source(&SourceId("d0".into()))
            .len(),
        3,
        "no orphans from prior 5-chunk version",
    );
}

#[test]
fn update_does_not_disturb_other_sources() {
    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: ScriptedExtractor::new(vec![
                "a0\n\na1\n\na2", // d_a first ingest
                "b0\n\nb1",       // d_b first ingest
                "a0",             // d_a edited down to 1 chunk
            ]),
            chunker: BlankLineChunker::new(),
            embedder: StubEmbedder::new(),
            indexer: MemIndexer::new(),
        },
        manifest: MemManifest::new(),
        cache: MemCache::new(),
        quality: librarian_domain::QualityConfig::default(),
    };

    runner.ingest_batch(&[doc("d_a", "ha-1"), doc("d_b", "hb-1")]);
    assert_eq!(runner.pipeline.indexer.count(), 5);

    runner.ingest_batch(&[doc("d_a", "ha-2")]);
    assert_eq!(
        runner
            .pipeline
            .indexer
            .by_source(&SourceId("d_a".into()))
            .len(),
        1
    );
    assert_eq!(
        runner
            .pipeline
            .indexer
            .by_source(&SourceId("d_b".into()))
            .len(),
        2,
        "d_b untouched by d_a's update"
    );
}

#[test]
fn remove_drops_all_chunks_and_records_removed_status() {
    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: ScriptedExtractor::new(vec!["p0\n\np1\n\np2"]),
            chunker: BlankLineChunker::new(),
            embedder: StubEmbedder::new(),
            indexer: MemIndexer::new(),
        },
        manifest: MemManifest::new(),
        cache: MemCache::new(),
        quality: librarian_domain::QualityConfig::default(),
    };

    runner.ingest_batch(&[doc("d0", "h0")]);
    assert_eq!(runner.pipeline.indexer.count(), 3);

    runner.remove(&SourceId("d0".into())).expect("remove");
    assert_eq!(runner.pipeline.indexer.count(), 0);

    let removed = runner
        .manifest
        .list_by_status(ManifestStatus::Removed)
        .unwrap();
    let stages: std::collections::HashSet<_> = removed.iter().map(|(_, s)| s.as_str()).collect();
    for stage in ["extract", "chunk", "embed", "index"] {
        assert!(
            stages.contains(stage),
            "Removed status missing for stage {stage}"
        );
    }
}

#[test]
fn remove_of_unknown_source_is_a_noop() {
    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: ScriptedExtractor::new(vec![]),
            chunker: BlankLineChunker::new(),
            embedder: StubEmbedder::new(),
            indexer: MemIndexer::new(),
        },
        manifest: MemManifest::new(),
        cache: MemCache::new(),
        quality: librarian_domain::QualityConfig::default(),
    };

    runner
        .remove(&SourceId("never-ingested".into()))
        .expect("remove");
    assert_eq!(runner.pipeline.indexer.count(), 0);
    // Still records Removed rows — the manifest is the audit log of operator intent.
    assert_eq!(
        runner
            .manifest
            .list_by_status(ManifestStatus::Removed)
            .unwrap()
            .len(),
        4
    );
}
