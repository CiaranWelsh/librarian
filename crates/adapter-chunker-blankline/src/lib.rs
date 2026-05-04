use librarian_domain::{
    AdapterIdentity, BookMeta, Chunk, ChunkId, ChunkPayload, Chunker, CodeMeta, ConfigHash,
    ContentType, Document, ExtractedText, PaperMeta, Provenance, StageVersion,
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
        let title = doc
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let mut chunks = Vec::new();
        let mut idx: u32 = 0;
        for span in &text.spans {
            for para in span.text.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
                chunks.push(Chunk {
                    chunk_id: ChunkId(format!("{}#{}", doc.source_id.0, idx)),
                    source_id: doc.source_id.clone(),
                    chunk_index: idx,
                    text: para.to_string(),
                    payload: payload_for(doc, &title, span.page),
                    provenance: Provenance::default(),
                });
                idx += 1;
            }
        }
        if chunks.is_empty() {
            return Err(BlankLineChunkError);
        }
        Ok(chunks)
    }
}

fn payload_for(doc: &Document, title: &str, page: Option<u32>) -> ChunkPayload {
    match doc.content_type {
        ContentType::Book => ChunkPayload::Book(BookMeta {
            title: title.to_string(),
            author: None,
            chapter: None,
            section: None,
            page,
        }),
        ContentType::Paper => ChunkPayload::Paper(PaperMeta {
            title: title.to_string(),
            authors: vec![],
            section: None,
            page_start: page,
            page_end: page,
        }),
        ContentType::Code => ChunkPayload::Code(CodeMeta {
            repo: None,
            commit: None,
            file_path: doc.path.display().to_string(),
            language: None,
            symbol: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::{ContentType, Document, SourceHash, SpanKind, TextSpan};

    fn doc() -> Document {
        Document {
            source_id: librarian_domain::SourceId("s".into()),
            source_hash: SourceHash("h".into()),
            content_type: ContentType::Book,
            path: "f.txt".into(),
            work_id: None,
        }
    }

    fn span(text: &str) -> TextSpan {
        TextSpan { kind: SpanKind::Paragraph, text: text.into(), page: None, byte_range: 0..text.len() }
    }

    #[test]
    fn empty_spans_errors() {
        let r = BlankLineChunker.chunk(&doc(), ExtractedText { spans: vec![] });
        assert!(r.is_err());
    }

    #[test]
    fn single_paragraph_yields_one_chunk_indexed_zero() {
        let r = BlankLineChunker.chunk(&doc(), ExtractedText { spans: vec![span("hello")] }).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].chunk_index, 0);
        assert_eq!(r[0].text, "hello");
    }

    #[test]
    fn three_paragraphs_yield_three_chunks_with_strict_index() {
        let text = ExtractedText { spans: vec![span("a\n\nb\n\nc")] };
        let r = BlankLineChunker.chunk(&doc(), text).unwrap();
        assert_eq!(r.iter().map(|c| c.chunk_index).collect::<Vec<_>>(), vec![0, 1, 2]);
    }

    #[test]
    fn collapses_runs_of_blank_lines() {
        let text = ExtractedText { spans: vec![span("a\n\n\n\nb")] };
        let r = BlankLineChunker.chunk(&doc(), text).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn provenance_starts_empty() {
        let r = BlankLineChunker.chunk(&doc(), ExtractedText { spans: vec![span("x")] }).unwrap();
        assert!(r[0].provenance.0.is_empty(), "runner appends provenance, not chunker");
    }
}
