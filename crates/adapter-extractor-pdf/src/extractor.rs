//! PDF text extractor backed by `marker_single`.
//!
//! Marker is a vision-language model that converts PDFs to clean markdown —
//! handling multi-column layout, math (as LaTeX), tables, and figure captions
//! that bytes-level parsers fail on. We shell out to the CLI rather than
//! linking the Python library directly; one subprocess per document.
//!
//! Output: one `TextSpan` containing the full markdown body. The downstream
//! `BlankLineChunker` splits on blank lines, which markdown naturally has
//! between headings, paragraphs, list blocks, and tables.
//!
//! Configuration: the marker binary is found via (in order):
//! - the path given to `with_marker_bin`,
//! - the `MARKER_BIN` env var,
//! - `marker_single` in `$PATH`.

use librarian_domain::{
    AdapterIdentity, ConfigHash, Document, ExtractedText, Extractor, SpanKind, StageVersion,
    TextSpan,
};
use std::path::PathBuf;
use std::process::Command;

use crate::error::PdfExtractError;

pub struct PdfExtractor {
    marker_bin: PathBuf,
}

impl Default for PdfExtractor {
    fn default() -> Self {
        Self {
            marker_bin: std::env::var("MARKER_BIN")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("marker_single")),
        }
    }
}

impl PdfExtractor {
    pub fn new() -> Self { Self::default() }

    pub fn with_marker_bin(mut self, p: impl Into<PathBuf>) -> Self {
        self.marker_bin = p.into();
        self
    }
}

impl AdapterIdentity for PdfExtractor {
    fn name(&self) -> &str { "extractor-pdf" }
    /// Bumped from 0.1.0 (lopdf) — invalidates any prior cache entries on
    /// purpose so the corpus gets re-extracted with marker.
    fn version(&self) -> StageVersion { StageVersion("0.2.0-marker".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash("marker-default".into()) }
}

impl Extractor for PdfExtractor {
    type Error = PdfExtractError;

    fn extract(&self, doc: &Document) -> Result<ExtractedText, Self::Error> {
        let tmp = tempfile::tempdir().map_err(PdfExtractError::Io)?;

        let output = Command::new(&self.marker_bin)
            .arg(&doc.path)
            .arg("--output_dir")
            .arg(tmp.path())
            .arg("--disable_image_extraction")
            .output()
            .map_err(PdfExtractError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Tail of stderr — marker logs are noisy on the way down.
            let tail: String = stderr.lines().rev().take(8).collect::<Vec<_>>()
                .into_iter().rev().collect::<Vec<_>>().join(" | ");
            return Err(PdfExtractError::Marker(format!(
                "exit {:?}: {}", output.status.code(), tail
            )));
        }

        // marker writes <tmp>/<stem>/<stem>.md
        let stem = doc.path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| PdfExtractError::Marker("non-UTF-8 pdf filename".into()))?;
        let md_path = tmp.path().join(stem).join(format!("{stem}.md"));

        let body = std::fs::read_to_string(&md_path).map_err(PdfExtractError::Io)?;
        if body.trim().is_empty() {
            return Err(PdfExtractError::Marker("empty markdown output".into()));
        }

        let len = body.len();
        Ok(ExtractedText {
            spans: vec![TextSpan {
                kind: SpanKind::Paragraph,
                text: body,
                page: None,
                byte_range: 0..len,
            }],
        })
    }
}
