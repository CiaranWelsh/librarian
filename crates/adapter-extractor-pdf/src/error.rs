#[derive(Debug, thiserror::Error)]
pub enum PdfExtractError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("pdf: {0}")]
    Pdf(#[source] lopdf::Error),
}
