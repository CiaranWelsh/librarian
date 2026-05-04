//! Integration: real Qdrant + local-dir NAS. Exercises snapshot, restore, list,
//! and prune. Gated on Qdrant; skips silently otherwise.

use adapter_indexer_qdrant::QdrantIndexer;
use adapter_snapshotter_qdrant_nas::QdrantNasSnapshotter;
use librarian_domain::{
    AdapterIdentity, BookMeta, Chunk, ChunkId, ChunkPayload, Indexer, Provenance, SnapshotId,
    Snapshotter, SourceId,
};
use tempfile::TempDir;

fn url() -> String {
    std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6533".into())
}

fn unique_collection(label: &str) -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    format!("librarian-snap-{label}-{nanos}")
}

fn chunk(sid: &str, idx: u32) -> Chunk {
    Chunk {
        chunk_id: ChunkId(format!("{sid}#{idx}")),
        source_id: SourceId(sid.into()),
        chunk_index: idx,
        text: format!("text-{idx}"),
        payload: ChunkPayload::Book(BookMeta {
            title: "t".into(), author: None, chapter: None, section: None, page: None,
        }),
        provenance: Provenance::default(),
    }
}

fn populate(collection: &str, dim: u64) -> Option<QdrantIndexer> {
    let ix = QdrantIndexer::open(&url(), collection, dim).ok()?;
    ix.upsert(
        &[chunk("d", 0), chunk("d", 1), chunk("d", 2)],
        &[vec![0.1; dim as usize], vec![0.2; dim as usize], vec![0.3; dim as usize]],
    ).ok()?;
    Some(ix)
}

#[test]
fn snapshot_then_list_returns_one_id_on_nas() {
    let collection = unique_collection("snap-list");
    let Some(ix) = populate(&collection, 4) else { eprintln!("skip"); return; };
    let nas = TempDir::new().unwrap();

    let s = QdrantNasSnapshotter::new(url(), &collection, nas.path()).unwrap();
    let id = s.snapshot().expect("snapshot");

    let listed = s.list().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0], id);

    // The file landed on the NAS.
    assert!(nas.path().join(&id.0).exists(), "{}", id.0);
    drop(ix);
}

#[test]
fn restore_into_empty_collection_brings_back_points() {
    let collection = unique_collection("snap-restore");
    let Some(ix) = populate(&collection, 4) else { eprintln!("skip"); return; };
    assert_eq!(ix.count().unwrap(), 3);

    let nas = TempDir::new().unwrap();
    let s = QdrantNasSnapshotter::new(url(), &collection, nas.path()).unwrap();
    let id = s.snapshot().expect("snapshot");

    // Wipe the collection.
    ix.delete_by_source_id(&SourceId("d".into())).unwrap();
    assert_eq!(ix.count().unwrap(), 0);

    s.restore(&id).expect("restore");

    // A fresh handle is needed to see restored state because the collection
    // got recreated from the snapshot.
    let ix2 = QdrantIndexer::open(&url(), &collection, 4).unwrap();
    assert_eq!(ix2.count().unwrap(), 3, "restored points present");
}

#[test]
fn prune_keeps_only_the_newest_n_snapshots() {
    let collection = unique_collection("snap-prune");
    let Some(_ix) = populate(&collection, 4) else { eprintln!("skip"); return; };
    let nas = TempDir::new().unwrap();
    let s = QdrantNasSnapshotter::new(url(), &collection, nas.path()).unwrap();

    for _ in 0..5 {
        s.snapshot().expect("snapshot");
        // Qdrant timestamps snapshot file names per second; nudge to keep mtimes distinct.
        std::thread::sleep(std::time::Duration::from_millis(1100));
    }
    assert_eq!(s.list().unwrap().len(), 5);

    s.prune(3).expect("prune");
    assert_eq!(s.list().unwrap().len(), 3);
}

#[test]
fn restore_unknown_id_is_a_clear_not_found_error() {
    let collection = unique_collection("snap-missing");
    let nas = TempDir::new().unwrap();
    let s = QdrantNasSnapshotter::new(url(), &collection, nas.path()).unwrap();
    let r = s.restore(&SnapshotId("does_not_exist.snapshot".into()));
    match r {
        Err(adapter_snapshotter_qdrant_nas::SnapshotError::NotFound(_)) => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn adapter_identity_changes_with_collection() {
    let dir = TempDir::new().unwrap();
    let s1 = QdrantNasSnapshotter::new(url(), "col-a", dir.path()).unwrap();
    let s2 = QdrantNasSnapshotter::new(url(), "col-b", dir.path()).unwrap();
    assert_ne!(s1.config_hash().0, s2.config_hash().0);
}
