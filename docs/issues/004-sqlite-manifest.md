# 004 — SQLite manifest adapter

**Phase:** B · **Status:** Open · **Deps:** 003

## Goal

Replace the in-memory `ManifestStore` with a SQLite-backed adapter (one file per collection).

## Acceptance criteria

- `adapter-manifest-sqlite` crate; `SqliteManifest` implements `ManifestStore`.
- Schema: one row per `(source_id, stage)`; columns: `status` (full `ManifestStatus` enum: `Pending | Success | Cached | Failed | RecoveredViaFallback | Skipped | Removed`), `attempts: INTEGER`, `error: TEXT NULL`, `output_ref: TEXT NULL` (CacheKey hex), `updated_at`. Migrations versioned in code.
- Indexes on `(source_id, stage)` and `status` for the F-5.4 query patterns.
- `SqliteManifest` is a concrete type used via the `ManifestStore` trait through generics; no `dyn`.
- Configurable file path. Connection-pool-friendly (or single connection for v1 — serial ingest is fine).
- Walking skeleton swaps in `SqliteManifest`; manifest persists across process restarts.

## Test plan

- Unit: insert/get/list-by-status round-trip; concurrent writers serialise correctly (one Mutex or DB-level lock).
- Integration: ingest, kill process, restart, observe `list_by_status(Success)` returns the prior run's rows.
