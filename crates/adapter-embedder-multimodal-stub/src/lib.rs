//! Stub multimodal embedder for slice 017. Produces deterministic vectors
//! from image-bytes hashes so the pipeline shape can be exercised without a
//! real CLIP / vendor model. Replace with a real adapter (Voyage-multimodal,
//! OpenAI clip-style, or local model server) before shipping a corpus that
//! relies on figure semantics.

use librarian_domain::Vector;
use sha2::{Digest, Sha256};

pub struct MultimodalStubEmbedder {
    dim: usize,
    name: &'static str,
}

impl Default for MultimodalStubEmbedder {
    fn default() -> Self { Self { dim: 32, name: "embedder-multimodal-stub" } }
}

impl MultimodalStubEmbedder {
    pub fn new() -> Self { Self::default() }
    pub fn with_dim(dim: usize) -> Self { Self { dim, name: "embedder-multimodal-stub" } }
    pub fn name(&self) -> &str { self.name }
    pub fn dimension(&self) -> usize { self.dim }

    /// Embed image bytes (or any byte payload) into a vector of `self.dim` f32s.
    /// Deterministic: same input → same vector.
    pub fn embed_image(&self, bytes: &[u8]) -> Vector {
        let digest = Sha256::digest(bytes);
        (0..self.dim).map(|i| {
            let b = digest[i % digest.len()];
            (b as f32) / 127.5 - 1.0
        }).collect()
    }

    pub fn embed_batch(&self, items: &[&[u8]]) -> Vec<Vector> {
        items.iter().map(|b| self.embed_image(b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_same_bytes() {
        let e = MultimodalStubEmbedder::new();
        let a = e.embed_image(b"\x89PNG\r\n\x1a\n--fake--");
        let b = e.embed_image(b"\x89PNG\r\n\x1a\n--fake--");
        assert_eq!(a, b);
    }

    #[test]
    fn different_bytes_yield_different_vectors() {
        let e = MultimodalStubEmbedder::new();
        let a = e.embed_image(b"image-a");
        let b = e.embed_image(b"image-b");
        assert_ne!(a, b);
    }

    #[test]
    fn dimension_is_constant() {
        let e = MultimodalStubEmbedder::with_dim(64);
        assert_eq!(e.embed_image(b"x").len(), 64);
    }

    #[test]
    fn batch_returns_one_vector_per_input() {
        let e = MultimodalStubEmbedder::new();
        let v = e.embed_batch(&[b"a", b"bb", b"ccc"]);
        assert_eq!(v.len(), 3);
    }
}
