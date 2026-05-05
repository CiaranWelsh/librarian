# 020 — Runner-level dual-vector wiring

**Phase:** I (post-v1) · **Status:** Open · **Deps:** 019

## Goal

Make F-3.2 (per-modality named vectors) real end-to-end through the CLI. Slice 016 delivered the indexer capability (`QdrantIndexer::open_with_slots` + `upsert_named`), but `BatchRunner` only ever calls single-vector `replace`. So the CLI cannot populate the `code` named vector for code chunks today — the slot exists but stays empty.

## Acceptance criteria

- `BatchRunner` gains an optional `code_embedder: Option<CE>` (or equivalent — possibly via a new `DualEmbedderBatchRunner` to avoid breaking ~10 existing call sites). Generic, no `dyn`.
- For chunks with `ChunkPayload::Code(_)`, the runner embeds the chunk text twice: once with the configured text embedder for the `text` slot, once with `code_embedder` for the `code` slot.
- The runner calls `Indexer::upsert_named` (new trait method on `Indexer`, mirroring `upsert`) when ≥1 named vector is in play.
- `MemIndexer` and `QdrantIndexer` both implement `upsert_named`.
- CLI's TOML config gains an optional `[code_embedder]` section (same shape as `[embedder]`); when present, the CLI's composition root constructs both embedders and the dual-vector runner.
- Existing single-vector tests (the ~50 that use `BatchRunner` today) keep working unchanged — either via default `code_embedder: None` or via the separate-runner approach.

## Test plan

- Stub-based unit test on the dual-vector runner: inject a counting code embedder, confirm it's called exactly when `ContentType::Code`, never otherwise.
- Integration test against real Qdrant: ingest one code file via the CLI with both `[embedder] kind = "stub"` and `[code_embedder] kind = "stub"` (variant dim); assert via `count_by_source` that the chunk lands and via Qdrant directly that both `text` and `code` vector slots are populated on the point.

## Notes

The existing `Embedder` trait is `&[&str] -> Vec<Vector>`. The text and code embedders share that shape, so no trait change is needed beyond adding `upsert_named` on `Indexer`.
