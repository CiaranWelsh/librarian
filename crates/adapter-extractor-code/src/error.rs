#[derive(Debug, thiserror::Error)]
pub enum CodeExtractError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("not utf-8: {0}")]
    Encoding(String),
}
