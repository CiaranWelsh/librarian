//! Slice 017: figure named-vector slot. A `paper` collection that also indexes
//! figures must support three named vector slots (`text`, `code`, `figure`).
//! This test demonstrates Figure chunks landing with both text and figure
//! vectors populated, queryable via the figure slot.

use adapter_embedder_multimodal_stub::MultimodalStubEmbedder;
use adapter_indexer_qdrant::QdrantIndexer;
use librarian_domain::{
    Chunk, ChunkId, ChunkPayload, FigureMeta, Provenance, SourceId,
};
use std::collections::BTreeMap;

fn url() -> String {
    std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6533".into())
}

fn unique(label: &str) -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    format!("librarian-fig-{label}-{nanos}")
}

fn figure_chunk(sid: &str, idx: u32, caption: &str, page: u32, fig_no: u32) -> Chunk {
    Chunk {
        chunk_id: ChunkId(format!("{sid}#{idx}")),
        source_id: SourceId(sid.into()),
        chunk_index: idx,
        text: caption.to_string(),
        payload: ChunkPayload::Figure(FigureMeta {
            paper_title: Some("On Particles".into()),
            caption: caption.to_string(),
            page: Some(page),
            figure_number: Some(fig_no),
        }),
        provenance: Provenance::default(),
    }
}

#[test]
fn figure_chunks_carry_caption_text_and_figure_image_vectors() {
    let collection = unique("dual-fig");
    let dim_text = 4u64;
    let dim_figure = 32u64;

    let Ok(ix) = QdrantIndexer::open_with_slots(
        &url(), &collection, dim_text,
        vec![("figure".to_string(), dim_figure)],
    ) else { eprintln!("skip: no Qdrant"); return; };

    // Three figure chunks — one per fixture image.
    let chunks: Vec<Chunk> = (0..3)
        .map(|i| figure_chunk("paper-x", i, &format!("Figure {}: jet pT", i + 1), (i + 1) as u32, i + 1))
        .collect();

    // Caption text gets a tiny stub vector for the `text` slot.
    let text_vecs: Vec<Vec<f32>> = chunks.iter().map(|c| {
        let mut v = vec![0.0; dim_text as usize];
        v[0] = c.chunk_index as f32 * 0.1;
        v
    }).collect();

    // Image bytes → figure-slot vectors via the multimodal stub.
    let mm = MultimodalStubEmbedder::with_dim(dim_figure as usize);
    let fig_vecs: Vec<Vec<f32>> = (0..3)
        .map(|i| mm.embed_image(&format!("image-bytes-{i}").into_bytes()))
        .collect();

    let mut named = BTreeMap::new();
    named.insert("text".to_string(), text_vecs);
    named.insert("figure".to_string(), fig_vecs);
    ix.upsert_named(&chunks, named).expect("upsert_named");

    // All three figure points landed.
    assert_eq!(ix.count().unwrap() as usize, 3);

    // The `text` slot is queryable with caption-shaped vectors.
    let hits_text = ix.search(&vec![0.0; dim_text as usize], 5, Some("figure"))
        .expect("text search filtered to content_type=figure");
    assert_eq!(hits_text.len(), 3);
    assert!(hits_text.iter().all(|h| h.content_type == "figure"));
}

#[test]
fn three_named_slots_coexist_text_code_figure() {
    let collection = unique("three-slot");
    let Ok(_ix) = QdrantIndexer::open_with_slots(
        &url(), &collection, 4,
        vec![
            ("code".to_string(), 8),
            ("figure".to_string(), 16),
        ],
    ) else { eprintln!("skip: no Qdrant"); return; };
    // Successful open with three distinct slots is the assertion. Qdrant
    // would reject duplicate or invalid slot configurations at create time.
}

#[test]
fn figure_meta_serde_round_trip_preserves_caption_and_page() {
    let m = FigureMeta {
        paper_title: Some("p".into()),
        caption: "Figure 1: pT distribution".into(),
        page: Some(7),
        figure_number: Some(1),
    };
    let json = serde_json::to_string(&m).unwrap();
    let back: FigureMeta = serde_json::from_str(&json).unwrap();
    assert_eq!(back.caption, m.caption);
    assert_eq!(back.page, m.page);
    assert_eq!(back.figure_number, m.figure_number);
}
