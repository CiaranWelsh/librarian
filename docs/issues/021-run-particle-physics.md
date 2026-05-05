# 021 — Run particle-physics + HEP code corpus

**Phase:** I (post-v1 ops) · **Status:** Open · **Deps:** 020

## Goal

Execute the runbook against the particle-physics corpus on Turbo: ~280 papers from `~/Documents/ParticleDetectorPapers/data/library/`, plus the HEP code overlay, plus selected overlap books. With slice 020 landed, this is the first time the dual-vector code path runs in anger.

## Acceptance criteria

- `~/.librarian/particle-physics.toml` written: `content_type = "paper"`, `extractor = "pdf"`, `[embedder]` for text vectors, `[code_embedder]` for the `voyage-code-3` code vectors.
- Three ingestion runs against the same collection (idempotent re-ingest is the design, but for the first pass each is a fresh source set):
  1. Papers — 280 PDFs.
  2. HEP code — directory tree of HEP source files; verify `code` named vectors land alongside `text`.
  3. Overlap books — small subset of books whose chapters relate to particle physics.
- ≥99% Documents in `Success` / `Cached`. Failed Documents diagnosed via manifest, recorded in postmortem.
- A snapshot taken and persisted to NAS.
- One golden query of each modality returns plausibly relevant hits via MCP:
  - text query → returns paper passages.
  - code-aware query → returns code chunks (filterable via `content_type=code`).

## Test plan

- Operational; no new automated tests. (Those landed in 020.)

## Postmortem

(bugs hit during the run, fixes needed)
