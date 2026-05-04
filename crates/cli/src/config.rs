//! TOML config schema, one file per collection (F-7.2).

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub collection: String,
    pub qdrant: QdrantConfig,
    pub paths: Paths,
    pub embedder: EmbedderConfig,
    #[serde(default)]
    pub ingest: IngestConfig,
}

#[derive(Debug, Deserialize)]
pub struct QdrantConfig {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct Paths {
    pub cache: PathBuf,
    pub manifest: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EmbedderConfig {
    Stub,
    Openai {
        model: String,
        dimensions: usize,
        #[serde(default)]
        batch_size: Option<usize>,
    },
}

#[derive(Debug, Deserialize, Default)]
pub struct IngestConfig {
    #[serde(default = "default_content_type")]
    pub content_type: String, // "book" | "paper" | "code"
    #[serde(default = "default_extractor")]
    pub extractor: String,    // "text" | "pdf"
}

fn default_content_type() -> String { "book".into() }
fn default_extractor() -> String { "text".into() }

impl Config {
    pub fn load(path: &std::path::Path) -> Result<Self, ConfigError> {
        let s = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
        toml::from_str(&s).map_err(ConfigError::Parse)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config io: {0}")]
    Io(#[source] std::io::Error),
    #[error("config parse: {0}")]
    Parse(#[source] toml::de::Error),
}
