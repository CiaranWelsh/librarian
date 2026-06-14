# Changelog

All notable changes to the librarian CLI are recorded here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/); versions track `crates/cli`.

## [1.1.0] - 2026-06-14

### Added

- **`librarian add <path> --to <collection>` — quality-gated single-resource ingestion.**
  One ergonomic command to add a PDF, ebook, HTML, code, or markdown file to a collection,
  with quality protection as the organising principle:
  - **Preview by default** (no `--commit`): infer type, derive the canonical path, extract and
    chunk in memory, and report an intrinsic-quality verdict — Gate 1 (`garble_signal` +
    section classification) plus chunk-count and fragment-rate — **writing nothing**. A garbled
    extraction aborts unless `--force`.
  - **`--commit`** places the file at its canonical corpus path and ingests it by reusing the
    existing pipeline. PDFs follow the decoupled flow (issue 029): Marker runs once to produce
    durable markdown, which is ingested via the text extractor (never re-Markered on re-ingest);
    in-place types (ebook/html/code/markdown) are ingested directly.
  - **Gate 2** runs after a successful commit: it measures retrieval health against the
    collection's golden probe set through the daemon, compares to the recorded JSONL history with
    a statistical regression test (mean ± k·σ with an absolute noise floor), and **auto-rolls-back**
    a regressing addition (removes the points and the placed files). It **fails open** with a loud
    warning when there is no golden set or the daemon is unreachable.
  - **Idempotency**: an already-present resource is skipped (`--force` re-adds).
  - **`--undo <source_id>`** reverses an add (removes points, the placed file, and an archived
    raw PDF). Plus `--shelf`, `--slug`, `--move`, and `--json` output.

- **Canonical relative source_ids and corpus layout (ADR-0007).** Ingest now enforces
  `<corpus_root>/<type>/<resource>` relative source_ids via `canonical_source_id`, so a corpus
  is portable across machines and the layout is consistent and traceable.

### Fixed

- `cli_integration` ingest round-trip assertions aligned with the recursive-chunker default
  (single chunk for the small fixture) and the relative source_ids from ADR-0007.

### Notes

- Set `LIBRARIAN_DAEMON=http://turbo:6700` on the host so `add`'s Gate 2 — and `query`/`health` —
  reach the daemon by default instead of `localhost:6700`.
- Known pre-existing issue (unrelated to this release): the `snapshot` integration test targets
  qdrant's gRPC port for a REST snapshot call; tracked separately.
