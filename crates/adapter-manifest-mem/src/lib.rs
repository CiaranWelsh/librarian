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
