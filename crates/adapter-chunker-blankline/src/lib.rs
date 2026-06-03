use librarian_domain::{
    AdapterIdentity, BookMeta, Chunk, ChunkId, ChunkPayload, Chunker, CodeMeta, ConfigHash,
    ContentType, Document, ExtractedText, PaperMeta, Provenance, StageVersion,
};

// Figure content_type isn't produced by the blank-line chunker — figures
// originate from a separate multimodal extractor (slice 017).

/// Default per-chunk character budget. Derived from the embedder limit: OpenAI
/// `text-embedding-3-large` accepts at most 8192 tokens per input. At a
/// conservative ~4 chars/token that is ~32k chars; we keep a wide margin (so a
/// chunk stays well under the limit even for token-dense content) and cap at
/// 20k chars (~5-6k tokens).
const DEFAULT_MAX_CHARS: usize = 20_000;

/// Default overlap between consecutive windows of an over-budget block — mirrors
/// `CodeChunker`'s 5-line overlap so a split block keeps local context across the
/// seam.
const DEFAULT_OVERLAP_LINES: usize = 5;

/// Splits the concatenated extracted text on blank lines. Any block exceeding
/// `max_chars` is further split into overlapping line-windows (the same windowing
/// policy `CodeChunker` uses) so no chunk exceeds the embedder's input limit.
pub struct BlankLineChunker {
    pub max_chars: usize,
    pub overlap_lines: usize,
}

impl Default for BlankLineChunker {
    fn default() -> Self {
        Self { max_chars: DEFAULT_MAX_CHARS, overlap_lines: DEFAULT_OVERLAP_LINES }
    }
}

impl BlankLineChunker {
    pub fn new() -> Self { Self::default() }

    /// Construct with an explicit budget. `max_chars` must be non-zero.
    pub fn with_budget(max_chars: usize, overlap_lines: usize) -> Self {
        assert!(max_chars > 0, "max_chars must be non-zero");
        Self { max_chars, overlap_lines }
    }

    /// Split one blank-line block into chunks no larger than `max_chars`.
    /// A block within budget is returned as-is (one window). An over-budget
    /// block is packed greedily into overlapping line-windows: each window grows
    /// line-by-line until the next line would breach the budget, then the next
    /// window restarts `overlap_lines` lines back. A single line that alone
    /// exceeds the budget is emitted whole (it cannot be line-split).
    fn windows(&self, block: &str) -> Vec<String> {
        if block.len() <= self.max_chars {
            return vec![block.to_string()];
        }
        let lines: Vec<&str> = block.lines().collect();
        let mut out = Vec::new();
        let mut start = 0usize;
        while start < lines.len() {
            let mut end = start;
            let mut len = 0usize;
            while end < lines.len() {
                let add = lines[end].len() + 1; // +1 for the rejoined newline
                if end > start && len + add > self.max_chars {
                    break;
                }
                len += add;
                end += 1;
            }
            out.push(lines[start..end].join("\n"));
            if end >= lines.len() {
                break;
            }
            // Advance with overlap, but always make progress past `start`.
            start = end.saturating_sub(self.overlap_lines).max(start + 1);
        }
        out
    }
}

#[derive(Debug, thiserror::Error)]
#[error("blank-line chunker: empty input")]
pub struct BlankLineChunkError;

