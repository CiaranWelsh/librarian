#[derive(Debug, thiserror::Error)]
pub enum FsCacheError {
    #[error("io: {0}")]
    Io(#[source] std::io::Error),
}
