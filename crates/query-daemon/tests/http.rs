//! In-process axum integration tests using tower `oneshot` — no network.

use std::sync::Arc;

use adapter_embedder_stub::StubEmbedder;
use adapter_indexer_mem::MemSearcher;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use librarian_domain::{
    AdapterIdentity, BookMeta, Chunk, ChunkId, ChunkPayload, ConfigHash, Embedder, EmbedderError,
    Provenance, SourceId, StageVersion, Vector,
};
use query_core::QueryService;
use query_daemon::access_log::AccessLog;
use query_daemon::auth::AuthState;
use query_daemon::{router, AppState};
use tower::ServiceExt; // oneshot

fn auth() -> Arc<AuthState> {
    Arc::new(AuthState::single_key("test", "t"))
}

fn book_chunk(source: &str, idx: u32, text: &str) -> Chunk {
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

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn test_app() -> axum::Router {
    let stub = StubEmbedder::new();
    let mem = MemSearcher::new();
    let apple = stub.embed(&["apple"]).unwrap().remove(0);
    mem.add("c", book_chunk("apple", 0, "apple body"), apple);
    let svc = QueryService::new(Arc::new(stub), mem, 4);
    router(AppState { svc: Arc::new(svc) }, Some(auth()), None)
}

// ---- plan tests ----------------------------------------------------------

#[tokio::test]
async fn healthz_ok() {
    let resp = test_app()
        .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn v1_requires_bearer_key() {
    // fail-closed (issue 032): no Authorization header -> 401 before the handler runs.
    let req = Request::post("/v1/search")
        .body(Body::from(r#"{"collection":"c","query":"apple"}"#))
        .unwrap();
    let resp = test_app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn search_returns_hits() {
    let req = Request::post("/v1/search")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test")
        .body(Body::from(
            r#"{"collection":"c","query":"apple","limit":5}"#,
        ))
        .unwrap();
    let resp = test_app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["hits"][0]["source_id"], "apple");
}

#[tokio::test]
async fn search_response_includes_confidence() {
    // Tier 0 (issue 028): every search carries a retrieval-confidence object.
    let req = Request::post("/v1/search")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test")
        .body(Body::from(
            r#"{"collection":"c","query":"apple","limit":5}"#,
        ))
        .unwrap();
    let resp = test_app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    let c = &v["confidence"];
    assert!(
        c.is_object(),
        "search response must carry a confidence object"
    );
    let label = c["label"].as_str().unwrap();
    assert!(
        matches!(label, "strong" | "weak" | "likely_no_answer"),
        "unexpected label {label}"
    );
    let value = c["value"].as_f64().unwrap();
    assert!((0.0..=1.0).contains(&value), "value {value} out of [0,1]");
    // Raw signals are exposed for transparency / calibration.
    for k in ["top_score", "margin", "score_spread", "fragment_rate"] {
        assert!(c[k].is_number(), "missing confidence signal {k}");
    }
}

#[tokio::test]
async fn confidence_is_no_answer_on_empty_result() {
    let stub = StubEmbedder::new();
    let mem = MemSearcher::new();
    let v = stub.embed(&["apple"]).unwrap().remove(0);
    mem.add("col", book_chunk("apple", 0, "apple body"), v);
    let svc = QueryService::new(Arc::new(stub), mem, 4);
    let app = router(AppState { svc: Arc::new(svc) }, Some(auth()), None);

    // content_type=paper matches nothing → zero hits → LikelyNoAnswer.
    let req = Request::post("/v1/search")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test")
        .body(Body::from(
            r#"{"collection":"col","query":"apple","content_type":"paper"}"#,
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let v = body_json(resp).await;
    assert_eq!(v["hits"].as_array().unwrap().len(), 0);
    assert_eq!(v["confidence"]["label"], "likely_no_answer");
    assert_eq!(v["confidence"]["value"], 0.0);
}

#[tokio::test]
async fn empty_query_is_400() {
    let req = Request::post("/v1/search")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test")
        .body(Body::from(r#"{"collection":"c","query":"  "}"#))
        .unwrap();
    let resp = test_app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let v = body_json(resp).await;
    assert_eq!(v["error"]["code"], "bad_request");
}

#[tokio::test]
async fn unknown_collection_is_404() {
    let req = Request::post("/v1/search")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test")
        .body(Body::from(r#"{"collection":"missing","query":"apple"}"#))
        .unwrap();
    let resp = test_app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn documents_lists_sources() {
    let resp = test_app()
        .oneshot(
            Request::get("/v1/documents?collection=c")
                .header("authorization", "Bearer test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["documents"][0], "apple");
}

// ---- coverage additions --------------------------------------------------

#[tokio::test]
async fn extract_returns_chunks() {
    let stub = StubEmbedder::new();
    let mem = MemSearcher::new();
    for i in 0..3u32 {
        mem.add(
            "demo",
            book_chunk("paper", i, &format!("chunk {i}")),
            vec![1.0_f32; 32],
        );
    }
    let svc = QueryService::new(Arc::new(stub), mem, 4);
    let app = router(AppState { svc: Arc::new(svc) }, Some(auth()), None);

    let req = Request::post("/v1/extract")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test")
        .body(Body::from(r#"{"collection":"demo","source_id":"paper"}"#))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["source_id"], "paper");
    let chunks = v["chunks"].as_array().unwrap();
    assert_eq!(chunks.len(), 3);
    // ordered by chunk_index
    assert_eq!(chunks[0]["chunk_index"], 0);
    assert_eq!(chunks[1]["chunk_index"], 1);
    assert_eq!(chunks[2]["chunk_index"], 2);
    assert!(!chunks[0]["text"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn limit_defaults_to_five() {
    let stub = StubEmbedder::new();
    let mem = MemSearcher::new();
    for i in 0..8u32 {
        let v = stub.embed(&[&format!("doc{i}")]).unwrap().remove(0);
        mem.add("col", book_chunk(&format!("doc{i}"), 0, "body"), v);
    }
    let svc = QueryService::new(Arc::new(stub), mem, 4);
    let app = router(AppState { svc: Arc::new(svc) }, Some(auth()), None);

    let req = Request::post("/v1/search")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test")
        .body(Body::from(r#"{"collection":"col","query":"doc0"}"#))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["hits"].as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn empty_result_is_200_not_404() {
    let stub = StubEmbedder::new();
    let mem = MemSearcher::new();
    let v = stub.embed(&["apple"]).unwrap().remove(0);
    // seed a book chunk; then search with content_type=paper (no papers exist)
    mem.add("col", book_chunk("apple", 0, "apple body"), v);
    let svc = QueryService::new(Arc::new(stub), mem, 4);
    let app = router(AppState { svc: Arc::new(svc) }, Some(auth()), None);

    let req = Request::post("/v1/search")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test")
        .body(Body::from(
            r#"{"collection":"col","query":"apple","content_type":"paper"}"#,
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["hits"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn not_found_envelope_has_code_and_message() {
    let req = Request::post("/v1/search")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test")
        .body(Body::from(r#"{"collection":"nope","query":"x"}"#))
        .unwrap();
    let resp = test_app().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let v = body_json(resp).await;
    assert_eq!(v["error"]["code"], "not_found");
    assert!(!v["error"]["message"].as_str().unwrap_or("").is_empty());
}

struct RecoverableEmbedder;

impl AdapterIdentity for RecoverableEmbedder {
    fn name(&self) -> &str {
        "recoverable"
    }
    fn version(&self) -> StageVersion {
        StageVersion("1".into())
    }
    fn config_hash(&self) -> ConfigHash {
        ConfigHash("0".into())
    }
}

impl Embedder for RecoverableEmbedder {
    fn embed(&self, _texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        Err(EmbedderError::Recoverable("rate limited".into()))
    }
    fn dimension(&self) -> usize {
        4
    }
}

#[tokio::test]
async fn recoverable_embed_is_503_with_retry_after() {
    let mem = MemSearcher::new();
    mem.add("col", book_chunk("doc", 0, "body"), vec![1.0_f32; 4]);
    let svc = QueryService::new(Arc::new(RecoverableEmbedder), mem, 4);
    let app = router(AppState { svc: Arc::new(svc) }, Some(auth()), None);

    let req = Request::post("/v1/search")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test")
        .body(Body::from(r#"{"collection":"col","query":"x"}"#))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        resp.headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok()),
        Some("2")
    );
}

// ---- access log (traffic monitoring, no telemetry stack) ------------------

fn seeded_app(log: Arc<AccessLog>) -> axum::Router {
    let stub = StubEmbedder::new();
    let mem = MemSearcher::new();
    let apple = stub.embed(&["apple"]).unwrap().remove(0);
    mem.add("c", book_chunk("apple", 0, "apple body"), apple);
    let svc = QueryService::new(Arc::new(stub), mem, 4);
    router(AppState { svc: Arc::new(svc) }, Some(auth()), Some(log))
}

fn read_lines(path: &std::path::Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[tokio::test]
async fn access_log_writes_one_line_per_request() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("access.jsonl");
    let app = seeded_app(Arc::new(AccessLog::new(path.clone(), true)));

    let ok = Request::post("/v1/search")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test")
        .body(Body::from(r#"{"collection":"c","query":"apple"}"#))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(ok).await.unwrap().status(),
        StatusCode::OK
    );
    // a rejected request is traffic too — logged, but with no user
    let bad = Request::post("/v1/search")
        .body(Body::from(r#"{"collection":"c","query":"apple"}"#))
        .unwrap();
    assert_eq!(
        app.oneshot(bad).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );

    let lines = read_lines(&path);
    assert_eq!(lines.len(), 2, "one line per /v1 request: {lines:?}");
    assert_eq!(lines[0]["route"], "/v1/search");
    assert_eq!(lines[0]["status"], 200);
    assert_eq!(lines[0]["user"], "t");
    assert_eq!(lines[0]["collection"], "c");
    assert_eq!(lines[0]["query"], "apple");
    assert!(lines[0]["confidence"].is_string(), "search logs its label");
    assert!(lines[0]["ts"].is_u64() && lines[0]["ms"].is_u64());
    assert_eq!(lines[1]["status"], 401);
    assert!(lines[1].get("user").is_none(), "reject has no identity");
}

#[tokio::test]
async fn access_log_omits_query_text_when_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("access.jsonl");
    let app = seeded_app(Arc::new(AccessLog::new(path.clone(), false)));

    let req = Request::post("/v1/search")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test")
        .body(Body::from(r#"{"collection":"c","query":"apple"}"#))
        .unwrap();
    assert_eq!(app.oneshot(req).await.unwrap().status(), StatusCode::OK);

    let lines = read_lines(&path);
    assert!(lines[0].get("query").is_none(), "query text suppressed");
    assert_eq!(lines[0]["collection"], "c", "shape still logged");
}
