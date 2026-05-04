# 008 — Update and remove

**Phase:** C · **Status:** Open · **Deps:** 005, 007

## Goal

Atomic update of a changed Document (no orphans) and explicit `remove` operation. Backs **F-1.8, F-1.9**.

## Acceptance criteria

- Runner: detects `source_hash` change → calls `Indexer::replace(source_id, new_chunks)` instead of `upsert`.
- `QdrantIndexer::replace` delete-then-upsert is atomic enough that a partial failure leaves the collection consistent (either old or new chunks, never a mix).
- `Pipeline::remove(source_id)` calls `Indexer::delete_by_source_id` and writes `ManifestStatus::Removed`.
- Removing a source file from disk SHALL NOT trigger removal — only explicit `remove` does (per F-1.9).

## Test plan

- Update test: ingest a fixture of 5 chunks; modify so it produces 3 chunks; re-ingest; query by `source_id` returns exactly 3 chunks (no orphans).
- Remove test: ingest, then `remove(source_id)`; query returns 0 chunks; manifest shows `Removed`.
