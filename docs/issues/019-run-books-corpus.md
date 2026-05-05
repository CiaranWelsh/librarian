# 019 — Run books corpus on Turbo

**Phase:** I (post-v1 ops) · **Status:** Open · **Deps:** 018

## Goal

Execute the v1 ingestion runbook against `~/Documents/books/` on Turbo, end to end. This is slice 018's actual operational deliverable — `cargo test` passes, but the toolchain has never been pointed at the user's real corpus. Real-world issues (PDFs the lopdf extractor chokes on, embedder rate-limits, ingest interrupted-and-resumed) only show up here.

## Acceptance criteria

- `~/.librarian/software.toml` written per the runbook (slice 018), pointing at the real Qdrant on Turbo and the real NAS path.
- `librarian ingest --config software.toml ~/Documents/books/` completes with ≥99% Documents in `Success` / `Cached` state.
- Failed Documents diagnosed via the manifest (`SELECT source_id, stage, error FROM manifest WHERE status='Failed'`); each failure has a one-line note recorded in this issue's "Postmortem" section below.
- `librarian status --config software.toml` shows the expected point count.
- One sample query via stdio MCP returns plausibly relevant chunks for a hand-crafted golden query (e.g. "hexagonal architecture" → at least one chunk from a software architecture book).
- A snapshot is taken and persisted to NAS.

## Test plan

- This slice has no automated tests of its own — it's an operational run. The test is "did the user's corpus land in Qdrant?".
- Bug reports from this slice spawn fix slices, not changes here.

## Postmortem

(filled in after the run — bugs hit, mitigations, anything we'd tighten in the toolchain)
