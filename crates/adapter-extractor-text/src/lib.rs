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
