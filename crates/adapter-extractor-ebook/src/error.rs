#[derive(Debug, thiserror::Error)]
pub enum EbookExtractError {
    #[error("io: {0}")]
    Io(#[source] std::io::Error),
    #[error("pandoc: {0}")]
    Pandoc(String),
    #[error("ebook-convert: {0}")]
    Calibre(String),
    #[error("unsupported extension: {0}")]
    UnsupportedExtension(String),
    #[error("empty extracted output")]
    Empty,
}
