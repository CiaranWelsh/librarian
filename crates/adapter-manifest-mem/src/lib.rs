//! In-memory `ManifestStore` — used by tests and the walking skeleton.

mod error;
mod manifest;
mod row;

pub use error::MemManifestError;
pub use manifest::MemManifest;
pub use row::Row;

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::{ManifestStatus, ManifestStore, SourceId};

    fn sid(s: &str) -> SourceId { SourceId(s.into()) }

    #[test]
    fn record_then_list_filters_by_status() {
        let m = MemManifest::new();
        m.record(&sid("a"), "extract", ManifestStatus::Success, 1, None, None).unwrap();
        m.record(&sid("b"), "extract", ManifestStatus::Failed, 1, Some("boom"), None).unwrap();
        m.record(&sid("c"), "extract", ManifestStatus::Success, 1, None, None).unwrap();

        let succ = m.list_by_status(ManifestStatus::Success).unwrap();
        assert_eq!(succ.len(), 2);
        let fail = m.list_by_status(ManifestStatus::Failed).unwrap();
        assert_eq!(fail, vec![(sid("b"), "extract".into())]);
    }

    #[test]
    fn errors_persisted_on_failed_rows() {
        let m = MemManifest::new();
        m.record(&sid("b"), "embed", ManifestStatus::Failed, 2, Some("oops"), None).unwrap();
        let rows = m.rows();
        assert_eq!(rows[0].error.as_deref(), Some("oops"));
        assert_eq!(rows[0].attempts, 2);
    }
}
