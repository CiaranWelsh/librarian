#[derive(Debug, thiserror::Error)]
#[error("mem-manifest poisoned")]
pub struct MemManifestError;
