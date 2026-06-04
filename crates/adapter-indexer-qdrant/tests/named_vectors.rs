//! Slice 016: dual-named-vector indexing on a real Qdrant. Demonstrates that
//! a `text` slot and a `code` slot coexist on code chunks.

use adapter_chunker_code::CodeChunker;
use adapter_extractor_code::CodeExtractor;
use adapter_indexer_qdrant::QdrantIndexer;
use librarian_domain::{Chunker, ContentType, Document, Extractor, Indexer, SourceHash, SourceId};
use std::collections::BTreeMap;

fn url() -> String {
    std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6533".into())
}

fn unique(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("librarian-named-{label}-{nanos}")
}

fn write_fixture_file(dir: &std::path::Path) -> std::path::PathBuf {
    let p = dir.join("hello.rs");
    let body: String = (0..100)
        .map(|i| format!("// line {i}\nfn line_{i}(x: i32) -> i32 {{ x + {i} }}\n"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&p, body).unwrap();
    p
}

#[test]
fn code_chunks_carry_both_text_and_code_named_vectors() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_fixture_file(dir.path());
    let doc = Document {
        source_id: SourceId(path.display().to_string()),
        source_hash: SourceHash("h".into()),
        content_type: ContentType::Code,
        path: path.clone(),
        work_id: None,
    };

    let extracted = CodeExtractor.extract(&doc).expect("extract");
    let chunks = CodeChunker::new().chunk(&doc, extracted).expect("chunk");
    assert!(!chunks.is_empty());

    let dim_text = 4u64;
    let dim_code = 8u64;
    let collection = unique("dual");
    let Ok(ix) = QdrantIndexer::open_with_extra_slot(
        &url(),
        &collection,
        dim_text,
        Some(("code".to_string(), dim_code)),
    ) else {
        eprintln!("skip: no Qdrant");
        return;
    };

    let text_vecs: Vec<Vec<f32>> = chunks
        .iter()
        .map(|_| vec![0.1; dim_text as usize])
        .collect();
    let code_vecs: Vec<Vec<f32>> = chunks
        .iter()
        .map(|_| vec![0.2; dim_code as usize])
        .collect();
    let mut named = BTreeMap::new();
    named.insert("text".to_string(), text_vecs);
    named.insert("code".to_string(), code_vecs);

    ix.upsert_named(&chunks, named).expect("upsert_named");

    let count = ix.count().unwrap();
    assert_eq!(count as usize, chunks.len());

    // Both vector slots existing on the same point can be searched.
    let hits_text = ix
        .search(&vec![0.1; dim_text as usize], 1, None)
        .expect("text search");
    assert!(!hits_text.is_empty());
}

#[test]
fn extra_slot_dimension_mismatch_is_rejected_at_search_time() {
    let collection = unique("dim-mismatch");
    let Ok(ix) =
        QdrantIndexer::open_with_extra_slot(&url(), &collection, 4, Some(("code".to_string(), 8)))
    else {
        eprintln!("skip: no Qdrant");
        return;
    };
    // Searching `text` (dim 4) with the code-sized vector should error from Qdrant.
    let r = ix.search(&vec![0.0; 8], 1, None);
    assert!(
        r.is_err(),
        "Qdrant rejects wrong-dim query against text slot"
    );
}

#[test]
fn cross_content_type_query_filters_code_or_book() {
    use librarian_domain::{BookMeta, Chunk, ChunkId, ChunkPayload, Provenance};
    let collection = unique("xtype");
    let dim = 4u64;
    let Ok(ix) = QdrantIndexer::open(&url(), &collection, dim) else {
        eprintln!("skip: no Qdrant");
        return;
    };

    // Two chunks: one Book payload, one Code payload.
    let book_chunk = Chunk {
        chunk_id: ChunkId("b#0".into()),
        source_id: SourceId("book-a".into()),
        chunk_index: 0,
        text: "hexagonal architecture".into(),
        payload: ChunkPayload::Book(BookMeta {
            title: "t".into(),
            author: None,
            chapter: None,
            section: None,
            page: None,
        }),
        provenance: Provenance::default(),
    };
    let code_chunk = {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.rs");
        std::fs::write(&path, "fn x() {}").unwrap();
        let doc = Document {
            source_id: SourceId("code-a".into()),
            source_hash: SourceHash("h".into()),
            content_type: ContentType::Code,
            path,
            work_id: None,
        };
        let ext = CodeExtractor.extract(&doc).unwrap();
        CodeChunker::new()
            .chunk(&doc, ext)
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
    };

    ix.upsert(
        &[book_chunk.clone(), code_chunk.clone()],
        &[vec![0.1; 4], vec![0.2; 4]],
    )
    .unwrap();

    let book_only = ix.search(&vec![0.1; 4], 5, Some("book")).unwrap();
    assert!(book_only.iter().all(|h| h.content_type == "book"));

    let code_only = ix.search(&vec![0.2; 4], 5, Some("code")).unwrap();
    assert!(code_only.iter().all(|h| h.content_type == "code"));
}
