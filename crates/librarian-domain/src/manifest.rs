//! Manifest: per-document, per-stage record of pipeline outcomes (F-5.4).
//! `ManifestStore` is the outbound port; `ManifestStatus` is the enum the
//! runner writes.

use serde::{Deserialize, Serialize};

use crate::ids::{CacheKey, SourceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManifestStatus {
    Pending,
    Success,
    Cached,
    Failed,
    RecoveredViaFallback,
    Skipped,
    Removed,
}

pub trait ManifestStore {
    type Error: std::error::Error + Send + Sync + 'static;
    fn record(
        &self,
        source_id: &SourceId,
        stage: &str,
        status: ManifestStatus,
        attempts: u32,
        error: Option<&str>,
        output_ref: Option<&CacheKey>,
    ) -> Result<(), Self::Error>;
    fn list_by_status(
        &self,
        status: ManifestStatus,
    ) -> Result<Vec<(SourceId, String)>, Self::Error>;
}

impl<T: ManifestStore + ?Sized> ManifestStore for &T {
    type Error = T::Error;
    fn record(
        &self, source_id: &SourceId, stage: &str, status: ManifestStatus,
        attempts: u32, error: Option<&str>, output_ref: Option<&CacheKey>,
    ) -> Result<(), Self::Error> {
        (**self).record(source_id, stage, status, attempts, error, output_ref)
    }
    fn list_by_status(&self, status: ManifestStatus) -> Result<Vec<(SourceId, String)>, Self::Error> {
        (**self).list_by_status(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_status_serde_roundtrip() {
        for s in [
            ManifestStatus::Pending,
            ManifestStatus::Success,
            ManifestStatus::Cached,
            ManifestStatus::Failed,
            ManifestStatus::RecoveredViaFallback,
            ManifestStatus::Skipped,
            ManifestStatus::Removed,
        ] {
            let json = serde_json::to_string(&s).unwrap();
            let back: ManifestStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back);
        }
    }
}
