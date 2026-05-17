#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("io: {0}")]
    Io(#[source] std::io::Error),
    #[error("http: {0}")]
    Http(String),
    #[error("qdrant: {0}")]
    Qdrant(String),
    #[error("snapshot not found on NAS: {0}")]
    NotFound(String),
}
