//! HTML extractor backed by pandoc:
//!
//! ```text
//! .html / .htm  →  pandoc -f html -t gfm --wrap=none  →  cleaner  →  markdown
//! ```
//!
//! Like the ebook extractor, marker isn't applicable — HTML is already
//! structured text. Pandoc preserves the structure; the shared `markdown-cleaner`
//! strips residual `<div>`/`<span>` scaffolding pandoc's GFM mode leaves behind.
//!
//! Configuration: `PANDOC_BIN` overrides the default `$PATH` lookup.

use librarian_domain::{
    AdapterIdentity, ConfigHash, Document, ExtractedText, Extractor, SpanKind, StageVersion,
    TextSpan,
};
use std::path::PathBuf;
use std::process::Command;

use crate::error::HtmlExtractError;

pub struct HtmlExtractor {
    pandoc_bin: PathBuf,
}

impl Default for HtmlExtractor {
    fn default() -> Self {
        Self {
            pandoc_bin: std::env::var("PANDOC_BIN")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("pandoc")),
        }
    }
}

impl HtmlExtractor {
    pub fn new() -> Self { Self::default() }
}

impl AdapterIdentity for HtmlExtractor {
    fn name(&self) -> &str { "extractor-html" }
    fn version(&self) -> StageVersion { StageVersion("0.1.0-pandoc-gfm".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash("pandoc-gfm+clean-v1".into()) }
}

impl Extractor for HtmlExtractor {
    type Error = HtmlExtractError;

    fn extract(&self, doc: &Document) -> Result<ExtractedText, Self::Error> {
        let ext = doc.path.extension().and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            "html" | "htm" => {}
            other => return Err(HtmlExtractError::UnsupportedExtension(other.into())),
        }

        let out = Command::new(&self.pandoc_bin)
            .arg("-f").arg("html")
            .arg("-t").arg("gfm")
            .arg("--wrap=none")
            .arg(&doc.path)
            .output()
            .map_err(HtmlExtractError::Io)?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let tail = stderr.lines().rev().take(8).collect::<Vec<_>>()
                .into_iter().rev().collect::<Vec<_>>().join(" | ");
            return Err(HtmlExtractError::Pandoc(format!("exit {:?}: {}", out.status.code(), tail)));
        }
        let markdown = String::from_utf8(out.stdout)
            .map_err(|e| HtmlExtractError::Pandoc(format!("non-UTF-8 pandoc output: {e}")))?;

        let body = markdown_cleaner::clean(&markdown);
        if body.trim().is_empty() {
            return Err(HtmlExtractError::Empty);
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
