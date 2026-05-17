//! Input document and the structured text produced by extraction.

use serde::{Deserialize, Serialize};

use crate::content_type::ContentType;
use crate::ids::{SourceHash, SourceId, WorkId};

#[derive(Debug, Clone)]
pub struct Document {
    pub source_id: SourceId,
    pub source_hash: SourceHash,
    pub content_type: ContentType,
    pub path: std::path::PathBuf,
    pub work_id: Option<WorkId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanKind {
    Heading,
    Paragraph,
    Code,
    Caption,
    ListItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSpan {
    pub kind: SpanKind,
    pub text: String,
    pub page: Option<u32>,
    pub byte_range: std::ops::Range<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedText {
    pub spans: Vec<TextSpan>,
}
