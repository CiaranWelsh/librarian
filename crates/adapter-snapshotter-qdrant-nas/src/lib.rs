//! Snapshot adapter: Qdrant native snapshot API → NAS push → optional retention.

mod error;
mod snapshotter;

pub use error::SnapshotError;
pub use snapshotter::QdrantNasSnapshotter;

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::Snapshotter;

    #[test]
    fn list_empty_when_nas_empty() {
        let dir = tempfile::tempdir().unwrap();
        let s = QdrantNasSnapshotter::new("http://localhost", "test-collection", dir.path()).unwrap();
        assert_eq!(s.list().unwrap(), vec![]);
    }

    #[test]
    fn list_filters_by_collection_prefix() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("col-a__snap1.snapshot"), b"x").unwrap();
        std::fs::write(dir.path().join("col-a__snap2.snapshot"), b"y").unwrap();
        std::fs::write(dir.path().join("col-b__snap3.snapshot"), b"z").unwrap();

        let s = QdrantNasSnapshotter::new("http://localhost", "col-a", dir.path()).unwrap();
        let mut ids: Vec<_> = s.list().unwrap().into_iter().map(|i| i.0).collect();
        ids.sort();
        assert_eq!(ids, vec!["col-a__snap1.snapshot", "col-a__snap2.snapshot"]);
    }

    #[test]
    fn prune_keeps_newest_n_files() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..5 {
            let p = dir.path().join(format!("c__s{i}.snapshot"));
            std::fs::write(&p, b"x").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(15));
            let now = std::time::SystemTime::now();
            filetime_set(&p, now).ok();
        }

        let s = QdrantNasSnapshotter::new("http://localhost", "c", dir.path()).unwrap();
        s.prune(3).unwrap();
        assert_eq!(s.list().unwrap().len(), 3);
    }

    fn filetime_set(p: &std::path::Path, t: std::time::SystemTime) -> std::io::Result<()> {
        let f = std::fs::File::open(p)?;
        f.set_modified(t)?;
        Ok(())
    }
}
