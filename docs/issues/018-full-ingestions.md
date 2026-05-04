# 018 — Full ingestions

**Phase:** H · **Status:** Open · **Deps:** 017

## Goal

Run the now-stable toolchain across the user's complete corpora, in the order specified in requirements §6: books → particle-physics papers → HEP code. End of v1.

## Acceptance criteria

- `software` collection: full books corpus from `~/Documents/books/`, ingested on Turbo, served via MCP.
- `particle-physics` collection: 280 papers from `~/Documents/ParticleDetectorPapers/data/library/` + selected overlap books + figures + HEP code.
- All three operate as a fleet on Turbo: `librarian status` shows them running on distinct ports.
- Snapshot retention enabled and pruning to a configurable budget.
- Smoke queries from Claude Code on Mac return relevant results in interactive time. (Latency is a concern, not a v1 driver — no hard budget.)

## Test plan

- Operational: each ingestion run completes, manifest shows ≥99% Documents in `Success` / `Cached` state, failed Documents diagnosed via the manifest.
- Acceptance: hand-crafted golden queries return expected papers/sections; mixed-content-type queries work across `book`/`paper`/`code`/`figure`.

## Exit

After this slice, v1 is done. Subsequent work is post-v1 (REST frontend, parallelism, etc.) and out of scope.
