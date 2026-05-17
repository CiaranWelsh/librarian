#[derive(Debug, thiserror::Error)]
pub enum CodeChunkError {
    #[error("empty source")]
    Empty,
}
