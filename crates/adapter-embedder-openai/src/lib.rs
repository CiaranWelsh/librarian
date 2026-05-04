//! OpenAI embeddings adapter. Sync, blocking `reqwest`.
//!
//! Error classification (slice 010 AC):
//! - HTTP 5xx, request timeout, transport error, 408, 429 → `Recoverable`.
//! - HTTP 4xx (excluding 408/429), auth/quota → `Terminal`.
//! - The runner / `FallbackEmbedder` decides whether to retry.

use librarian_domain::{
    AdapterIdentity, ConfigHash, Embedder, EmbedderError, StageVersion, Vector,
};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_ENDPOINT: &str = "https://api.openai.com/v1/embeddings";
const DEFAULT_BATCH: usize = 96;
const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub struct OpenAiEmbedder {
    client: Client,
    endpoint: String,
    api_key: String,
    model: String,
    dimensions: usize,
    batch_size: usize,
}

#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    pub model: String,
    pub dimensions: usize,
    pub endpoint: Option<String>,
    pub batch_size: Option<usize>,
    pub timeout: Option<Duration>,
}

impl OpenAiEmbedder {
    /// Construct from explicit config + API key. Use [`from_env`] in production.
    pub fn new(api_key: impl Into<String>, cfg: OpenAiConfig) -> Result<Self, OpenAiBuildError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(OpenAiBuildError::MissingApiKey);
        }
        let timeout = cfg.timeout.unwrap_or(Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(OpenAiBuildError::Http)?;
        Ok(Self {
            client,
            endpoint: cfg.endpoint.unwrap_or_else(|| DEFAULT_ENDPOINT.to_string()),
            api_key,
            model: cfg.model,
            dimensions: cfg.dimensions,
            batch_size: cfg.batch_size.unwrap_or(DEFAULT_BATCH),
        })
    }

    /// Read API key from `OPENAI_API_KEY`.
    pub fn from_env(cfg: OpenAiConfig) -> Result<Self, OpenAiBuildError> {
        let key = std::env::var("OPENAI_API_KEY").map_err(|_| OpenAiBuildError::MissingApiKey)?;
        Self::new(key, cfg)
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        let req = EmbedRequest {
            input: texts,
            model: &self.model,
            dimensions: self.dimensions,
        };
        let resp = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&req)
            .send();

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                if e.is_timeout() || e.is_connect() {
                    return Err(EmbedderError::Recoverable(format!("transport: {e}")));
                }
                return Err(EmbedderError::Terminal(format!("transport: {e}")));
            }
        };

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            return Err(classify(status.as_u16(), &body));
        }

        let parsed: EmbedResponse = resp
            .json()
            .map_err(|e| EmbedderError::Terminal(format!("decode: {e}")))?;
        if parsed.data.len() != texts.len() {
            return Err(EmbedderError::Terminal(format!(
                "response has {} embeddings, expected {}",
                parsed.data.len(), texts.len()
            )));
        }
        Ok(parsed.data.into_iter().map(|d| d.embedding).collect())
    }
}

fn classify(status: u16, body: &str) -> EmbedderError {
    match status {
        408 | 429 | 500..=599 => {
            EmbedderError::Recoverable(format!("http {status}: {}", truncate(body)))
        }
        _ => EmbedderError::Terminal(format!("http {status}: {}", truncate(body))),
    }
}

fn truncate(s: &str) -> String {
    if s.len() <= 200 { s.to_string() } else { format!("{}…", &s[..200]) }
}

#[derive(Debug, thiserror::Error)]
pub enum OpenAiBuildError {
    #[error("OPENAI_API_KEY missing")]
    MissingApiKey,
    #[error("http: {0}")]
    Http(#[source] reqwest::Error),
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    input: &'a [&'a str],
    model: &'a str,
    dimensions: usize,
}

#[derive(Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedDatum>,
}

#[derive(Deserialize)]
struct EmbedDatum {
    embedding: Vector,
}

impl AdapterIdentity for OpenAiEmbedder {
    fn name(&self) -> &str { "embedder-openai" }
    fn version(&self) -> StageVersion {
        // The model name is the user-meaningful version axis here.
        StageVersion(self.model.clone())
    }
    fn config_hash(&self) -> ConfigHash {
        ConfigHash(format!("model={};dim={}", self.model, self.dimensions))
    }
}

impl Embedder for OpenAiEmbedder {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        if texts.is_empty() {
            return Err(EmbedderError::Terminal("empty batch".into()));
        }
        let mut out = Vec::with_capacity(texts.len());
        for chunk in texts.chunks(self.batch_size) {
            let v = self.embed_batch(chunk)?;
            out.extend(v);
        }
        Ok(out)
    }

    fn dimension(&self) -> usize { self.dimensions }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_api_key_is_a_terminal_build_error() {
        let r = OpenAiEmbedder::new("", OpenAiConfig {
            model: "text-embedding-3-small".into(), dimensions: 4,
            endpoint: None, batch_size: None, timeout: None,
        });
        assert!(matches!(r, Err(OpenAiBuildError::MissingApiKey)));
    }

    #[test]
    fn classify_5xx_is_recoverable() {
        assert!(matches!(classify(503, ""), EmbedderError::Recoverable(_)));
        assert!(matches!(classify(500, ""), EmbedderError::Recoverable(_)));
    }

    #[test]
    fn classify_429_is_recoverable() {
        assert!(matches!(classify(429, ""), EmbedderError::Recoverable(_)));
    }

    #[test]
    fn classify_4xx_other_is_terminal() {
        assert!(matches!(classify(401, ""), EmbedderError::Terminal(_)));
        assert!(matches!(classify(400, ""), EmbedderError::Terminal(_)));
        assert!(matches!(classify(404, ""), EmbedderError::Terminal(_)));
    }

    #[test]
    fn empty_batch_is_terminal_via_embed() {
        let e = OpenAiEmbedder::new("k", OpenAiConfig {
            model: "m".into(), dimensions: 4, endpoint: Some("http://localhost".into()),
            batch_size: None, timeout: None,
        }).unwrap();
        match e.embed(&[]).unwrap_err() {
            EmbedderError::Terminal(_) => {}
            _ => panic!("empty batch should be Terminal"),
        }
    }

    #[test]
    fn truncate_caps_long_bodies() {
        let long = "a".repeat(500);
        assert!(truncate(&long).len() <= 250);
        assert_eq!(truncate("short"), "short");
    }
}
