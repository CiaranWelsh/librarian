//! librarian-domain — pure types and ports. No I/O, no async, no implementations.
//! Per ADR-0004 (hexagonal): adapter crates implement these traits from outside.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─── Newtype IDs ──────────────────────────────────────────────────────────────

macro_rules! string_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub String);
    };
}
string_newtype!(SourceId);
string_newtype!(WorkId);
string_newtype!(SnapshotId);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceHash(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConfigHash(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StageVersion(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId(pub String);

// ─── Content types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentType {
    Book,
    Paper,
    Code,
}

// ─── Documents and extracted text ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Document {
    pub source_id: SourceId,
    pub source_hash: SourceHash,
    pub content_type: ContentType,
    pub path: std::path::PathBuf,
    pub work_id: Option<WorkId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanKind {
    Heading,
    Paragraph,
    Code,
    Caption,
    ListItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSpan {
    pub kind: SpanKind,
    pub text: String,
    pub page: Option<u32>,
    pub byte_range: std::ops::Range<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedText {
    pub spans: Vec<TextSpan>,
}

// ─── Typed chunk payloads (F-M.3) ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookMeta {
    pub title: String,
    pub author: Option<String>,
    pub chapter: Option<String>,
    pub section: Option<String>,
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperMeta {
    pub title: String,
    pub authors: Vec<String>,
    pub section: Option<String>,
    pub page_start: Option<u32>,
    pub page_end: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMeta {
    pub repo: Option<String>,
    pub commit: Option<String>,
    pub file_path: String,
    pub language: Option<String>,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkPayload {
    Book(BookMeta),
    Paper(PaperMeta),
    Code(CodeMeta),
}

// ─── Provenance (F-M.6) ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceLink {
    pub stage_name: String,
    pub stage_version: StageVersion,
    pub config_hash: ConfigHash,
    pub cache_key: CacheKey,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Provenance(pub Vec<ProvenanceLink>);

// ─── Chunk ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub chunk_id: ChunkId,
    pub source_id: SourceId,
    pub chunk_index: u32,
    pub text: String,
    pub payload: ChunkPayload,
    pub provenance: Provenance,
}

pub type Vector = Vec<f32>;

// ─── Work (metadata-only grouping) ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Work {
    pub work_id: WorkId,
    pub title: String,
    pub members: Vec<SourceId>,
}

// ─── Manifest status (F-5.4) ──────────────────────────────────────────────────

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

// ─── Cache-key derivation (ADR-0001 §4) ───────────────────────────────────────

pub mod cache_key {
    use super::*;
    use sha2::{Digest, Sha256};

    /// `sha256(source_hash ‖ 0x1F ‖ stage_name ‖ 0x1F ‖ stage_version ‖ 0x1F ‖ config_hash)`
    /// 0x1F (Unit Separator) prevents adversarial concatenation collisions.
    pub fn derive(
        source_hash: &SourceHash,
        stage_name: &str,
        stage_version: &StageVersion,
        config_hash: &ConfigHash,
    ) -> CacheKey {
        let sep = [0x1Fu8];
        let mut h = Sha256::new();
        h.update(source_hash.0.as_bytes());
        h.update(sep);
        h.update(stage_name.as_bytes());
        h.update(sep);
        h.update(stage_version.0.as_bytes());
        h.update(sep);
        h.update(config_hash.0.as_bytes());
        CacheKey(hex::encode(h.finalize()))
    }
}

// ─── Adapter identity (supertrait) ────────────────────────────────────────────

/// Every stage adapter exposes its identity so the runner — not the adapter —
/// derives `CacheKey`. Adapters never see CacheKey.
pub trait AdapterIdentity {
    fn name(&self) -> &str;
    fn version(&self) -> StageVersion;
    fn config_hash(&self) -> ConfigHash;
}

// ─── Stage ports ──────────────────────────────────────────────────────────────

pub trait Extractor: AdapterIdentity {
    type Error: std::error::Error + Send + Sync + 'static;
    fn extract(&self, doc: &Document) -> Result<ExtractedText, Self::Error>;
}

pub trait Chunker: AdapterIdentity {
    type Error: std::error::Error + Send + Sync + 'static;
    fn chunk(&self, doc: &Document, text: ExtractedText) -> Result<Vec<Chunk>, Self::Error>;
}

/// Fixed-shape error so `FallbackEmbedder` (slice 011) can pattern-match.
#[derive(Debug, Error)]
pub enum EmbedderError {
    #[error("recoverable: {0}")]
    Recoverable(String),
    #[error("terminal: {0}")]
    Terminal(String),
}

pub trait Embedder: AdapterIdentity {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError>;
    fn dimension(&self) -> usize;
    /// If the most recent `embed` was wrapped by a fallback combinator, the
    /// runner reads this to decide between `Success` / `RecoveredViaFallback`
    /// / multi-error `Failed`. Default: never a fallback.
    fn last_event(&self) -> Option<FallbackEvent> { None }
}

/// Communicates fallback-combinator outcomes to the runner without changing
/// the trait's success/error shape (slice 011).
#[derive(Debug, Clone)]
pub struct FallbackEvent {
    pub primary_error: String,
    /// `true` if the fallback succeeded; `false` if both primary and fallback failed.
    pub recovered: bool,
    /// Populated when the fallback also failed terminally.
    pub fallback_error: Option<String>,
}

pub trait Indexer: AdapterIdentity {
    type Error: std::error::Error + Send + Sync + 'static;
    fn upsert(&self, chunks: &[Chunk], vectors: &[Vector]) -> Result<(), Self::Error>;
    fn replace(
        &self,
        source_id: &SourceId,
        chunks: &[Chunk],
        vectors: &[Vector],
    ) -> Result<(), Self::Error>;
    fn delete_by_source_id(&self, source_id: &SourceId) -> Result<(), Self::Error>;
}

// ─── Storage ports (declarations; full contracts in pass 2) ───────────────────

pub trait Cache {
    type Error: std::error::Error + Send + Sync + 'static;
    fn get(&self, key: &CacheKey) -> Result<Option<Vec<u8>>, Self::Error>;
    fn put(&self, key: &CacheKey, value: &[u8]) -> Result<(), Self::Error>;
}

impl<T: Cache + ?Sized> Cache for &T {
    type Error = T::Error;
    fn get(&self, key: &CacheKey) -> Result<Option<Vec<u8>>, Self::Error> { (**self).get(key) }
    fn put(&self, key: &CacheKey, value: &[u8]) -> Result<(), Self::Error> { (**self).put(key, value) }
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

pub trait Snapshotter: AdapterIdentity {
    type Error: std::error::Error + Send + Sync + 'static;
    fn snapshot(&self) -> Result<SnapshotId, Self::Error>;
    fn restore(&self, id: &SnapshotId) -> Result<(), Self::Error>;
    fn list(&self) -> Result<Vec<SnapshotId>, Self::Error>;
    fn prune(&self, keep_last: usize) -> Result<(), Self::Error>;
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

    #[test]
    fn chunk_payload_serde_preserves_variant() {
        let cases = vec![
            ChunkPayload::Book(BookMeta {
                title: "t".into(), author: None, chapter: None, section: None, page: Some(3),
            }),
            ChunkPayload::Paper(PaperMeta {
                title: "t".into(), authors: vec!["a".into()], section: None,
                page_start: Some(1), page_end: Some(2),
            }),
            ChunkPayload::Code(CodeMeta {
                repo: None, commit: None, file_path: "x.rs".into(),
                language: Some("rust".into()), symbol: None,
            }),
        ];
        for c in cases {
            let json = serde_json::to_string(&c).unwrap();
            let back: ChunkPayload = serde_json::from_str(&json).unwrap();
            // Round-trip is structural; check discriminant via match.
            match (&c, &back) {
                (ChunkPayload::Book(_), ChunkPayload::Book(_))
                | (ChunkPayload::Paper(_), ChunkPayload::Paper(_))
                | (ChunkPayload::Code(_), ChunkPayload::Code(_)) => {}
                _ => panic!("variant changed across serde"),
            }
        }
    }

    #[test]
    fn newtype_equality_is_value_based() {
        assert_eq!(SourceId("a".into()), SourceId("a".into()));
        assert_ne!(SourceId("a".into()), SourceId("b".into()));
    }
}
