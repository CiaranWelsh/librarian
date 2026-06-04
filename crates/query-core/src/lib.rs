//! query-core — framework-free query logic over the `Embedder` and `Searcher`
//! ports. No HTTP, no Qdrant types. Generic (no `Box<dyn>`).

mod confidence;
mod error;
mod service;

pub use confidence::{
    retrieval_confidence, retrieval_confidence_with, ConfidenceLabel, ConfidenceThresholds,
    RetrievalConfidence,
};
pub use error::QueryError;
pub use service::QueryService;
