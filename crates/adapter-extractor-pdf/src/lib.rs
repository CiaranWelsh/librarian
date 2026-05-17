//! PDF text extractor.

mod error;
mod extractor;

pub use error::PdfExtractError;
pub use extractor::PdfExtractor;

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::{ContentType, Document, Extractor, SourceHash, SourceId};

    #[test]
    fn missing_pdf_yields_pdf_error() {
        let doc = Document {
            source_id: SourceId("s".into()),
            source_hash: SourceHash("h".into()),
            content_type: ContentType::Book,
            path: "/no/such/file.pdf".into(),
            work_id: None,
        };
        let r = PdfExtractor.extract(&doc);
        assert!(matches!(r, Err(PdfExtractError::Pdf(_))));
    }
}
