#[derive(Debug, thiserror::Error)]
pub enum PdfExtractError {
    #[error("io: {0}")]
    Io(#[source] std::io::Error),
    #[error("marker: {0}")]
    Marker(String),
}
