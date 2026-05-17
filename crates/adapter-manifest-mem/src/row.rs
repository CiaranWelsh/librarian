use librarian_domain::{CacheKey, ManifestStatus, SourceId};

#[derive(Debug, Clone)]
pub struct Row {
    pub source_id: SourceId,
    pub stage: String,
    pub status: ManifestStatus,
    pub attempts: u32,
    pub error: Option<String>,
    pub output_ref: Option<CacheKey>,
}
