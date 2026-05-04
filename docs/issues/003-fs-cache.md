# 003 — Filesystem cache adapter

**Phase:** B · **Status:** Open · **Deps:** 002

## Goal

Replace the in-memory `Cache` with a filesystem-backed adapter that survives across runs.

## Acceptance criteria

- `adapter-cache-fs` crate; `FsCache` implements `Cache`.
- Configurable root directory; key → relative path scheme deterministic and collision-free.
- **Atomic writes** — write to `<key>.tmp`, fsync, rename. Partial writes never corrupt the cache.
- `get` returns `None` on missing; `Err` only on real I/O failure.
- Walking skeleton swaps in `FsCache`; survives across process restarts.

(Cache-reuse measurement and zero-rework guarantees belong to slice 007 — not this slice.)

## Test plan

- Unit: round-trip, missing key, atomic rename under simulated crash (write `.tmp`, leave it, verify `get` still returns `None`).
- Integration: write/read survives process restart against the in-memory pipeline.
