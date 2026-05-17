//! Server-side TOML config + embedder construction helpers.

use adapter_embedder_openai::{OpenAiConfig, OpenAiEmbedder};
use adapter_embedder_stub::StubEmbedder;
use librarian_domain::Embedder;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub collection: String,
    pub qdrant: QdrantCfg,
    pub paths: Paths,
    pub embedder: EmbedderCfg,
}

#[derive(Debug, Deserialize)]
pub struct QdrantCfg { pub url: String }

#[derive(Debug, Deserialize)]
pub struct Paths { pub manifest: PathBuf }

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EmbedderCfg {
    Stub,
    Openai { model: String, dimensions: usize, #[serde(default)] batch_size: Option<usize> },
}

/// Dimension lookup used by composition root before constructing the concrete embedder.
pub fn embedder_dim(cfg: &EmbedderCfg) -> u64 {
    match cfg {
        EmbedderCfg::Stub => StubEmbedder::new().dimension() as u64,
        EmbedderCfg::Openai { dimensions, .. } => *dimensions as u64,
    }
}

pub fn embed_query(cfg: &EmbedderCfg, q: &str) -> Result<Vec<f32>, String> {
    match cfg {
        EmbedderCfg::Stub => StubEmbedder::new().embed(&[q]).map(|v| v.into_iter().next().unwrap()).map_err(|e| e.to_string()),
        EmbedderCfg::Openai { model, dimensions, batch_size } => {
            let e = OpenAiEmbedder::from_env(OpenAiConfig {
                model: model.clone(), dimensions: *dimensions,
                endpoint: None, batch_size: *batch_size, timeout: None,
            }).map_err(|e| e.to_string())?;
            e.embed(&[q]).map(|v| v.into_iter().next().unwrap()).map_err(|e| e.to_string())
        }
    }
}
