//! `VoyageEmbedder` — same shape as `OpenAiEmbedder`, different endpoint and
//! a Voyage-specific `input_type` field.

use librarian_domain::{
    AdapterIdentity, ConfigHash, Embedder, EmbedderError, StageVersion, Vector,
};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::classify::classify;
use crate::config::VoyageConfig;
use crate::error::VoyageBuildError;

const DEFAULT_ENDPOINT: &str = "https://api.voyageai.com/v1/embeddings";
const DEFAULT_BATCH: usize = 64;
const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub struct VoyageEmbedder {
    client: Client,
    endpoint: String,
    api_key: String,
    model: String,
    dimensions: usize,
    batch_size: usize,
}

impl VoyageEmbedder {
    pub fn new(api_key: impl Into<String>, cfg: VoyageConfig) -> Result<Self, VoyageBuildError> {
        let api_key = api_key.into();
        if api_key.is_empty() { return Err(VoyageBuildError::MissingApiKey); }
        let timeout = cfg.timeout.unwrap_or(Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        let client = Client::builder().timeout(timeout).build().map_err(VoyageBuildError::Http)?;
        Ok(Self {
            client,
            endpoint: cfg.endpoint.unwrap_or_else(|| DEFAULT_ENDPOINT.to_string()),
            api_key,
            model: cfg.model,
            dimensions: cfg.dimensions,
            batch_size: cfg.batch_size.unwrap_or(DEFAULT_BATCH),
        })
    }

    pub fn from_env(cfg: VoyageConfig) -> Result<Self, VoyageBuildError> {
        let key = std::env::var("VOYAGE_API_KEY").map_err(|_| VoyageBuildError::MissingApiKey)?;
        Self::new(key, cfg)
    }

    /// Embed a single query string with `input_type=query` (Voyage's
    /// recommendation: documents and queries get distinct prompts so
    /// asymmetric retrieval works as designed).
    pub fn embed_query(&self, q: &str) -> Result<Vector, EmbedderError> {
        let mut v = self.embed_with(&[q], "query")?;
        Ok(v.pop().expect("non-empty"))
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        self.embed_with(texts, "document")
    }

    fn embed_with(&self, texts: &[&str], input_type: &'static str) -> Result<Vec<Vector>, EmbedderError> {
        let req = EmbedRequest { input: texts, model: &self.model, input_type };
        let resp = self.client.post(&self.endpoint).bearer_auth(&self.api_key).json(&req).send();
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
        let parsed: EmbedResponse = resp.json().map_err(|e| EmbedderError::Terminal(format!("decode: {e}")))?;
        if parsed.data.len() != texts.len() {
            return Err(EmbedderError::Terminal(format!(
                "response has {} embeddings, expected {}",
                parsed.data.len(), texts.len()
            )));
        }
        Ok(parsed.data.into_iter().map(|d| d.embedding).collect())
    }
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    input: &'a [&'a str],
    model: &'a str,
    /// Voyage distinguishes "document" vs "query" prompts. We only embed
    /// indexable content here; query-time embedding can pass another flag.
    input_type: &'a str,
}

#[derive(Deserialize)]
struct EmbedResponse { data: Vec<EmbedDatum> }

#[derive(Deserialize)]
struct EmbedDatum { embedding: Vector }

impl AdapterIdentity for VoyageEmbedder {
    fn name(&self) -> &str { "embedder-voyage" }
    fn version(&self) -> StageVersion { StageVersion(self.model.clone()) }
    fn config_hash(&self) -> ConfigHash {
        ConfigHash(format!("model={};dim={}", self.model, self.dimensions))
    }
}

impl Embedder for VoyageEmbedder {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        if texts.is_empty() { return Err(EmbedderError::Terminal("empty batch".into())); }
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
    fn empty_api_key_is_terminal_build_error() {
        let r = VoyageEmbedder::new("", VoyageConfig {
            model: "voyage-code-3".into(), dimensions: 4, endpoint: None,
            batch_size: None, timeout: None,
        });
        assert!(matches!(r, Err(VoyageBuildError::MissingApiKey)));
    }

    #[test]
    fn empty_batch_is_terminal_via_embed() {
        let e = VoyageEmbedder::new("k", VoyageConfig {
            model: "m".into(), dimensions: 4, endpoint: Some("http://localhost".into()),
            batch_size: None, timeout: None,
        }).unwrap();
        assert!(matches!(e.embed(&[]).unwrap_err(), EmbedderError::Terminal(_)));
    }
}
