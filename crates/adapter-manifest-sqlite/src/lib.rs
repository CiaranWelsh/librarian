//! SQLite-backed `ManifestStore` adapter.

mod error;
mod manifest;
mod queries;
mod status;

pub use error::SqliteManifestError;
pub use manifest::SqliteManifest;
pub use queries::{distinct_ingested_sources, get_row, Row};

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::{CacheKey, ManifestStatus, ManifestStore, SourceId};
    use tempfile::tempdir;

    fn fresh() -> (tempfile::TempDir, SqliteManifest) {
        let d = tempdir().unwrap();
        let m = SqliteManifest::open(d.path().join("m.sqlite")).unwrap();
        (d, m)
    }

    fn sid(s: &str) -> SourceId { SourceId(s.into()) }

    #[test]
    fn migration_runs_clean() {
        let (_d, m) = fresh();
        assert_eq!(m.schema_version().unwrap(), 1);
    }

    #[test]
    fn record_upserts_on_source_id_stage_pair() {
        let (_d, m) = fresh();
        m.record(&sid("a"), "extract", ManifestStatus::Pending, 0, None, None).unwrap();
        m.record(&sid("a"), "extract", ManifestStatus::Success, 1, None, None).unwrap();
        let row = get_row(&m, &sid("a"), "extract").unwrap().unwrap();
        assert_eq!(row.status, ManifestStatus::Success);
        assert_eq!(row.attempts, 1);
        assert_eq!(m.list_by_status(ManifestStatus::Success).unwrap().len(), 1);
        assert_eq!(m.list_by_status(ManifestStatus::Pending).unwrap().len(), 0);
    }

    #[test]
    fn list_by_status_empty_when_no_matches() {
        let (_d, m) = fresh();
        assert_eq!(m.list_by_status(ManifestStatus::Failed).unwrap(), vec![]);
    }

    #[test]
    fn round_trip_preserves_each_status_variant() {
        let (_d, m) = fresh();
        let all = [
            ManifestStatus::Pending, ManifestStatus::Success, ManifestStatus::Cached,
            ManifestStatus::Failed, ManifestStatus::RecoveredViaFallback,
            ManifestStatus::Skipped, ManifestStatus::Removed,
        ];
        for (i, s) in all.iter().enumerate() {
            m.record(&sid(&format!("d{i}")), "extract", *s, 0, None, None).unwrap();
        }
        for s in all {
            let listed = m.list_by_status(s).unwrap();
            assert_eq!(listed.len(), 1, "status {s:?}");
        }
    }

    #[test]
    fn error_and_output_ref_persist_when_set_or_null() {
        let (_d, m) = fresh();
        m.record(&sid("a"), "extract", ManifestStatus::Failed, 2, Some("boom"), None).unwrap();
        let r = get_row(&m, &sid("a"), "extract").unwrap().unwrap();
        assert_eq!(r.error.as_deref(), Some("boom"));
        assert_eq!(r.output_ref, None);

        let key = CacheKey("k".repeat(64));
        m.record(&sid("b"), "embed", ManifestStatus::Cached, 0, None, Some(&key)).unwrap();
        let r = get_row(&m, &sid("b"), "embed").unwrap().unwrap();
        assert_eq!(r.error, None);
        assert_eq!(r.output_ref, Some(key));
    }

    #[test]
    fn distinct_stages_for_same_source_coexist() {
        let (_d, m) = fresh();
        m.record(&sid("a"), "extract", ManifestStatus::Success, 1, None, None).unwrap();
        m.record(&sid("a"), "embed", ManifestStatus::Failed, 1, Some("x"), None).unwrap();
        assert_eq!(m.list_by_status(ManifestStatus::Success).unwrap().len(), 1);
        assert_eq!(m.list_by_status(ManifestStatus::Failed).unwrap().len(), 1);
    }

    #[test]
    fn open_creates_parent_directory() {
        let d = tempdir().unwrap();
        let nested = d.path().join("a/b/m.sqlite");
        let _ = SqliteManifest::open(&nested).unwrap();
        assert!(nested.exists());
    }
}
