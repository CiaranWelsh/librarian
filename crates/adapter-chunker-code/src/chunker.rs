//! Line-window chunker for code. Splits source into overlapping windows.
//! TODO: tree-sitter for symbol-aware chunking once we hit a corpus that
//! demands it.

use librarian_domain::{
    AdapterIdentity, Chunk, ChunkId, ChunkPayload, Chunker, CodeMeta, ConfigHash, Document,
    ExtractedText, Provenance, StageVersion,
};

use crate::error::CodeChunkError;
use crate::language::detect_language;

pub struct CodeChunker {
    pub window_lines: usize,
    pub overlap_lines: usize,
}

impl Default for CodeChunker {
    fn default() -> Self { Self { window_lines: 30, overlap_lines: 5 } }
}

impl CodeChunker {
    pub fn new() -> Self { Self::default() }
    pub fn with_window(window_lines: usize, overlap_lines: usize) -> Self {
        assert!(window_lines > overlap_lines, "window must exceed overlap");
        Self { window_lines, overlap_lines }
    }
}

impl AdapterIdentity for CodeChunker {
    fn name(&self) -> &str { "chunker-code" }
    fn version(&self) -> StageVersion { StageVersion("0.1.0".into()) }
    fn config_hash(&self) -> ConfigHash {
        ConfigHash(format!("w={};o={}", self.window_lines, self.overlap_lines))
    }
}

impl Chunker for CodeChunker {
    type Error = CodeChunkError;

    fn chunk(&self, doc: &Document, text: ExtractedText) -> Result<Vec<Chunk>, Self::Error> {
        let body: String = text.spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("\n");
        if body.trim().is_empty() {
            return Err(CodeChunkError::Empty);
        }
        let lines: Vec<&str> = body.lines().collect();
        if lines.is_empty() { return Err(CodeChunkError::Empty); }

        let lang = detect_language(&doc.path);
        let file_path = doc.path.display().to_string();

        let mut chunks = Vec::new();
        let stride = self.window_lines.saturating_sub(self.overlap_lines).max(1);

        let mut idx: u32 = 0;
        let mut start = 0usize;
        while start < lines.len() {
            let end = (start + self.window_lines).min(lines.len());
            let window = lines[start..end].join("\n");
            chunks.push(Chunk {
                chunk_id: ChunkId(format!("{}#{}", doc.source_id.0, idx)),
                source_id: doc.source_id.clone(),
                chunk_index: idx,
                text: window,
                payload: ChunkPayload::Code(CodeMeta {
                    repo: None,
                    commit: None,
                    file_path: file_path.clone(),
                    language: lang.clone(),
                    symbol: None,
                }),
                provenance: Provenance::default(),
            });
            idx += 1;
            if end == lines.len() { break; }
            start += stride;
        }
        Ok(chunks)
    }
}
