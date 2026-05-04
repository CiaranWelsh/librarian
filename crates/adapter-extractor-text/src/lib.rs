use librarian_domain::{
    AdapterIdentity, ConfigHash, Document, ExtractedText, Extractor, SpanKind, StageVersion,
    TextSpan,
};

/// Trivial extractor: reads UTF-8 file, emits one Paragraph span over the whole text.
#[derive(Default)]
pub struct TextExtractor;

impl TextExtractor {
    pub fn new() -> Self { Self }
}

#[derive(Debug, thiserror::Error)]
pub enum TextExtractError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl AdapterIdentity for TextExtractor {
    fn name(&self) -> &str { "extractor-text" }
    fn version(&self) -> StageVersion { StageVersion("0.1.0".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash("default".into()) }
}

impl Extractor for TextExtractor {
    type Error = TextExtractError;

    fn extract(&self, doc: &Document) -> Result<ExtractedText, Self::Error> {
        let text = std::fs::read_to_string(&doc.path)?;
        let len = text.len();
        Ok(ExtractedText {
            spans: vec![TextSpan {
                kind: SpanKind::Paragraph,
                text,
                page: None,
                byte_range: 0..len,
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::{ContentType, SourceHash, SourceId};

    fn doc(path: &str) -> Document {
        Document {
            source_id: SourceId("s".into()),
            source_hash: SourceHash("h".into()),
            content_type: ContentType::Book,
            path: path.into(),
            work_id: None,
        }
    }

    #[test]
    fn missing_file_yields_io_error() {
        let r = TextExtractor.extract(&doc("/nonexistent/path.txt"));
        assert!(matches!(r, Err(TextExtractError::Io(_))));
    }

    #[test]
    fn reads_a_real_file() {
        let dir = std::env::temp_dir().join(format!("librarian-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.txt");
        std::fs::write(&p, "hello world").unwrap();

        let r = TextExtractor.extract(&doc(p.to_str().unwrap())).unwrap();
        assert_eq!(r.spans.len(), 1);
        assert_eq!(r.spans[0].text, "hello world");
        assert_eq!(r.spans[0].byte_range, 0..11);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
