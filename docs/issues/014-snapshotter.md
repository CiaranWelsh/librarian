# 014 — Snapshotter

**Phase:** F · **Status:** Open · **Deps:** 013

## Goal

Implements **F-7.3, QA-O3**: snapshot/restore via Qdrant's native API + NAS push, with rolling retention. CLI commands `snapshot` / `restore` go from stubs to working.

## Acceptance criteria

- `adapter-snapshotter-qdrant-nas` crate; `QdrantNasSnapshotter` implements `Snapshotter`.
- `snapshot()` triggers Qdrant's snapshot API, pushes the resulting file to NAS via HTTPS or SCP (configurable). No NFS mount.
- `prune(keep_last)` deletes old snapshots beyond the retention window.
- `restore(id)` downloads the snapshot file to Qdrant and triggers restore.
- Domain's snapshot orchestrator (in `librarian-domain`) wraps these calls, writes a manifest record, applies retention.
- CLI route: `librarian snapshot` and `librarian restore <id>` invoke the orchestrator (not the adapter directly — hexagonal discipline per ADR-0004).
- Orchestrator is generic over the `Snapshotter` trait; the CLI binds the concrete `QdrantNasSnapshotter` at the composition root.

## Test plan

- Integration: run snapshot against a containerised Qdrant; restore into a fresh container; verify points match.
- Retention: create 5 snapshots, `prune(3)` deletes 2 oldest.
