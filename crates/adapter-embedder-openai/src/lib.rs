//! OpenAI embeddings adapter. Sync, blocking `reqwest`.
//!
//! Error classification (slice 010 AC):
//! - HTTP 5xx, request timeout, transport error, 408, 429 → `Recoverable`.
//! - HTTP 4xx (excluding 408/429), auth/quota → `Terminal`.
//! - The runner / `FallbackEmbedder` decides whether to retry.

mod classify;
mod config;
mod embedder;
mod error;

pub use config::OpenAiConfig;
pub use embedder::OpenAiEmbedder;
pub use error::OpenAiBuildError;
