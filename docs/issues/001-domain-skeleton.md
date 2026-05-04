# 001 — Domain skeleton

**Phase:** A · **Status:** Open · **Deps:** none

## Goal

`librarian-domain` crate with all v1 types and outbound port traits. No I/O, no infrastructure dependencies.

## Acceptance criteria

- Workspace `Cargo.toml` and `crates/librarian-domain/` present.
- Domain dependencies limited to `thiserror`, `serde`, `sha2`, `hex`, `chrono`. No qdrant / reqwest / tokio.
- Types: `Document`, `Chunk`, `ChunkPayload` (enum: `Book`/`Paper`/`Code`), `Provenance`, `Work`, newtype IDs (`SourceId`, `SourceHash`, `WorkId`, `StageVersion`, `ConfigHash`, `CacheKey`, `ChunkId`, `SnapshotId`).
- Traits: `Extractor`, `Chunker`, `Embedder`, `Indexer`, `Cache`, `ManifestStore`, `Snapshotter`. `AdapterIdentity` supertrait exposing `name() -> &str`, `version() -> StageVersion`, `config_hash() -> ConfigHash` — runner uses these to derive cache keys; adapters never derive their own.
- `ManifestStatus` enum: `Pending | Success | Cached | Failed | RecoveredViaFallback | Skipped | Removed`.
- `cache_key::derive` function (per ADR-0001 §4 formula).
- All types are sync. No `Box<dyn Trait>` anywhere — generics only.

## Test plan

- `cargo build` clean.
- ≥6 unit tests on `cache_key::derive`: deterministic, distinguishes each input axis, separator-safe, hex-encoded SHA-256.
