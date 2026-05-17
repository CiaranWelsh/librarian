/// One result row from a Qdrant `search`.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub score: f32,
    pub source_id: String,
    pub chunk_index: u32,
    pub text: String,
    pub content_type: String,
}
