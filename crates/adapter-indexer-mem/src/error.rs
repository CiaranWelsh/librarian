#[derive(Debug, thiserror::Error)]
pub enum MemIndexerError {
    #[error("length mismatch: {chunks} chunks vs {vectors} vectors")]
    LengthMismatch { chunks: usize, vectors: usize },
    #[error("poisoned")]
    Poisoned,
}
