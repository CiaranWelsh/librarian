//! Integration: the sqlite file persists across opens. Slice-004 AC.

use adapter_manifest_sqlite::{get_row, SqliteManifest};
use librarian_domain::{ManifestStatus, ManifestStore, SourceId};
use tempfile::tempdir;

#[test]
fn rows_survive_reopen() {
    let d = tempdir().unwrap();
    let path = d.path().join("m.sqlite");

    {
        let m = SqliteManifest::open(&path).unwrap();
        m.record(
            &SourceId("alpha".into()),
            "extract",
            ManifestStatus::Success,
            1,
            None,
            None,
        )
        .unwrap();
        m.record(
            &SourceId("beta".into()),
            "embed",
            ManifestStatus::Failed,
            3,
            Some("net"),
            None,
        )
        .unwrap();
    }

    let m2 = SqliteManifest::open(&path).unwrap();
    let succ = m2.list_by_status(ManifestStatus::Success).unwrap();
    assert_eq!(succ, vec![(SourceId("alpha".into()), "extract".into())]);

    let beta = get_row(&m2, &SourceId("beta".into()), "embed")
        .unwrap()
        .unwrap();
    assert_eq!(beta.status, ManifestStatus::Failed);
    assert_eq!(beta.attempts, 3);
    assert_eq!(beta.error.as_deref(), Some("net"));
}

#[test]
fn idempotent_reopen_does_not_destroy_rows() {
    let d = tempdir().unwrap();
    let path = d.path().join("m.sqlite");

    let m1 = SqliteManifest::open(&path).unwrap();
    m1.record(
        &SourceId("alpha".into()),
        "extract",
        ManifestStatus::Success,
        1,
        None,
        None,
    )
    .unwrap();
    drop(m1);

    // Opening again runs CREATE TABLE IF NOT EXISTS — must not wipe data.
    let m2 = SqliteManifest::open(&path).unwrap();
    assert_eq!(m2.list_by_status(ManifestStatus::Success).unwrap().len(), 1);
    assert_eq!(m2.schema_version().unwrap(), 1);
}
