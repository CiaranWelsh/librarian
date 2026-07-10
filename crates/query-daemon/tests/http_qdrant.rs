#![cfg(feature = "test-support")]
//! Daemon-over-qdrant integration test (B3 via daemon). Ignored by default — run with:
//!   LIBRARIAN_QDRANT_URL=http://localhost:6334 \
//!     cargo test -p query-daemon --features test-support --test http_qdrant -- --ignored

use std::sync::Arc;

use adapter_embedder_stub::StubEmbedder;
use adapter_indexer_qdrant::QdrantSearcher;
use query_core::QueryService;
use query_daemon::{router, AppState};

fn qurl() -> String {
    std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6334".into())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs live qdrant"]
async fn collections_endpoint_over_qdrant() {
    let searcher = QdrantSearcher::open(&qurl()).unwrap();
    let svc = Arc::new(QueryService::new(
        Arc::new(StubEmbedder::new()),
        searcher,
        4,
    ));
    let app = router(AppState { svc }, None, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let url = format!("http://{addr}/v1/collections");
    let v: serde_json::Value = tokio::task::spawn_blocking(move || {
        reqwest::blocking::Client::new()
            .get(&url)
            .send()
            .unwrap()
            .json()
            .unwrap()
    })
    .await
    .unwrap();
    let names: Vec<&str> = v["collections"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["name"].as_str())
        .collect();
    assert!(
        names.contains(&"particle-physics"),
        "got collections: {names:?}"
    );
}
