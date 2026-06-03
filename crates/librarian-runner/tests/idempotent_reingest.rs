//! Slice 007: cache-hit short-circuit. Re-running ingest on unchanged input
//! produces zero new embed work.

use adapter_cache_mem::MemCache;
use adapter_chunker_blankline::BlankLineChunker;
use adapter_indexer_mem::MemIndexer;
use adapter_manifest_mem::MemManifest;
use librarian_domain::{
    AdapterIdentity, ConfigHash, ContentType, Document, Embedder, EmbedderError, ExtractedText,
    Extractor, ManifestStatus, ManifestStore, SourceHash, SourceId, SpanKind, StageVersion,
    TextSpan, Vector,
};
use librarian_runner::{BatchRunner, Pipeline};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Real-shape extractor that counts calls, used to detect cache hits.
struct CountingExtractor {
    calls: AtomicUsize,
    name: &'static str,
    version: StageVersion,
    cfg: ConfigHash,
}
impl CountingExtractor {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            name: "counting-ext",
            version: StageVersion("v1".into()),
            cfg: ConfigHash("default".into()),
        }
    }
    fn count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}
impl AdapterIdentity for CountingExtractor {
    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> StageVersion {
        self.version.clone()
    }
    fn config_hash(&self) -> ConfigHash {
        self.cfg.clone()
    }
}
#[derive(Debug, thiserror::Error)]
#[error("never")]
struct EErr;
impl Extractor for CountingExtractor {
    type Error = EErr;
    fn extract(&self, doc: &Document) -> Result<ExtractedText, Self::Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
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

/// Embedder with a configurable config_hash so we can simulate a "version bump".
struct CountingEmbedder {
    calls: AtomicUsize,
    cfg: ConfigHash,
}
impl CountingEmbedder {
    fn new(cfg: &str) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            cfg: ConfigHash(cfg.into()),
        }
    }
    fn count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}
impl AdapterIdentity for CountingEmbedder {
    fn name(&self) -> &str {
        "counting-emb"
    }
    fn version(&self) -> StageVersion {
        StageVersion("v1".into())
    }
    fn config_hash(&self) -> ConfigHash {
        self.cfg.clone()
    }
}
impl Embedder for CountingEmbedder {
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

fn doc(id: &str, hash: &str) -> Document {
    Document {
        source_id: SourceId(id.into()),
        source_hash: SourceHash(hash.into()),
        content_type: ContentType::Book,
        path: format!("/tmp/{id}").into(),
        work_id: None,
    }
}

fn make_runner(
    emb_cfg: &str,
) -> BatchRunner<
    CountingExtractor,
    BlankLineChunker,
    CountingEmbedder,
    MemIndexer,
    MemManifest,
    MemCache,
> {
    BatchRunner {
        pipeline: Pipeline {
            extractor: CountingExtractor::new(),
            chunker: BlankLineChunker::new(),
            embedder: CountingEmbedder::new(emb_cfg),
            indexer: MemIndexer::new(),
        },
        manifest: MemManifest::new(),
        cache: MemCache::new(),
        quality: librarian_domain::QualityConfig::default(),
    }
}

#[test]
fn second_run_with_unchanged_input_calls_no_adapter_stages() {
    let runner = make_runner("emb-default");
    let docs = [doc("d0", "h0"), doc("d1", "h1")];

    runner.ingest_batch(&docs);
    assert_eq!(
        runner.pipeline.extractor.count(),
        2,
        "first run: extract per doc"
    );
    assert_eq!(
        runner.pipeline.embedder.count(),
        2,
        "first run: embed per doc"
    );

    runner.ingest_batch(&docs);
    assert_eq!(
        runner.pipeline.extractor.count(),
        2,
        "second run: extract cached"
    );
    assert_eq!(
        runner.pipeline.embedder.count(),
        2,
        "second run: embed cached"
    );

    // Manifest reflects: most recent rows are Cached for extract/chunk/embed.
    let cached = runner
        .manifest
        .list_by_status(ManifestStatus::Cached)
        .unwrap();
    let cached_stages: std::collections::HashSet<_> =
        cached.iter().map(|(_, s)| s.as_str()).collect();
    assert!(cached_stages.contains("extract"));
    assert!(cached_stages.contains("chunk"));
    assert!(cached_stages.contains("embed"));
}

#[test]
fn embedder_config_change_busts_only_embed_cache() {
    // Two runners share the same cache + manifest so we can swap the embedder out.
    let cache = MemCache::new();
    let manifest = MemManifest::new();

    // First run: original embedder config.
    {
        let r = BatchRunner {
            pipeline: Pipeline {
                extractor: CountingExtractor::new(),
                chunker: BlankLineChunker::new(),
                embedder: CountingEmbedder::new("v1"),
                indexer: MemIndexer::new(),
            },
            manifest: &manifest,
            cache: &cache,
            quality: librarian_domain::QualityConfig::default(),
        };
        r.ingest_batch(&[doc("d0", "h0")]);
        assert_eq!(r.pipeline.extractor.count(), 1);
        assert_eq!(r.pipeline.embedder.count(), 1);
    }

    // Second run: bumped embedder config. Extract+chunk hit; embed misses.
    let r2 = BatchRunner {
        pipeline: Pipeline {
            extractor: CountingExtractor::new(),
            chunker: BlankLineChunker::new(),
            embedder: CountingEmbedder::new("v2"),
            indexer: MemIndexer::new(),
        },
        manifest: &manifest,
        cache: &cache,
        quality: librarian_domain::QualityConfig::default(),
    };
    r2.ingest_batch(&[doc("d0", "h0")]);
    assert_eq!(
        r2.pipeline.extractor.count(),
        0,
        "extract cached across version bump"
    );
    assert_eq!(
        r2.pipeline.embedder.count(),
        1,
        "embed re-runs after config change"
    );
}

#[test]
fn adding_a_new_document_to_the_tree_triggers_exactly_one_new_embed() {
    let runner = make_runner("emb-default");
    runner.ingest_batch(&[doc("d0", "h0"), doc("d1", "h1")]);
    assert_eq!(runner.pipeline.embedder.count(), 2);

    runner.ingest_batch(&[doc("d0", "h0"), doc("d1", "h1"), doc("d2", "h2")]);
    assert_eq!(
        runner.pipeline.embedder.count(),
        3,
        "only the new doc embeds"
    );
    assert_eq!(
        runner.pipeline.extractor.count(),
        3,
        "only the new doc extracts"
    );
}
