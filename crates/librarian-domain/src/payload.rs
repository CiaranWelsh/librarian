//! Typed per-content-type chunk payloads (F-M.3). The enum's variant carries
//! the metadata appropriate to its content type, enforced at compile time.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookMeta {
    pub title: String,
    pub author: Option<String>,
    pub chapter: Option<String>,
    pub section: Option<String>,
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperMeta {
    pub title: String,
    pub authors: Vec<String>,
    pub section: Option<String>,
    pub page_start: Option<u32>,
    pub page_end: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMeta {
    pub repo: Option<String>,
    pub commit: Option<String>,
    pub file_path: String,
    pub language: Option<String>,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FigureMeta {
    /// Title of the parent paper (or other carrier document).
    pub paper_title: Option<String>,
    /// Extracted caption text (e.g. "Figure 3: jet pT distribution"). Empty if
    /// the extractor couldn't pair an image with caption text.
    pub caption: String,
    pub page: Option<u32>,
    pub figure_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkPayload {
    Book(BookMeta),
    Paper(PaperMeta),
    Code(CodeMeta),
    Figure(FigureMeta),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_payload_serde_preserves_variant() {
        let cases = vec![
            ChunkPayload::Book(BookMeta {
                title: "t".into(), author: None, chapter: None, section: None, page: Some(3),
            }),
            ChunkPayload::Paper(PaperMeta {
                title: "t".into(), authors: vec!["a".into()], section: None,
                page_start: Some(1), page_end: Some(2),
            }),
            ChunkPayload::Code(CodeMeta {
                repo: None, commit: None, file_path: "x.rs".into(),
                language: Some("rust".into()), symbol: None,
            }),
        ];
        for c in cases {
            let json = serde_json::to_string(&c).unwrap();
            let back: ChunkPayload = serde_json::from_str(&json).unwrap();
            match (&c, &back) {
                (ChunkPayload::Book(_), ChunkPayload::Book(_))
                | (ChunkPayload::Paper(_), ChunkPayload::Paper(_))
                | (ChunkPayload::Code(_), ChunkPayload::Code(_)) => {}
                _ => panic!("variant changed across serde"),
            }
        }
    }
}
