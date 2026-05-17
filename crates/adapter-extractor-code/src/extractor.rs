//! Reads the file as UTF-8 and emits a single Code span. Walking + filtering
//! happens in the CLI; see `filter::should_include`.

use librarian_domain::{
    AdapterIdentity, ConfigHash, Document, ExtractedText, Extractor, SpanKind, StageVersion,
    TextSpan,
};

use crate::error::CodeExtractError;

#[derive(Default)]
pub struct CodeExtractor;

impl CodeExtractor {
    pub fn new() -> Self { Self }
}

impl AdapterIdentity for CodeExtractor {
    fn name(&self) -> &str { "extractor-code" }
    fn version(&self) -> StageVersion { StageVersion("0.1.0".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash("default".into()) }
}

impl Extractor for CodeExtractor {
    type Error = CodeExtractError;
    fn extract(&self, doc: &Document) -> Result<ExtractedText, Self::Error> {
        let bytes = std::fs::read(&doc.path)?;
        let text = String::from_utf8(bytes).map_err(|e| CodeExtractError::Encoding(e.to_string()))?;
        let len = text.len();
        Ok(ExtractedText { spans: vec![TextSpan {
            kind: SpanKind::Code,
            text,
            page: None,
            byte_range: 0..len,
        }]})
    }
}
