use librarian_domain::{CacheKey, ManifestStatus, ManifestStore, SourceId};
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Row {
    pub source_id: SourceId,
    pub stage: String,
    pub status: ManifestStatus,
    pub attempts: u32,
    pub error: Option<String>,
    pub output_ref: Option<CacheKey>,
}

#[derive(Default)]
pub struct MemManifest {
    rows: Mutex<Vec<Row>>,
}

impl MemManifest {
    pub fn new() -> Self { Self::default() }
    pub fn rows(&self) -> Vec<Row> { self.rows.lock().unwrap().clone() }
}

#[derive(Debug, thiserror::Error)]
#[error("mem-manifest poisoned")]
pub struct MemManifestError;

impl ManifestStore for MemManifest {
    type Error = MemManifestError;

    fn record(
        &self,
        source_id: &SourceId,
        stage: &str,
        status: ManifestStatus,
        attempts: u32,
        error: Option<&str>,
        output_ref: Option<&CacheKey>,
    ) -> Result<(), Self::Error> {
        let mut g = self.rows.lock().map_err(|_| MemManifestError)?;
        g.push(Row {
            source_id: source_id.clone(),
            stage: stage.to_string(),
            status,
            attempts,
            error: error.map(|s| s.to_string()),
            output_ref: output_ref.cloned(),
        });
        Ok(())
    }

    fn list_by_status(
        &self,
        status: ManifestStatus,
    ) -> Result<Vec<(SourceId, String)>, Self::Error> {
        let g = self.rows.lock().map_err(|_| MemManifestError)?;
        Ok(g.iter()
            .filter(|r| r.status == status)
            .map(|r| (r.source_id.clone(), r.stage.clone()))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
