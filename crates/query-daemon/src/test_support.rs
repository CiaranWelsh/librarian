//! Shared test harness: a stub+mem daemon on an ephemeral port. Behind the
//! `test-support` feature so cli/server can integration-test the HTTP boundary
//! with no OpenAI or qdrant dependency.

use std::sync::Arc;

use adapter_embedder_stub::StubEmbedder;
use adapter_indexer_mem::MemSearcher;
use librarian_domain::{BookMeta, Chunk, ChunkId, ChunkPayload, Embedder, Provenance, SourceId};
use query_core::QueryService;

use crate::{router, AppState};

fn demo_chunk(source: &str, idx: u32, text: &str) -> Chunk {
    Chunk {
        chunk_id: ChunkId(format!("{source}#{idx}")),
        source_id: SourceId(source.into()),
        chunk_index: idx,
        text: text.into(),
        payload: ChunkPayload::Book(BookMeta {
            title: source.into(),
            author: None,
            chapter: None,
            section: None,
            page: None,
        }),
        provenance: Provenance::default(),
    }
}

/// Serve a stub+mem daemon seeded with a `demo` collection (`apple`, `zebra`)
/// on `127.0.0.1:0`. Returns `(base_url, handle)`; keep `handle` alive for the
/// test's duration (dropping it aborts the server task).
///
/// The listener `bind().await`s BEFORE the URL is returned, so callers never
/// race the server — the port is reserved and axum is serving before this
/// function resolves (DET-1).
pub async fn spawn() -> (String, tokio::task::JoinHandle<()>) {
    let stub = StubEmbedder::new();
    let mem = MemSearcher::new();
    for (s, t) in [("apple", "apple body"), ("zebra", "zebra body")] {
        let v = stub.embed(&[s]).unwrap().remove(0);
        mem.add("demo", demo_chunk(s, 0, t), v);
    }
    let svc = Arc::new(QueryService::new(Arc::new(stub), mem, 4));
    // No auth, no access log: the harness tests the query surface, not the gate.
    let app = router(AppState { svc }, None, None);
    // bind() completes before we return the URL — callers never race the server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), handle)
}
