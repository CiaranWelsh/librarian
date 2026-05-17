//! Deterministic point IDs. UUID v5 over `(source_id, chunk_index)` so the
//! same input always maps to the same Qdrant point — the foundation of the
//! `upsert`/`replace` idempotency story (slice 008).

use librarian_domain::SourceId;

/// UUID v5 namespace for librarian point IDs (deterministic across runs).
const NAMESPACE: uuid::Uuid = uuid::Uuid::from_bytes([
    0xc0, 0x11, 0xec, 0x70, 0x71, 0xbe, 0x40, 0xa1, 0x95, 0x10, 0xd0, 0xea, 0xd0, 0x70, 0x6e, 0x73,
]);

pub fn point_id(source_id: &SourceId, chunk_index: u32) -> uuid::Uuid {
    uuid::Uuid::new_v5(
        &NAMESPACE,
        format!("{}#{}", source_id.0, chunk_index).as_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_id_is_deterministic() {
        let a = point_id(&SourceId("doc-a".into()), 0);
        let b = point_id(&SourceId("doc-a".into()), 0);
        assert_eq!(a, b);
    }

    #[test]
    fn point_id_distinguishes_index() {
        let a = point_id(&SourceId("doc-a".into()), 0);
        let b = point_id(&SourceId("doc-a".into()), 1);
        assert_ne!(a, b);
    }

    #[test]
    fn point_id_distinguishes_source() {
        let a = point_id(&SourceId("doc-a".into()), 0);
        let b = point_id(&SourceId("doc-b".into()), 0);
        assert_ne!(a, b);
    }
}
