//! librarian-domain — pure types and ports. No I/O, no async, no implementations.
//!
//! Per ADR-0004 (hexagonal): this crate is the dependency-free leaf. Adapter
//! crates implement these traits from outside; the runner orchestrates them.
//!
//! This module file is intentionally minimal — it declares submodules and
//! re-exports their items at the crate root so callers continue to write
//! `use librarian_domain::Chunk` rather than `use librarian_domain::chunk::Chunk`.
//! Each submodule below holds one cohesive piece of the domain.

mod adapter_identity;
mod cache;
mod chunk;
mod chunker;
mod content_type;
mod document;
mod embedder;
mod extractor;
mod ids;
mod indexer;
mod manifest;
mod payload;
mod provenance;
mod quality;
mod snapshotter;
mod work;

pub use adapter_identity::AdapterIdentity;
pub use cache::{cache_key, Cache};
pub use chunk::{Chunk, Vector};
pub use chunker::Chunker;
pub use content_type::ContentType;
pub use document::{Document, ExtractedText, SpanKind, TextSpan};
pub use embedder::{Embedder, EmbedderError, FallbackEvent};
pub use extractor::Extractor;
pub use ids::{
    CacheKey, ChunkId, ConfigHash, SnapshotId, SourceHash, SourceId, StageVersion, WorkId,
};
pub use indexer::Indexer;
pub use manifest::{ManifestStatus, ManifestStore};
pub use payload::{BookMeta, ChunkPayload, CodeMeta, FigureMeta, PaperMeta};
pub use provenance::{Provenance, ProvenanceLink};
pub use quality::{
    classify_name, classify_section, garble_signal, garble_text, GarbleConfig, GarbleSignal,
    QualityConfig, SectionConfig, SectionDecision,
};
pub use snapshotter::Snapshotter;
pub use work::Work;
