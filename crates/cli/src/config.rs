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
    #[serde(default)]
    pub snapshot: SnapshotConfig,
    #[serde(default)]
    pub quality: QualityConfig,
}

#[derive(Debug, Deserialize)]
pub struct QdrantConfig {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct Paths {
    pub cache: PathBuf,
    pub manifest: PathBuf,
    #[serde(default)]
    pub snapshots: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Default)]
pub struct SnapshotConfig {
    #[serde(default = "default_retention")]
    pub retention: usize,
}

fn default_retention() -> usize {
    5
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
    Voyage {
        model: String,
        dimensions: usize,
        #[serde(default)]
        batch_size: Option<usize>,
    },
}

#[derive(Debug, Deserialize)]
pub struct IngestConfig {
    #[serde(default = "default_content_type")]
    pub content_type: String, // "book" | "paper" | "code"
    #[serde(default = "default_extractor")]
    pub extractor: String, // "text" | "pdf"
    /// Chunker to use for text content: "recursive" (issue 027, default) or "blankline".
    /// Ignored for code content (always uses CodeChunker).
    #[serde(default = "default_chunker")]
    pub chunker: String,
    /// Recursive chunker target size in characters (~512 tokens ≈ 2000 chars).
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
    /// Recursive chunker overlap in characters (~10%).
    #[serde(default = "default_chunk_overlap")]
    pub chunk_overlap: usize,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            content_type: default_content_type(),
            extractor: default_extractor(),
            chunker: default_chunker(),
            chunk_size: default_chunk_size(),
            chunk_overlap: default_chunk_overlap(),
        }
    }
}

fn default_content_type() -> String {
    "book".into()
}
fn default_extractor() -> String {
    "text".into()
}
fn default_chunker() -> String {
    "recursive".into()
}
fn default_chunk_size() -> usize {
    2000
}
fn default_chunk_overlap() -> usize {
    200
}

/// Ingest-quality config (ADR-0006). Maps to `librarian_domain::QualityConfig`.
#[derive(Debug, Deserialize, Default)]
pub struct QualityConfig {
    #[serde(default)]
    pub sections: SectionsConfig,
    #[serde(default)]
    pub garble: GarbleConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct SectionsConfig {
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub keep: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct GarbleConfig {
    #[serde(default = "default_flag_above")]
    pub flag_above: f64,
}

impl Default for GarbleConfig {
    fn default() -> Self {
        Self {
            flag_above: default_flag_above(),
        }
    }
}

fn default_flag_above() -> f64 {
    1.0
}

impl QualityConfig {
    pub fn to_domain(&self) -> librarian_domain::QualityConfig {
        librarian_domain::QualityConfig {
            sections: librarian_domain::SectionConfig {
                exclude: self.sections.exclude.clone(),
                keep: self.sections.keep.clone(),
            },
            garble: librarian_domain::GarbleConfig {
                flag_above: self.garble.flag_above,
            },
        }
    }
}

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
