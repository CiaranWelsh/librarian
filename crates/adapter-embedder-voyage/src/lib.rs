//! Voyage AI embeddings adapter (e.g. `voyage-code-3`).

mod classify;
mod config;
mod embedder;
mod error;

pub use config::VoyageConfig;
pub use embedder::VoyageEmbedder;
pub use error::VoyageBuildError;
