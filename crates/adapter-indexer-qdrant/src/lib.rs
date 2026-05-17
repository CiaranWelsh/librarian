//! Qdrant-backed `Indexer`. Sync trait wraps an internal Tokio runtime so the
//! domain stays sync.

mod error;
mod indexer;
mod payload;
mod point_id;
mod search;

pub use error::QdrantError;
pub use indexer::QdrantIndexer;
pub use point_id::point_id;
pub use search::SearchHit;
