//! Integration tests against a LIVE qdrant. Ignored by default — run with:
//!   LIBRARIAN_QDRANT_URL=http://localhost:6334 cargo test -p adapter-indexer-qdrant \
//!     --test searcher_integration -- --ignored
//! Requires a collection named `particle-physics` (clean librarian collection).

use adapter_indexer_qdrant::QdrantSearcher;
use librarian_domain::Searcher;

fn url() -> String {
    std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6334".into())
}

#[tokio::test]
#[ignore = "needs live qdrant"]
async fn list_collections_includes_known_collection() {
    let s = QdrantSearcher::open(&url()).unwrap();
    let cols = s.list_collections().await.unwrap();
    assert!(
        cols.iter()
            .any(|c| c.name == "particle-physics" && c.points > 0),
        "expected a populated particle-physics collection, got {cols:?}"
    );
}

#[tokio::test]
#[ignore = "needs live qdrant"]
async fn unknown_collection_is_not_found() {
    let s = QdrantSearcher::open(&url()).unwrap();
    let err = s
        .list_documents("definitely-not-a-collection")
        .await
        .unwrap_err();
    assert!(
        matches!(err, librarian_domain::SearchError::NotFound(_)),
        "expected NotFound, got {err:?}"
    );
}

#[tokio::test]
#[ignore = "needs live qdrant"]
async fn search_returns_well_formed_hits() {
    let s = QdrantSearcher::open(&url()).unwrap();
    // software / particle-physics use text-embedding-3-large (3072 dims).
    let q = vec![0.01_f32; 3072];
    let hits = s.search("particle-physics", &q, 5, None).await.unwrap();
    assert!(
        !hits.is_empty(),
        "expected hits from a populated collection"
    );
    for h in &hits {
        assert!(!h.source_id.0.is_empty());
        assert!(h.score <= 1.0001, "cosine score out of range: {}", h.score);
    }
}

#[tokio::test]
#[ignore = "needs live qdrant"]
async fn extract_of_first_hit_is_ordered() {
    let s = QdrantSearcher::open(&url()).unwrap();
    let q = vec![0.01_f32; 3072];
    let hits = s.search("particle-physics", &q, 1, None).await.unwrap();
    let sid = hits[0].source_id.clone();
    let chunks = s
        .get_extract("particle-physics", &sid, 0, u32::MAX)
        .await
        .unwrap();
    assert!(
        chunks
            .windows(2)
            .all(|w| w[0].chunk_index <= w[1].chunk_index),
        "extract chunks not ordered by chunk_index"
    );
}
