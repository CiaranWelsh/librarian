use librarian_domain::SearchError;

/// All ways a query can fail. The daemon maps each variant to an HTTP status
/// (ADR-0005 error table); `Search` carries the port's own `SearchError`.
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("empty query")]
    EmptyQuery,
    #[error("embedder temporarily unavailable: {0}")]
    EmbedRecoverable(String),
    #[error("embedder failed: {0}")]
    EmbedTerminal(String),
    #[error("embedding task panicked")]
    EmbedPanic,
    #[error(transparent)]
    Search(#[from] SearchError),
}
