use crate::adapter_identity::AdapterIdentity;
use crate::ids::SnapshotId;

pub trait Snapshotter: AdapterIdentity {
    type Error: std::error::Error + Send + Sync + 'static;
    fn snapshot(&self) -> Result<SnapshotId, Self::Error>;
    fn restore(&self, id: &SnapshotId) -> Result<(), Self::Error>;
    fn list(&self) -> Result<Vec<SnapshotId>, Self::Error>;
    fn prune(&self, keep_last: usize) -> Result<(), Self::Error>;
}
