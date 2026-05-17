#[derive(Debug, thiserror::Error)]
pub enum QdrantError {
    #[error("runtime: {0}")]
    Runtime(#[source] std::io::Error),
    #[error("client: {0}")]
    Client(String),
    #[error("length mismatch: {chunks} chunks vs {vectors} vectors")]
    LengthMismatch { chunks: usize, vectors: usize },
}

impl QdrantError {
    pub(crate) fn client<E: std::fmt::Display>(e: E) -> Self {
        QdrantError::Client(e.to_string())
    }
}
