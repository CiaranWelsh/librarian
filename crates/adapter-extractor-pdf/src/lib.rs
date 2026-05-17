//! PDF text extractor — backed by marker (vision-LM PDF → markdown).

mod error;
mod extractor;

pub use error::PdfExtractError;
pub use extractor::PdfExtractor;

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::AdapterIdentity;

    #[test]
    fn adapter_identity_stable() {
        let e = PdfExtractor::new();
        assert_eq!(e.name(), "extractor-pdf");
        assert_eq!(e.version().0, "0.2.0-marker");
    }
}
