//! Qdrant payload construction. Each chunk becomes a key/value map carrying
//! the fields F-M.4 indexes (`content_type`, `source_id`, `work_id`) plus the
//! raw chunk text and the typed `ChunkPayload` as a serialized JSON string.

use librarian_domain::{Chunk, ChunkPayload};
use qdrant_client::qdrant::Value as QValue;
use qdrant_client::Payload;
use std::collections::HashMap;

pub(crate) fn build_payload(c: &Chunk) -> Payload {
    let mut map: HashMap<String, QValue> = HashMap::new();
    map.insert("source_id".into(), c.source_id.0.clone().into());
    map.insert("chunk_index".into(), (c.chunk_index as i64).into());
    map.insert("text".into(), c.text.clone().into());
    let content_type = match &c.payload {
        ChunkPayload::Book(_) => "book",
        ChunkPayload::Paper(_) => "paper",
        ChunkPayload::Code(_) => "code",
        ChunkPayload::Figure(_) => "figure",
    };
    map.insert("content_type".into(), content_type.into());
    if let Ok(s) = serde_json::to_string(&c.payload) {
        map.insert("payload_json".into(), s.into());
    }
    Payload::from(map)
}
