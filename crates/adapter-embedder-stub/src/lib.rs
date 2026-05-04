use librarian_domain::{
    AdapterIdentity, ConfigHash, Embedder, EmbedderError, StageVersion, Vector,
};
use sha2::{Digest, Sha256};

/// Deterministic stub: SHA-256 of the chunk text → 32 f32s in [-1, 1].
pub struct StubEmbedder {
    dim: usize,
}

impl Default for StubEmbedder {
    fn default() -> Self { Self { dim: 32 } }
}

impl StubEmbedder {
    pub fn new() -> Self { Self::default() }
}

impl AdapterIdentity for StubEmbedder {
    fn name(&self) -> &str { "embedder-stub" }
    fn version(&self) -> StageVersion { StageVersion("0.1.0".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash(format!("dim={}", self.dim)) }
}

impl Embedder for StubEmbedder {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        if texts.is_empty() {
            return Err(EmbedderError::Terminal("empty batch".into()));
        }
        Ok(texts.iter().map(|t| hash_to_vec(t, self.dim)).collect())
    }

    fn dimension(&self) -> usize { self.dim }
}

fn hash_to_vec(text: &str, dim: usize) -> Vector {
    let digest = Sha256::digest(text.as_bytes());
    (0..dim)
        .map(|i| {
            let b = digest[i % digest.len()];
            (b as f32) / 127.5 - 1.0
        })
        .collect()
}
