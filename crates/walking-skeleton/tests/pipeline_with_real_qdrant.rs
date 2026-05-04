//! Cross-cutting integration: the full pipeline with the **real Qdrant**
//! indexer instead of the in-memory one. Other stages stay deterministic
//! (text extractor on real fs, blank-line chunker, stub embedder).
//!
//! Gated on `LIBRARIAN_QDRANT_URL` (default `http://localhost:6533`); skips
//! cleanly if Qdrant unreachable.

use adapter_chunker_blankline::BlankLineChunker;
use adapter_embedder_stub::StubEmbedder;
use adapter_extractor_text::TextExtractor;
use adapter_indexer_qdrant::QdrantIndexer;
use librarian_domain::{ContentType, Document, Embedder, SourceHash, SourceId};
use librarian_runner::Pipeline;

fn qdrant_url() -> String {
    std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6533".into())
}

fn unique_collection(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("librarian-pipeline-{label}-{nanos}")
}

fn fixture_path() -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.push("tests/fixtures/sample.txt");
    p
}

#[test]
fn three_paragraph_fixture_lands_three_points_in_qdrant() {
    let collection = unique_collection("three");
    let stub = StubEmbedder::new();
    let dim = stub.dimension() as u64;
    let Ok(ix) = QdrantIndexer::open(&qdrant_url(), &collection, dim) else {
        eprintln!("skip: no Qdrant at {}", qdrant_url());
        return;
    };

    let doc = Document {
        source_id: SourceId(format!("{}-doc", collection)),
        source_hash: SourceHash("h".into()),
        content_type: ContentType::Book,
        path: fixture_path(),
        work_id: None,
    };

    let pipeline = Pipeline {
        extractor: TextExtractor::new(),
        chunker: BlankLineChunker::new(),
        embedder: stub,
        indexer: ix,
    };

    let summary = pipeline.run(&doc).expect("pipeline run");
    assert_eq!(summary.chunks_indexed, 3);
    assert_eq!(pipeline.indexer.count().unwrap(), 3);
    assert_eq!(pipeline.indexer.count_by_source(&doc.source_id).unwrap(), 3);
}

#[test]
fn rerun_is_idempotent_in_qdrant_thanks_to_deterministic_point_ids() {
    let collection = unique_collection("idem");
    let stub = StubEmbedder::new();
    let dim = stub.dimension() as u64;
    let Ok(ix) = QdrantIndexer::open(&qdrant_url(), &collection, dim) else {
        eprintln!("skip: no Qdrant at {}", qdrant_url());
        return;
    };

    let doc = Document {
        source_id: SourceId(format!("{}-doc", collection)),
        source_hash: SourceHash("h".into()),
        content_type: ContentType::Book,
        path: fixture_path(),
        work_id: None,
    };

    let pipeline = Pipeline {
        extractor: TextExtractor::new(),
        chunker: BlankLineChunker::new(),
        embedder: stub,
        indexer: ix,
    };

    pipeline.run(&doc).expect("first run");
    pipeline.run(&doc).expect("second run");
    // Deterministic point IDs (UUID v5 from source_id + chunk_index) mean the
    // second run upserts the same points in place — no duplicates.
    assert_eq!(pipeline.indexer.count_by_source(&doc.source_id).unwrap(), 3);
}
