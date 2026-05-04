//! Integration: real reqwest against a mockito server, mirroring the OpenAI
//! adapter's HTTP behaviour tests.

use adapter_embedder_voyage::{VoyageConfig, VoyageEmbedder};
use librarian_domain::{Embedder, EmbedderError};

fn cfg(endpoint: String, batch: usize) -> VoyageConfig {
    VoyageConfig {
        model: "voyage-code-3".into(),
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
        .with_status(200).with_body(body).expect(1).create();

    let e = VoyageEmbedder::new("k", cfg(endpoint, 96)).unwrap();
    let v = e.embed(&["alpha", "beta"]).unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v[0], vec![0.1, 0.2, 0.3, 0.4]);
    m.assert();
}

#[test]
fn http_500_is_recoverable() {
    let mut server = mockito::Server::new();
    let endpoint = format!("{}/v1/embeddings", server.url());
    server.mock("POST", "/v1/embeddings").with_status(500).create();
    let e = VoyageEmbedder::new("k", cfg(endpoint, 96)).unwrap();
    assert!(matches!(e.embed(&["x"]).unwrap_err(), EmbedderError::Recoverable(_)));
}

#[test]
fn http_429_is_recoverable() {
    let mut server = mockito::Server::new();
    let endpoint = format!("{}/v1/embeddings", server.url());
    server.mock("POST", "/v1/embeddings").with_status(429).create();
    let e = VoyageEmbedder::new("k", cfg(endpoint, 96)).unwrap();
    assert!(matches!(e.embed(&["x"]).unwrap_err(), EmbedderError::Recoverable(_)));
}

#[test]
fn http_401_is_terminal() {
    let mut server = mockito::Server::new();
    let endpoint = format!("{}/v1/embeddings", server.url());
    server.mock("POST", "/v1/embeddings").with_status(401).create();
    let e = VoyageEmbedder::new("k", cfg(endpoint, 96)).unwrap();
    assert!(matches!(e.embed(&["x"]).unwrap_err(), EmbedderError::Terminal(_)));
}

#[test]
fn body_carries_voyage_code_3_model_and_input_type_document() {
    let mut server = mockito::Server::new();
    let endpoint = format!("{}/v1/embeddings", server.url());
    let m = server.mock("POST", "/v1/embeddings")
        .match_body(mockito::Matcher::Regex(r#""model":"voyage-code-3""#.into()))
        .match_body(mockito::Matcher::Regex(r#""input_type":"document""#.into()))
        .with_status(200)
        .with_body(r#"{"data":[{"embedding":[0.0,0.0,0.0,0.0]}]}"#)
        .expect(1)
        .create();
    let e = VoyageEmbedder::new("k", cfg(endpoint, 96)).unwrap();
    e.embed(&["fn main(){}"]).expect("ok");
    m.assert();
}

#[test]
fn batches_split_at_configured_size_boundary() {
    let mut server = mockito::Server::new();
    let endpoint = format!("{}/v1/embeddings", server.url());
    let two = r#"{"data":[{"embedding":[0.0,0.0,0.0,0.0]},{"embedding":[0.0,0.0,0.0,0.0]}]}"#;
    let one = r#"{"data":[{"embedding":[0.0,0.0,0.0,0.0]}]}"#;
    let m_two = server.mock("POST", "/v1/embeddings").with_status(200).with_body(two).expect(2).create();
    let m_one = server.mock("POST", "/v1/embeddings").with_status(200).with_body(one).expect(1).create();
    let e = VoyageEmbedder::new("k", cfg(endpoint, 2)).unwrap();
    let v = e.embed(&["a","b","c","d","e"]).unwrap();
    assert_eq!(v.len(), 5);
    m_two.assert();
    m_one.assert();
}
