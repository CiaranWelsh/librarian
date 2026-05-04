# 005 — Qdrant indexer adapter

**Phase:** B · **Status:** Open · **Deps:** 004

## Goal

Replace the in-memory `Indexer` with a Qdrant-backed adapter using the Rust client.

## Acceptance criteria

- `adapter-indexer-qdrant` crate; `QdrantIndexer` implements `Indexer`.
- `upsert`, `delete_by_source_id`, `replace` all working against a real Qdrant.
- Deterministic point IDs from `(source_id, chunk_index)` (UUID v5).
- Payload schema: `ChunkPayload` serialised as Qdrant payload; indexed fields per F-M.4 (`content_type`, `work_id`).
- Collection-creation logic: idempotent. Reserves the `text` named vector slot (F-3.1); `code` and `figure` slots added in 016/017.
- `QdrantIndexer` is a concrete type satisfying `Indexer`; pipeline composition uses generics, not `dyn`.
- Walking skeleton swaps in `QdrantIndexer`; integration test ingests against a containerised Qdrant.

## Test plan

- Integration: `docker run qdrant/qdrant`, ingest fixture, query points by `source_id`, verify count and payload.
- Unit (no Qdrant): point-ID derivation deterministic across runs.
