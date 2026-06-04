//! query-core — framework-free query logic over the `Embedder` and `Searcher`
//! ports. No HTTP, no Qdrant types. Generic (no `Box<dyn>`).

mod error;
mod service;

pub use error::QueryError;
pub use service::QueryService;
