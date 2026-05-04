//! Slice 009 AC: end-to-end ingest of one paper through Phase B/C adapters.
//! Verify chunk count and `PaperMeta.page_start` populated.

use adapter_cache_mem::MemCache;
use adapter_chunker_blankline::BlankLineChunker;
use adapter_embedder_stub::StubEmbedder;
use adapter_extractor_pdf::PdfExtractor;
use adapter_indexer_mem::MemIndexer;
use adapter_manifest_mem::MemManifest;
use librarian_domain::{ChunkPayload, ContentType, Document, SourceHash, SourceId};
use librarian_runner::{BatchRunner, Outcome, Pipeline};
use printpdf::*;
use std::io::BufWriter;
use tempfile::tempdir;

fn write_paper(path: &std::path::Path) {
    let (mut doc, page1, layer1) = PdfDocument::new("On Vector DBs", Mm(210.0), Mm(297.0), "L1");
    doc = doc.with_author("Test Author");
    let font = doc.add_builtin_font(BuiltinFont::Helvetica).unwrap();

    let l1 = doc.get_page(page1).get_layer(layer1);
    l1.use_text("Abstract: vectors and indexing.", 12.0, Mm(20.0), Mm(280.0), &font);
    l1.use_text("Section 1: motivation paragraph.", 12.0, Mm(20.0), Mm(264.0), &font);

    let (p2, l2id) = doc.add_page(Mm(210.0), Mm(297.0), "L2");
    let l2 = doc.get_page(p2).get_layer(l2id);
    l2.use_text("Section 2: empirical results.", 12.0, Mm(20.0), Mm(280.0), &font);

    let f = std::fs::File::create(path).unwrap();
    doc.save(&mut BufWriter::new(f)).unwrap();
}

#[test]
fn ingest_paper_through_full_pipeline_populates_paper_meta_with_page_start() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("paper.pdf");
    write_paper(&path);

    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: PdfExtractor::new(),
            chunker: BlankLineChunker::new(),
            embedder: StubEmbedder::new(),
            indexer: MemIndexer::new(),
        },
        manifest: MemManifest::new(),
        cache: MemCache::new(),
    };

    let doc = Document {
        source_id: SourceId("paper-1".into()),
        source_hash: SourceHash("h".into()),
        content_type: ContentType::Paper,
        path,
        work_id: None,
    };

    let outcomes = runner.ingest_batch(&[doc.clone()]);
    assert!(matches!(outcomes[0], Outcome::Success { .. }));

    let pts = runner.pipeline.indexer.by_source(&doc.source_id);
    assert!(pts.len() >= 2, "at least 2 chunks across the two pages (got {})", pts.len());
    let pages: std::collections::HashSet<u32> = pts.iter().filter_map(|p| match &p.chunk.payload {
        ChunkPayload::Paper(meta) => meta.page_start, _ => None,
    }).collect();
    assert!(pages.len() >= 2, "chunks span both pages (got {pages:?})");

    // Every chunk has a Paper payload with page_start populated.
    for p in &pts {
        match &p.chunk.payload {
            ChunkPayload::Paper(meta) => {
                assert!(meta.page_start.is_some(), "page_start populated");
                assert_eq!(meta.title, "paper", "from file stem");
            }
            other => panic!("expected Paper payload, got {other:?}"),
        }
    }
}
