# 002 — Walking skeleton — in-memory end-to-end

**Phase:** A · **Status:** Open · **Deps:** 001

## Goal

A runnable ingest of one text fixture, end-to-end through the pipeline, using only in-memory adapters. Proves the wiring; touches no external system.

## Acceptance criteria

- In-memory adapter crates: `adapter-cache-mem`, `adapter-manifest-mem`, `adapter-indexer-mem`.
- Trivial `Extractor`: reads UTF-8 file, returns text + a single `TextSpan { kind: Paragraph }`.
- Trivial `Chunker`: splits on blank lines, emits `Chunk`s with deterministic `chunk_index`.
- Stub `Embedder`: SHA-256 of chunk text → fixed-length `f32` array (deterministic, no network).
- `Pipeline::run(document)` orchestrates `extract → chunk → embed → index`. Serial. No fault catching yet.
- Composition root binary that ingests one fixture path passed on the CLI. Pipeline is generic over its stage types (`Pipeline<E: Extractor, Ch: Chunker, Em: Embedder, Ix: Indexer, C: Cache, M: ManifestStore>`); no `Box<dyn Trait>`.
- Manifest schema concretised: `(source_id, stage, status: ManifestStatus, attempts: u32, error: Option<String>, output_ref: Option<CacheKey>, updated_at)`.

## Test plan

- Integration test: ingest `tests/fixtures/sample.txt` (3 paragraphs) → in-memory indexer holds 3 points with deterministic IDs.
- Stub-based unit tests on `Pipeline::run` — proves runner is testable without any real adapter.
