//! Code-file extractor.

mod error;
mod extractor;
mod filter;

pub use error::CodeExtractError;
pub use extractor::CodeExtractor;
pub use filter::{should_include, DEFAULT_INCLUDE_EXTS, DEFAULT_SKIP_DIRS};

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::{ContentType, Document, Extractor, SourceHash, SourceId, SpanKind};
    use std::path::Path;

    fn doc(p: &Path) -> Document {
        Document {
            source_id: SourceId(p.display().to_string()),
            source_hash: SourceHash("h".into()),
            content_type: ContentType::Code,
            path: p.to_path_buf(),
            work_id: None,
        }
    }

    #[test]
    fn reads_utf8_source_file_as_single_code_span() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("hello.rs");
        std::fs::write(&p, "fn main() {}\n").unwrap();
        let r = CodeExtractor.extract(&doc(&p)).unwrap();
        assert_eq!(r.spans.len(), 1);
        assert!(matches!(r.spans[0].kind, SpanKind::Code));
        assert_eq!(r.spans[0].text, "fn main() {}\n");
    }

    #[test]
    fn missing_file_yields_io_error() {
        let r = CodeExtractor.extract(&doc(Path::new("/no/such/file.rs")));
        assert!(matches!(r, Err(CodeExtractError::Io(_))));
    }

    #[test]
    fn non_utf8_is_classified_as_encoding_error() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("binary.rs");
        std::fs::write(&p, [0xff, 0xfe, 0x00, 0x00]).unwrap();
        let r = CodeExtractor.extract(&doc(&p));
        assert!(matches!(r, Err(CodeExtractError::Encoding(_))));
    }
}
