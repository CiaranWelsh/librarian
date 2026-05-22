//! EPUB / MOBI extractor backed by pandoc (+ calibre's `ebook-convert` for
//! MOBI). The pipeline is:
//!
//! ```text
//! .epub  →  pandoc -f epub -t gfm --wrap=none  →  cleaner  →  markdown
//! .mobi  →  ebook-convert tmp.epub             →  pandoc    →  cleaner  →  markdown
//! ```
//!
//! Marker isn't an option here — it's a vision-LM trained on rendered PDFs,
//! whereas EPUB is structured HTML underneath. Pandoc preserves that structure
//! losslessly; the cleaner strips the residual HTML scaffolding that pandoc's
//! GFM mode leaves behind (calibre's `<span class="kbd …">` for inline code is
//! the biggest single quality wart, and the cleaner recovers it).
//!
//! Configuration: `PANDOC_BIN` and `EBOOK_CONVERT_BIN` env vars override the
//! default `$PATH` lookup.

use librarian_domain::{
    AdapterIdentity, ConfigHash, Document, ExtractedText, Extractor, SpanKind, StageVersion,
    TextSpan,
};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cleaner::clean;
use crate::error::EbookExtractError;

pub struct EbookExtractor {
    pandoc_bin: PathBuf,
    ebook_convert_bin: PathBuf,
}

impl Default for EbookExtractor {
    fn default() -> Self {
        Self {
            pandoc_bin: std::env::var("PANDOC_BIN")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("pandoc")),
            ebook_convert_bin: std::env::var("EBOOK_CONVERT_BIN")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("ebook-convert")),
        }
    }
}

impl EbookExtractor {
    pub fn new() -> Self { Self::default() }
}

impl AdapterIdentity for EbookExtractor {
    fn name(&self) -> &str { "extractor-ebook" }
    fn version(&self) -> StageVersion { StageVersion("0.1.0-pandoc-clean".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash("pandoc-gfm+clean-v1".into()) }
}

impl Extractor for EbookExtractor {
    type Error = EbookExtractError;

    fn extract(&self, doc: &Document) -> Result<ExtractedText, Self::Error> {
        let ext = doc.path.extension().and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        let markdown = match ext.as_str() {
            "epub" => self.pandoc_to_md(&doc.path)?,
            "mobi" | "azw3" | "azw" => {
                let tmp = tempfile::tempdir().map_err(EbookExtractError::Io)?;
                let epub_path = tmp.path().join("converted.epub");
                self.calibre_to_epub(&doc.path, &epub_path)?;
                self.pandoc_to_md(&epub_path)?
            }
            other => return Err(EbookExtractError::UnsupportedExtension(other.into())),
        };

        let body = clean(&markdown);
        if body.trim().is_empty() {
            return Err(EbookExtractError::Empty);
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

impl EbookExtractor {
    fn pandoc_to_md(&self, epub: &Path) -> Result<String, EbookExtractError> {
        let out = Command::new(&self.pandoc_bin)
            .arg("-f").arg("epub")
            .arg("-t").arg("gfm")
            .arg("--wrap=none")
            .arg(epub)
            .output()
            .map_err(EbookExtractError::Io)?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let tail = stderr.lines().rev().take(8).collect::<Vec<_>>()
                .into_iter().rev().collect::<Vec<_>>().join(" | ");
            return Err(EbookExtractError::Pandoc(format!(
                "exit {:?}: {}", out.status.code(), tail
            )));
        }
        String::from_utf8(out.stdout)
            .map_err(|e| EbookExtractError::Pandoc(format!("non-UTF-8 pandoc output: {e}")))
    }

    fn calibre_to_epub(&self, src: &Path, dst: &Path) -> Result<(), EbookExtractError> {
        let out = Command::new(&self.ebook_convert_bin)
            .arg(src)
            .arg(dst)
            .output()
            .map_err(EbookExtractError::Io)?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let tail = stderr.lines().rev().take(8).collect::<Vec<_>>()
                .into_iter().rev().collect::<Vec<_>>().join(" | ");
            return Err(EbookExtractError::Calibre(format!(
                "exit {:?}: {}", out.status.code(), tail
            )));
        }
        Ok(())
    }
}
