//! Integration test for the `RecursiveChunker` (issue 027): the `Chunker` trait composes the
//! span join → markdown breadcrumb → recursive pipeline → domain `Chunk`s. The chunk *texts*
//! must match the Python-validated pipeline (`fixtures/golden_vectors.json`); indices must be
//! sequential and the `source_id` preserved. Payload specifics are left to the implementation.

use adapter_chunker_recursive::RecursiveChunker;
use librarian_domain::{
    Chunker, ContentType, Document, ExtractedText, SourceHash, SourceId, SpanKind, TextSpan,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct MdCase {
    name: String,
    file: String,
    text: String,
    chunk_size: usize,
    chunk_overlap: usize,
    expected: Vec<String>,
}

#[derive(Deserialize)]
struct Golden {
    md: Vec<MdCase>,
}

fn case(name: &str) -> MdCase {
    let raw = include_str!("../fixtures/golden_vectors.json");
    let golden: Golden = serde_json::from_str(raw).expect("parse golden fixture");
    golden
        .md
        .into_iter()
        .find(|c| c.name == name)
        .expect("golden md case present")
}

#[test]
fn chunk_reproduces_pipeline_and_indexes_sequentially() {
    let c = case("headers_basic");
    let doc = Document {
        source_id: SourceId(c.file.clone()),
        source_hash: SourceHash("h".into()),
        content_type: ContentType::Book,
        path: c.file.clone().into(),
        work_id: None,
    };
    let span = TextSpan {
        kind: SpanKind::Paragraph,
        text: c.text.clone(),
        page: None,
        byte_range: 0..c.text.len(),
    };
    let chunker = RecursiveChunker::with_budget(c.chunk_size, c.chunk_overlap);
    let chunks = chunker
        .chunk(&doc, ExtractedText { spans: vec![span] })
        .expect("chunking succeeds");

    let texts: Vec<&str> = chunks.iter().map(|ch| ch.text.as_str()).collect();
    assert_eq!(
        texts, c.expected,
        "chunk texts must match the validated pipeline"
    );

    let idxs: Vec<u32> = chunks.iter().map(|ch| ch.chunk_index).collect();
    assert_eq!(idxs, vec![0, 1, 2], "chunk_index must be sequential from 0");

    assert!(
        chunks.iter().all(|ch| ch.source_id == doc.source_id),
        "source_id must be preserved on every chunk"
    );
}
