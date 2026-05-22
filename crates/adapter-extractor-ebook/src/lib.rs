//! EPUB / MOBI extractor — pandoc → cleaner → markdown.

mod cleaner;
mod error;
mod extractor;

pub use cleaner::clean;
pub use error::EbookExtractError;
pub use extractor::EbookExtractor;

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::AdapterIdentity;

    #[test]
    fn adapter_identity_stable() {
        let e = EbookExtractor::new();
        assert_eq!(e.name(), "extractor-ebook");
        assert_eq!(e.version().0, "0.1.0-pandoc-clean");
    }
}