impl AdapterIdentity for BlankLineChunker {
    fn name(&self) -> &str { "chunker-blankline" }
    fn version(&self) -> StageVersion { StageVersion("0.2.0".into()) }
    fn config_hash(&self) -> ConfigHash {
        ConfigHash(format!("max_chars={};overlap={}", self.max_chars, self.overlap_lines))
    }
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
            for block in span.text.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
                for window in self.windows(block) {
                    chunks.push(Chunk {
                        chunk_id: ChunkId(format!("{}#{}", doc.source_id.0, idx)),
                        source_id: doc.source_id.clone(),
                        chunk_index: idx,
                        text: window,
                        payload: payload_for(doc, &title, span.page),
                        provenance: Provenance::default(),
                    });
                    idx += 1;
                }
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
        let r = BlankLineChunker::new().chunk(&doc(), ExtractedText { spans: vec![] });
        assert!(r.is_err());
    }

    #[test]
    fn single_paragraph_yields_one_chunk_indexed_zero() {
        let r = BlankLineChunker::new().chunk(&doc(), ExtractedText { spans: vec![span("hello")] }).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].chunk_index, 0);
        assert_eq!(r[0].text, "hello");
    }

    #[test]
    fn three_paragraphs_yield_three_chunks_with_strict_index() {
        let text = ExtractedText { spans: vec![span("a\n\nb\n\nc")] };
        let r = BlankLineChunker::new().chunk(&doc(), text).unwrap();
        assert_eq!(r.iter().map(|c| c.chunk_index).collect::<Vec<_>>(), vec![0, 1, 2]);
    }

    #[test]
    fn collapses_runs_of_blank_lines() {
        let text = ExtractedText { spans: vec![span("a\n\n\n\nb")] };
        let r = BlankLineChunker::new().chunk(&doc(), text).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn provenance_starts_empty() {
        let r = BlankLineChunker::new().chunk(&doc(), ExtractedText { spans: vec![span("x")] }).unwrap();
        assert!(r[0].provenance.0.is_empty(), "runner appends provenance, not chunker");
    }

    // ----- bounded-chunking (issue 026) -----

    // B1: an over-budget multi-line block splits into >1 window, each <= budget.
    // Source: issue 026 — embedder 8192-token limit; CodeChunker windowing policy.
    #[test]
    fn b1_oversized_block_splits_into_windows_under_budget() {
        let block: String = (0..50).map(|i| format!("line number {i}")).collect::<Vec<_>>().join("\n");
        let ch = BlankLineChunker::with_budget(40, 2);
        let r = ch.chunk(&doc(), ExtractedText { spans: vec![span(&block)] }).unwrap();
        assert!(r.len() > 1, "expected multiple windows, got {}", r.len());
        for c in &r {
            assert!(c.text.len() <= 40, "chunk exceeds budget: {} chars", c.text.len());
        }
    }

    // B2: consecutive windows overlap by `overlap_lines`.
    #[test]
    fn b2_windows_overlap_by_overlap_lines() {
        let block: String = (0..30).map(|i| format!("L{i}")).collect::<Vec<_>>().join("\n");
        let overlap = 2;
        let ch = BlankLineChunker::with_budget(12, overlap);
        let r = ch.chunk(&doc(), ExtractedText { spans: vec![span(&block)] }).unwrap();
        assert!(r.len() >= 2);
        for w in r.windows(2) {
            let prev: Vec<&str> = w[0].text.lines().collect();
            let next: Vec<&str> = w[1].text.lines().collect();
            assert_eq!(&prev[prev.len() - overlap..], &next[..overlap], "windows must overlap");
        }
    }

    // B3: a block within budget is left as a single chunk (no over-splitting).
    #[test]
    fn b3_within_budget_block_unchanged() {
        let block = "a\nb\nc\nd";
        let r = BlankLineChunker::with_budget(10_000, 5)
            .chunk(&doc(), ExtractedText { spans: vec![span(block)] }).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].text, block);
    }

    // B4: indices stay strictly sequential across normal + oversized blocks.
    #[test]
    fn b4_indices_sequential_across_mixed_blocks() {
        let big: String = (0..40).map(|i| format!("x{i}")).collect::<Vec<_>>().join("\n");
        let text = ExtractedText { spans: vec![span(&format!("small\n\n{big}\n\ntail"))] };
        let r = BlankLineChunker::with_budget(20, 2).chunk(&doc(), text).unwrap();
        let idxs: Vec<u32> = r.iter().map(|c| c.chunk_index).collect();
        for w in idxs.windows(2) { assert_eq!(w[1], w[0] + 1); }
        assert_eq!(idxs[0], 0);
        assert!(r.len() > 3, "oversized middle block should produce extra chunks");
    }

    // B5: a single line longer than the budget is emitted whole (cannot line-split;
    // must not panic or loop). Source: issue 026 edge case.
    #[test]
    fn b5_single_overlong_line_emitted_whole() {
        let line = "z".repeat(100);
        let r = BlankLineChunker::with_budget(10, 2)
            .chunk(&doc(), ExtractedText { spans: vec![span(&line)] }).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].text, line);
    }
}
