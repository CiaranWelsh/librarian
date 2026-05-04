use adapter_chunker_blankline::BlankLineChunker;
use adapter_embedder_stub::StubEmbedder;
use adapter_extractor_text::TextExtractor;
use adapter_indexer_mem::MemIndexer;
use librarian_domain::{ContentType, Document, SourceHash, SourceId};
use librarian_runner::Pipeline;

fn fixture_path() -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.push("tests/fixtures/sample.txt");
    p
}

#[test]
fn three_paragraphs_become_three_points() {
    let path = fixture_path();
    let doc = Document {
        source_id: SourceId("sample".into()),
        source_hash: SourceHash("deadbeef".into()),
        content_type: ContentType::Book,
        path,
        work_id: None,
    };

    let pipeline = Pipeline {
        extractor: TextExtractor::new(),
        chunker: BlankLineChunker::new(),
        embedder: StubEmbedder::new(),
        indexer: MemIndexer::new(),
    };

    let summary = pipeline.run(&doc).expect("run");
    assert_eq!(summary.chunks_indexed, 3);
    assert_eq!(pipeline.indexer.count(), 3);
    let pts = pipeline.indexer.by_source(&doc.source_id);
    let mut indices: Vec<_> = pts.iter().map(|p| p.chunk.chunk_index).collect();
    indices.sort();
    assert_eq!(indices, vec![0, 1, 2]);
    // Provenance has 3 links: extract, chunk, embed.
    assert_eq!(pts[0].chunk.provenance.0.len(), 3);
}
