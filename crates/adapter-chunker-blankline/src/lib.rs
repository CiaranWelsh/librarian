use librarian_domain::{
    AdapterIdentity, BookMeta, Chunk, ChunkId, ChunkPayload, Chunker, ConfigHash, Document,
    ExtractedText, Provenance, StageVersion,
};

/// Splits the concatenated extracted text on blank lines.
#[derive(Default)]
pub struct BlankLineChunker;

impl BlankLineChunker {
    pub fn new() -> Self { Self }
}

#[derive(Debug, thiserror::Error)]
#[error("blank-line chunker: empty input")]
pub struct BlankLineChunkError;

impl AdapterIdentity for BlankLineChunker {
    fn name(&self) -> &str { "chunker-blankline" }
    fn version(&self) -> StageVersion { StageVersion("0.1.0".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash("default".into()) }
}

impl Chunker for BlankLineChunker {
    type Error = BlankLineChunkError;

    fn chunk(
        &self,
        doc: &Document,
        text: ExtractedText,
    ) -> Result<Vec<Chunk>, Self::Error> {
        if text.spans.is_empty() {
            return Err(BlankLineChunkError);
        }
        let joined: String = text
            .spans
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let mut chunks = Vec::new();
        for (idx, para) in joined
            .split("\n\n")
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .enumerate()
        {
            chunks.push(Chunk {
                chunk_id: ChunkId(format!("{}#{}", doc.source_id.0, idx)),
                source_id: doc.source_id.clone(),
                chunk_index: idx as u32,
                text: para.to_string(),
                payload: ChunkPayload::Book(BookMeta {
                    title: doc.path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string(),
                    author: None,
                    chapter: None,
                    section: None,
                    page: None,
                }),
                provenance: Provenance::default(),
            });
        }
        Ok(chunks)
    }
}
