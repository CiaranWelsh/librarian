//! Daemon config + runtime-chosen embedder via enum dispatch (no `Box<dyn>`).

use std::time::Duration;

use adapter_embedder_openai::{OpenAiConfig, OpenAiEmbedder};
use adapter_embedder_stub::StubEmbedder;
use librarian_domain::{
    AdapterIdentity, ConfigHash, Embedder, EmbedderError, StageVersion, Vector,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DaemonConfig {
    /// e.g. "0.0.0.0:6700"
    pub bind: String,
    /// qdrant gRPC url, e.g. "http://localhost:6334"
    pub qdrant_url: String,
    #[serde(default = "default_embeds")]
    pub max_concurrent_embeds: usize,
    pub embedder: EmbedderCfg,
}

fn default_embeds() -> usize {
    8
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum EmbedderCfg {
    Stub,
    Openai {
        model: String,
        dimensions: usize,
        batch_size: Option<usize>,
    },
}

/// Runtime-selected embedder. Enum dispatch keeps the daemon generic-free at
/// the bin boundary without `Box<dyn Embedder>` (CLAUDE.md; Rust in Action
/// Ch. 2.2.6 — dynamic dispatch "can be viewed as an antipattern").
pub enum AppEmbedder {
    Stub(StubEmbedder),
    Openai(OpenAiEmbedder),
}

impl AppEmbedder {
    pub fn from_cfg(cfg: &EmbedderCfg) -> Result<Self, String> {
        match cfg {
            EmbedderCfg::Stub => Ok(AppEmbedder::Stub(StubEmbedder::new())),
            EmbedderCfg::Openai {
                model,
                dimensions,
                batch_size,
            } => {
                let oc = OpenAiConfig {
                    model: model.clone(),
                    dimensions: *dimensions,
                    endpoint: None,
                    batch_size: *batch_size,
                    timeout: Some(Duration::from_secs(30)),
                };
                let e = OpenAiEmbedder::from_env(oc).map_err(|e| e.to_string())?;
                Ok(AppEmbedder::Openai(e))
            }
        }
    }
}

impl AdapterIdentity for AppEmbedder {
    fn name(&self) -> &str {
        match self {
            AppEmbedder::Stub(e) => e.name(),
            AppEmbedder::Openai(e) => e.name(),
        }
    }

    fn version(&self) -> StageVersion {
        match self {
            AppEmbedder::Stub(e) => e.version(),
            AppEmbedder::Openai(e) => e.version(),
        }
    }

    fn config_hash(&self) -> ConfigHash {
        match self {
            AppEmbedder::Stub(e) => e.config_hash(),
            AppEmbedder::Openai(e) => e.config_hash(),
        }
    }
}

impl Embedder for AppEmbedder {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        match self {
            AppEmbedder::Stub(e) => e.embed(texts),
            AppEmbedder::Openai(e) => e.embed(texts),
        }
    }

    fn dimension(&self) -> usize {
        match self {
            AppEmbedder::Stub(e) => e.dimension(),
            AppEmbedder::Openai(e) => e.dimension(),
        }
    }
}
