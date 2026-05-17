//! Text-PDF extractor backed by `lopdf`. v1: per-page text → one paragraph
//! `TextSpan` per non-empty paragraph block, with the page number set.
//! Heading detection is intentionally deferred — the chunker can split on
//! blank lines for now.

use librarian_domain::{
    AdapterIdentity, BookMeta, ChunkPayload, ConfigHash, Document, ExtractedText, Extractor,
    PaperMeta, SpanKind, StageVersion, TextSpan,
};
use lopdf::Document as PdfDoc;

use crate::error::PdfExtractError;

#[derive(Default)]
pub struct PdfExtractor;

impl PdfExtractor {
    pub fn new() -> Self { Self }

    /// Build a `ChunkPayload` populated with the document's metadata for the
    /// chunker / indexer. Title is taken from the PDF's Info dictionary if
    /// present, falling back to the file stem.
    pub fn payload_for(&self, doc: &Document) -> Result<ChunkPayload, PdfExtractError> {
        let pdf = PdfDoc::load(&doc.path).map_err(PdfExtractError::Pdf)?;
        let title = pdf_info_string(&pdf, b"Title").unwrap_or_else(|| {
            doc.path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("(untitled)")
                .to_string()
        });
        let author = pdf_info_string(&pdf, b"Author");
        Ok(match doc.content_type {
            librarian_domain::ContentType::Book => ChunkPayload::Book(BookMeta {
                title, author, chapter: None, section: None, page: None,
            }),
            librarian_domain::ContentType::Paper => ChunkPayload::Paper(PaperMeta {
                title,
                authors: author.into_iter().collect(),
                section: None,
                page_start: None,
                page_end: None,
            }),
            librarian_domain::ContentType::Code => ChunkPayload::Book(BookMeta {
                title, author, chapter: None, section: None, page: None,
            }),
        })
    }
}

impl AdapterIdentity for PdfExtractor {
    fn name(&self) -> &str { "extractor-pdf" }
    fn version(&self) -> StageVersion { StageVersion("0.1.0".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash("default".into()) }
}

impl Extractor for PdfExtractor {
    type Error = PdfExtractError;

    fn extract(&self, doc: &Document) -> Result<ExtractedText, Self::Error> {
        let pdf = PdfDoc::load(&doc.path).map_err(PdfExtractError::Pdf)?;
        let pages = pdf.get_pages();

        let mut spans: Vec<TextSpan> = Vec::new();
        let mut cursor = 0usize;
        for (page_no, _) in pages.iter() {
            let page_text = pdf
                .extract_text(&[*page_no])
                .map_err(PdfExtractError::Pdf)?;
            for para in page_text.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
                let len = para.len();
                spans.push(TextSpan {
                    kind: SpanKind::Paragraph,
                    text: para.to_string(),
                    page: Some(*page_no),
                    byte_range: cursor..cursor + len,
                });
                cursor += len;
            }
        }
        Ok(ExtractedText { spans })
    }
}

fn pdf_info_string(pdf: &PdfDoc, key: &[u8]) -> Option<String> {
    let info_id = pdf.trailer.get(b"Info").ok()?.as_reference().ok()?;
    let dict = pdf.get_dictionary(info_id).ok()?;
    let v = dict.get(key).ok()?;
    let bytes = v.as_str().ok()?;
    Some(String::from_utf8_lossy(bytes).to_string())
}
