//! `SnapshotOrchestrator` — wraps a `Snapshotter` with a `ManifestStore` and a
//! retention budget. Generic over the trait so the CLI binds the concrete
//! adapter at the composition root (hexagonal discipline per ADR-0004).

use librarian_domain::{ManifestStatus, ManifestStore, SnapshotId, Snapshotter, SourceId};

pub struct SnapshotOrchestrator<S, M> {
    pub snapshotter: S,
    pub manifest: M,
    pub retention: usize,
}

impl<S, M> SnapshotOrchestrator<S, M>
where
    S: Snapshotter,
    M: ManifestStore,
{
    /// Take a snapshot, record it in the manifest, and prune old snapshots
    /// down to `self.retention`.
    pub fn snapshot(&self) -> Result<SnapshotId, String> {
        let id = self.snapshotter.snapshot().map_err(|e| e.to_string())?;
        let snap_source = SourceId(format!("@snapshot:{}", id.0));
        let _ = self.manifest.record(
            &snap_source,
            "snapshot",
            ManifestStatus::Success,
            1,
            None,
            None,
        );
        if self.retention > 0 {
            let _ = self.snapshotter.prune(self.retention);
        }
        Ok(id)
    }

    pub fn restore(&self, id: &SnapshotId) -> Result<(), String> {
        self.snapshotter.restore(id).map_err(|e| e.to_string())
    }

    pub fn list(&self) -> Result<Vec<SnapshotId>, String> {
        self.snapshotter.list().map_err(|e| e.to_string())
    }
}
