//! QA-Q1 concurrency gate: 12 concurrent searches all succeed and the embed
//! semaphore caps simultaneous embeds at exactly the configured limit (3).
//!
//! The Barrier forces exactly `limit` embeds to overlap deterministically —
//! no sleeps, no heuristics. The semaphore proves it never exceeds `limit`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

use adapter_indexer_mem::MemSearcher;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use librarian_domain::{
    AdapterIdentity, BookMeta, Chunk, ChunkId, ChunkPayload, ConfigHash, Embedder, EmbedderError,
    Provenance, SourceId, StageVersion, Vector,
};
use query_core::QueryService;
use query_daemon::auth::AuthState;
use query_daemon::{router, AppState};
use tower::ServiceExt;

const LIMIT: usize = 3;
const REQUESTS: usize = 12; // must be a multiple of LIMIT

struct CountingEmbedder {
    inflight: AtomicUsize,
    peak: AtomicUsize,
    // Barrier of size LIMIT forces exactly LIMIT threads to overlap per wave.
    barrier: Arc<Barrier>,
}

impl AdapterIdentity for CountingEmbedder {
    fn name(&self) -> &str {
        "counting"
    }
    fn version(&self) -> StageVersion {
        StageVersion("1".into())
    }
    fn config_hash(&self) -> ConfigHash {
        ConfigHash("0".into())
    }
}

impl Embedder for CountingEmbedder {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        let now = self.inflight.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(now, Ordering::SeqCst);
        // All LIMIT concurrent embeds must arrive before any can leave.
        // Because REQUESTS % LIMIT == 0 each full wave hits this; no partial
        // wave hangs on the barrier.
        self.barrier.wait();
        self.inflight.fetch_sub(1, Ordering::SeqCst);
        Ok(texts.iter().map(|_| vec![1.0_f32; 4]).collect())
    }

    fn dimension(&self) -> usize {
        4
    }
}

fn seed_chunk(source: &str) -> Chunk {
    Chunk {
        chunk_id: ChunkId(format!("{source}#0")),
        source_id: SourceId(source.into()),
        chunk_index: 0,
        text: "body".into(),
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_searches_succeed_and_embeds_are_bounded() {
    let emb = Arc::new(CountingEmbedder {
        inflight: AtomicUsize::new(0),
        peak: AtomicUsize::new(0),
        barrier: Arc::new(Barrier::new(LIMIT)),
    });
    let mem = MemSearcher::new();
    mem.add("c", seed_chunk("doc"), vec![1.0_f32; 4]);
    let svc = Arc::new(QueryService::new(Arc::clone(&emb), mem, LIMIT));
    let app = router(
        AppState { svc },
        Some(Arc::new(AuthState::single_key("test", "t"))),
        None,
    );

    let mut handles = Vec::new();
    for _ in 0..REQUESTS {
        let app = app.clone();
        handles.push(tokio::spawn(async move {
            let req = Request::post("/v1/search")
                .header("content-type", "application/json")
                .header("authorization", "Bearer test")
                .body(Body::from(r#"{"collection":"c","query":"x"}"#))
                .unwrap();
            app.oneshot(req).await.unwrap().status()
        }));
    }
    for h in handles {
        assert_eq!(h.await.unwrap(), StatusCode::OK);
    }

    let peak = emb.peak.load(Ordering::SeqCst);
    assert_eq!(
        peak, LIMIT,
        "expected peak == {LIMIT} (deterministic barrier), got {peak}"
    );
}
