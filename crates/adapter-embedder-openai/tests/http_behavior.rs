//! Integration: real reqwest client against a local mockito server. Exercises
//! batching, error classification, and request shape.

use adapter_embedder_openai::{OpenAiConfig, OpenAiEmbedder};
use librarian_domain::{Embedder, EmbedderError};

fn cfg(endpoint: String, batch: usize) -> OpenAiConfig {
    OpenAiConfig {
        model: "text-embedding-3-small".into(),
        dimensions: 4,
        endpoint: Some(endpoint),
        batch_size: Some(batch),
        timeout: Some(std::time::Duration::from_secs(2)),
    }
}

#[test]
fn returns_one_vector_per_input_on_200() {
    let mut server = mockito::Server::new();
    let endpoint = format!("{}/v1/embeddings", server.url());
    let body = r#"{"data":[
        {"embedding":[0.1,0.2,0.3,0.4]},
        {"embedding":[0.5,0.6,0.7,0.8]}
    ]}"#;
    let m = server.mock("POST", "/v1/embeddings")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .expect(1)
        .create();

    let e = OpenAiEmbedder::new("k", cfg(endpoint, 96)).unwrap();
    let v = e.embed(&["alpha", "beta"]).unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v[0], vec![0.1, 0.2, 0.3, 0.4]);
    m.assert();
}

#[test]
fn batches_split_at_configured_size_boundary() {
    let mut server = mockito::Server::new();
    let endpoint = format!("{}/v1/embeddings", server.url());
    // Returns one element per requested input — mockito doesn't see the body,
    // but our embedder copies inputs into the request and we only assert on
    // call count + total returned vectors.
    let one = r#"{"data":[{"embedding":[0.0,0.0,0.0,0.0]}]}"#;
    let two = r#"{"data":[{"embedding":[0.0,0.0,0.0,0.0]},{"embedding":[0.0,0.0,0.0,0.0]}]}"#;

    // 5 inputs, batch=2 → 2 + 2 + 1 = 3 calls. Two return 2-element bodies, one returns 1.
    let m_two = server.mock("POST", "/v1/embeddings")
        .with_status(200).with_body(two).expect(2).create();
    let m_one = server.mock("POST", "/v1/embeddings")
        .with_status(200).with_body(one).expect(1).create();

    let e = OpenAiEmbedder::new("k", cfg(endpoint, 2)).unwrap();
    let v = e.embed(&["a", "b", "c", "d", "e"]).unwrap();
    assert_eq!(v.len(), 5, "5 vectors returned across 3 batched calls");
    m_two.assert();
    m_one.assert();
}

#[test]
fn http_500_classifies_as_recoverable() {
    let mut server = mockito::Server::new();
    let endpoint = format!("{}/v1/embeddings", server.url());
    server.mock("POST", "/v1/embeddings").with_status(500).with_body("server boom").create();

    let e = OpenAiEmbedder::new("k", cfg(endpoint, 96)).unwrap();
    match e.embed(&["x"]).unwrap_err() {
        EmbedderError::Recoverable(msg) => assert!(msg.contains("500")),
        EmbedderError::Terminal(_) => panic!("5xx must be Recoverable"),
    }
}

#[test]
fn http_429_classifies_as_recoverable() {
    let mut server = mockito::Server::new();
    let endpoint = format!("{}/v1/embeddings", server.url());
    server.mock("POST", "/v1/embeddings").with_status(429).with_body("rate limit").create();

    let e = OpenAiEmbedder::new("k", cfg(endpoint, 96)).unwrap();
    match e.embed(&["x"]).unwrap_err() {
        EmbedderError::Recoverable(_) => {}
        EmbedderError::Terminal(_) => panic!("429 must be Recoverable"),
    }
}

#[test]
fn http_401_classifies_as_terminal() {
    let mut server = mockito::Server::new();
    let endpoint = format!("{}/v1/embeddings", server.url());
    server.mock("POST", "/v1/embeddings").with_status(401).with_body(r#"{"error":"bad key"}"#).create();

    let e = OpenAiEmbedder::new("k", cfg(endpoint, 96)).unwrap();
    match e.embed(&["x"]).unwrap_err() {
        EmbedderError::Terminal(msg) => assert!(msg.contains("401")),
        EmbedderError::Recoverable(_) => panic!("401 must be Terminal"),
    }
}

#[test]
fn request_carries_bearer_auth_and_model_in_body() {
    let mut server = mockito::Server::new();
    let endpoint = format!("{}/v1/embeddings", server.url());
    let m = server.mock("POST", "/v1/embeddings")
        .match_header("authorization", "Bearer my-secret")
        .match_body(mockito::Matcher::Regex(r#""model":"text-embedding-3-small""#.into()))
        .with_status(200)
        .with_body(r#"{"data":[{"embedding":[0.0,0.0,0.0,0.0]}]}"#)
        .expect(1)
        .create();

    let e = OpenAiEmbedder::new("my-secret", cfg(endpoint, 96)).unwrap();
    e.embed(&["hi"]).expect("ok");
    m.assert();
}

#[test]
fn response_with_wrong_count_is_terminal() {
    let mut server = mockito::Server::new();
    let endpoint = format!("{}/v1/embeddings", server.url());
    server.mock("POST", "/v1/embeddings")
        .with_status(200)
        .with_body(r#"{"data":[{"embedding":[0.0,0.0,0.0,0.0]}]}"#)
        .create();

    // Asked for 2 embeddings but mock returns 1.
    let e = OpenAiEmbedder::new("k", cfg(endpoint, 96)).unwrap();
    match e.embed(&["a", "b"]).unwrap_err() {
        EmbedderError::Terminal(msg) => assert!(msg.contains("expected 2")),
        _ => panic!("count mismatch must be Terminal"),
    }
}
