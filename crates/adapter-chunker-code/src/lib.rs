//! Code-aware chunker.

mod chunker;
mod error;
mod language;

pub use chunker::CodeChunker;
pub use error::CodeChunkError;
pub use language::detect_language;

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::{
        ChunkPayload, ContentType, Document, ExtractedText, SourceHash, SourceId, SpanKind, TextSpan, Chunker,
    };

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
        let r = CodeChunker::new().chunk(&doc("foo.rs"), extracted(&body)).unwrap();
        assert!(r.len() >= 4);
        let first_lines: Vec<&str> = r[0].text.lines().collect();
        let second_lines: Vec<&str> = r[1].text.lines().collect();
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
