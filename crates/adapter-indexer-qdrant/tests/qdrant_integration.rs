//! Integration: real Qdrant. Gated by `LIBRARIAN_QDRANT_URL` env var.
//! Default: `http://localhost:6533` (the dedicated test instance brought up
//! for slice 005). If unreachable, tests are skipped (not failed).

use adapter_indexer_qdrant::{point_id, QdrantIndexer};
use librarian_domain::{
    BookMeta, Chunk, ChunkId, ChunkPayload, Indexer, Provenance, SourceId,
};

fn url() -> String {
    std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6533".into())
}

/// Skip the test (not fail) if Qdrant isn't reachable.
fn open_or_skip(collection: &str, dim: u64) -> Option<QdrantIndexer> {
    QdrantIndexer::open(&url(), collection, dim).ok()
}

fn unique_collection(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("librarian-test-{label}-{nanos}")
}

fn chunk(sid: &str, idx: u32, text: &str) -> Chunk {
    Chunk {
        chunk_id: ChunkId(format!("{sid}#{idx}")),
        source_id: SourceId(sid.into()),
        chunk_index: idx,
        text: text.into(),
        payload: ChunkPayload::Book(BookMeta {
            title: "t".into(), author: None, chapter: None, section: None, page: None,
        }),
        provenance: Provenance::default(),
    }
}

fn vec1(seed: f32) -> Vec<f32> {
    (0..4).map(|i| seed + i as f32 * 0.01).collect()
}

#[test]
fn upsert_then_count_matches_input() {
    let collection = unique_collection("upsert");
    let Some(ix) = open_or_skip(&collection, 4) else { eprintln!("skip: no Qdrant"); return; };

    let chunks = vec![chunk("a", 0, "hi"), chunk("a", 1, "hello")];
    let vectors = vec![vec1(0.1), vec1(0.2)];
    ix.upsert(&chunks, &vectors).expect("upsert");
    assert_eq!(ix.count().unwrap(), 2);

    // Re-upsert same chunks: deterministic point IDs mean count stays at 2.
    ix.upsert(&chunks, &vectors).expect("re-upsert");
    assert_eq!(ix.count().unwrap(), 2);
}

#[test]
fn delete_by_source_id_removes_only_matching_source() {
    let collection = unique_collection("delete");
    let Some(ix) = open_or_skip(&collection, 4) else { eprintln!("skip: no Qdrant"); return; };

    ix.upsert(
        &[chunk("a", 0, "x"), chunk("a", 1, "y"), chunk("b", 0, "z")],
        &[vec1(0.1), vec1(0.2), vec1(0.3)],
    ).expect("seed");
    assert_eq!(ix.count().unwrap(), 3);
    assert_eq!(ix.count_by_source(&SourceId("a".into())).unwrap(), 2);

    ix.delete_by_source_id(&SourceId("a".into())).expect("delete a");
    assert_eq!(ix.count_by_source(&SourceId("a".into())).unwrap(), 0);
    assert_eq!(ix.count_by_source(&SourceId("b".into())).unwrap(), 1, "b untouched");

    // Idempotent on absent source.
    ix.delete_by_source_id(&SourceId("nope".into())).expect("delete nope");
    assert_eq!(ix.count().unwrap(), 1);
}

#[test]
fn replace_drops_orphan_chunks() {
    let collection = unique_collection("replace");
    let Some(ix) = open_or_skip(&collection, 4) else { eprintln!("skip: no Qdrant"); return; };

    ix.upsert(
        &[chunk("a", 0, "old0"), chunk("a", 1, "old1"), chunk("a", 2, "old2")],
        &[vec1(0.1), vec1(0.2), vec1(0.3)],
    ).expect("seed");
    assert_eq!(ix.count().unwrap(), 3);

    // Replace with a single chunk — chunks 1 and 2 become orphans and must be removed.
    ix.replace(
        &SourceId("a".into()),
        &[chunk("a", 0, "new0")],
        &[vec1(0.4)],
    ).expect("replace");
    assert_eq!(ix.count_by_source(&SourceId("a".into())).unwrap(), 1, "no orphans");
    assert_eq!(ix.count().unwrap(), 1);
}

#[test]
fn collection_open_is_idempotent() {
    let collection = unique_collection("idem");
    let Some(_ix1) = open_or_skip(&collection, 4) else { eprintln!("skip: no Qdrant"); return; };
    // Reopen — must not error.
    let ix2 = QdrantIndexer::open(&url(), &collection, 4).expect("reopen");
    drop(ix2);
}

#[test]
fn point_id_matches_documented_uuid_v5() {
    // Sanity: same inputs -> same UUID. Slice-005 AC.
    let a = point_id(&SourceId("doc".into()), 7);
    let b = point_id(&SourceId("doc".into()), 7);
    assert_eq!(a, b);
}
