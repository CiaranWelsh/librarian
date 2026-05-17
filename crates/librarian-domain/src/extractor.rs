use crate::adapter_identity::AdapterIdentity;
use crate::document::{Document, ExtractedText};

pub trait Extractor: AdapterIdentity {
    type Error: std::error::Error + Send + Sync + 'static;
    fn extract(&self, doc: &Document) -> Result<ExtractedText, Self::Error>;
}
