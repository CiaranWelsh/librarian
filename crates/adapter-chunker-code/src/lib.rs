//! Line-window chunker for code. Splits source into overlapping windows.
//! TODO: tree-sitter for symbol-aware chunking once we hit a corpus that
//! demands it.

use librarian_domain::{
    AdapterIdentity, Chunk, ChunkId, ChunkPayload, Chunker, CodeMeta, ConfigHash, Document,
    ExtractedText, Provenance, StageVersion,
};
use std::path::Path;

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

#[derive(Debug, thiserror::Error)]
pub enum CodeChunkError {
    #[error("empty source")]
    Empty,
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

pub fn detect_language(path: &Path) -> Option<String> {
    let ext = path.extension().and_then(|s| s.to_str())?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" | "jsx" => "javascript",
        "ts" | "tsx" => "typescript",
        "go" => "go",
        "java" => "java",
        "kt" => "kotlin",
        "swift" => "swift",
        "c" | "h" => "c",
        "cc" | "cpp" | "hpp" => "cpp",
        "cs" => "csharp",
        "rb" => "ruby",
        "sh" | "bash" | "zsh" => "shell",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "md" => "markdown",
        _ => return None,
    }.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::{ContentType, SourceHash, SourceId, SpanKind, TextSpan};

    fn doc(path: &str) -> Document {
        Document {
            source_id: SourceId("s".into()),
            source_hash: SourceHash("h".into()),
            content_type: ContentType::Code,
            path: path.into(),
            work_id: None,
        }
    }

    fn extracted(text: &str) -> ExtractedText {
        ExtractedText { spans: vec![TextSpan {
            kind: SpanKind::Code, text: text.into(), page: None, byte_range: 0..text.len(),
        }]}
    }

    #[test]
    fn detects_language_from_extension() {
        assert_eq!(detect_language(Path::new("foo.rs")).as_deref(), Some("rust"));
        assert_eq!(detect_language(Path::new("foo.py")).as_deref(), Some("python"));
        assert_eq!(detect_language(Path::new("foo.unknown")), None);
    }

    #[test]
    fn empty_source_errors() {
        let r = CodeChunker::new().chunk(&doc("foo.rs"), extracted("   "));
        assert!(matches!(r, Err(CodeChunkError::Empty)));
    }

    #[test]
    fn short_file_yields_one_chunk() {
        let body = "fn main() { println!(\"hi\"); }";
        let r = CodeChunker::new().chunk(&doc("foo.rs"), extracted(body)).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].chunk_index, 0);
        match &r[0].payload {
            ChunkPayload::Code(m) => {
                assert_eq!(m.language.as_deref(), Some("rust"));
                assert_eq!(m.file_path, "foo.rs");
            }
            _ => panic!("expected Code payload"),
        }
    }

    #[test]
    fn long_file_splits_into_overlapping_windows() {
        let body: String = (0..100).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        // Default window=30, overlap=5 → stride=25. 100 lines = 4 windows.
        let r = CodeChunker::new().chunk(&doc("foo.rs"), extracted(&body)).unwrap();
        assert!(r.len() >= 4);
        // Adjacent chunks share `overlap` lines.
        let first_lines: Vec<&str> = r[0].text.lines().collect();
        let second_lines: Vec<&str> = r[1].text.lines().collect();
        // The last `overlap` lines of chunk 0 equal the first `overlap` lines of chunk 1.
        assert_eq!(&first_lines[first_lines.len()-5..], &second_lines[0..5]);
    }

    #[test]
    fn chunk_indexes_are_strictly_increasing_from_zero() {
        let body: String = (0..100).map(|i| format!("l{i}")).collect::<Vec<_>>().join("\n");
        let r = CodeChunker::new().chunk(&doc("foo.rs"), extracted(&body)).unwrap();
        let idxs: Vec<u32> = r.iter().map(|c| c.chunk_index).collect();
        for w in idxs.windows(2) { assert_eq!(w[1], w[0] + 1); }
        assert_eq!(idxs[0], 0);
    }

    #[test]
    fn payload_is_code_meta_with_file_path() {
        let r = CodeChunker::new().chunk(&doc("/repo/src/lib.rs"), extracted("fn x(){}")).unwrap();
        match &r[0].payload {
            ChunkPayload::Code(m) => assert_eq!(m.file_path, "/repo/src/lib.rs"),
            _ => panic!("not Code payload"),
        }
    }

    #[test]
    fn provenance_starts_empty() {
        let r = CodeChunker::new().chunk(&doc("a.rs"), extracted("x")).unwrap();
        assert!(r[0].provenance.0.is_empty());
    }
}
