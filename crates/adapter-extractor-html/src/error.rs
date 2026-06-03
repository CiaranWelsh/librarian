#[derive(Debug, thiserror::Error)]
pub enum HtmlExtractError {
    #[error("io: {0}")]
    Io(#[source] std::io::Error),
    #[error("pandoc: {0}")]
    Pandoc(String),
    #[error("unsupported extension: {0}")]
    UnsupportedExtension(String),
    #[error("empty extraction")]
    Empty,
}
