use crate::adapter_identity::AdapterIdentity;
use crate::chunk::Chunk;
use crate::document::{Document, ExtractedText};

pub trait Chunker: AdapterIdentity {
    type Error: std::error::Error + Send + Sync + 'static;
    fn chunk(&self, doc: &Document, text: ExtractedText) -> Result<Vec<Chunk>, Self::Error>;
}
