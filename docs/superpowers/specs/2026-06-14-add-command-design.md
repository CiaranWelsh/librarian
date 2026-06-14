# Design: `librarian add` — quality-gated resource ingestion

**Status:** Approved (brainstorm 2026-06-14). Builds on ADR-0007 (`canonical_source_id`, G6).

## Goal
One ergonomic command to add any supported resource to any collection, following the
canonical corpus path automatically, **gated by quality so the collection cannot be
contaminated.** The human supplies only the file and the collection; everything else
(type, slug, path, extractor, Marker, config) is inferred. Quality protection is the
organising principle: the default is to *prove clean, then write*.

## Command surface
```
librarian add <PATH> --to <collection>
    [--commit] [--shelf NAME] [--slug NAME] [--move] [--force] [--judge]
librarian add --undo <source_id> --to <collection>
```
- **Default (no `--commit`) = PREVIEW**: plan + free quality checks, NO embedding, NO write.
- **`--commit`** = qualify-then-write, with the post-write health gate + auto-rollback.
- **`--undo <source_id>`** = remove a previously-added resource (thin wrap of `remove`).

## Two-phase, quality-first model
**Preview (default, free):** detect type → extract (Marker for pdf) → Gate 1 (garble/section)
→ chunk → fragment-rate + chunk stats → print plan + intrinsic verdict. No embed, no live write.
If Gate 1 fails, stop here — the collection was never touched.

**Commit (`--commit`):** re-run gates → embed → upsert into the collection → Gate 2 (health vs
baseline) → if regressed, **auto-rollback** (remove the just-added source). The live touch is
seconds and fully reversible, so the collection only *keeps* qualified material.

Rationale: intrinsic quality (garble, fragments) is measurable for free in preview; crowd-out
contamination (new material burying existing answers) only manifests once the material sits
beside the old, so Gate 2 must write-then-measure — made safe by trivial rollback.

## Requirements

### Functional (FR)
- FR-1  Accept a file `<PATH>`; `--to <collection>` (software | particle-physics) required.
- FR-2  Infer type from extension: pdf, ebook (.epub/.mobi/.azw3), html, code, markdown.
- FR-3  Derive resource slug from filename (LAYOUT.md slug rule); `--slug` overrides; optional `--shelf` prefix.
- FR-4  Plan canonical paths under `/data/corpus/<collection>/<type>/…` (path-planner = inverse of `canonical_source_id`).
- FR-5  PDF → Marker → markdown (reuse existing pipeline incl. chapter-split when a TOC exists); other types ingest in place.
- FR-6  Place the file canonically — **copy** by default; `--move` to move.
- FR-7  Resolve the ingest config from (collection, type) automatically — user never names a toml.
- FR-8  Ingest via the existing pipeline; G6's `canonical_source_id` yields the relative source_id.
- FR-9  Idempotent: detect already-present source_id; skip unless `--force`.
- FR-10 `--undo <source_id>`: remove a previously-added resource (index points + its corpus files).

### Quality (QR) — the spine
- QR-1  Preview is the default; writing requires `--commit`.
- QR-2  Gate 1 (pre-ingest, free): `garble_signal` + `classify_section` (ADR-0006). Garbled → abort, nothing written. Legit-degenerate genres exempt; `--force` overrides.
- QR-3  Preview reports chunk count + fragment-rate of the new material (no embedding).
- QR-4  Gate 2 (commit-time): `health` vs baseline (issue-028 golden probes + JSONL history); hit-rate@k, MRR, fragment-rate deltas.
- QR-5  Statistical thresholds: regression = metric drop / fragment rise beyond the health-history's historical variance (e.g. > k·σ of the rolling mean), not a fixed constant.
- QR-6  Auto-rollback on Gate-2 regression (default): remove the addition, restore the collection.
- QR-7  Optional `--judge` (issue-028 LLM relevance) for big additions; off by default (OpenAI cost); auto-considered above a configurable chunk count.
- QR-8  Record each commit's health run to the JSONL history (baseline self-updates).
- QR-9  This command is the single enforcement point automating `docs/quality-standard.md`.

### Ergonomic (ER)
- ER-1  Supply only file + collection; everything else inferred.
- ER-2  Verdict is the headline output (preview / committed / regressed-rolled-back), with the metric diffs.
- ER-3  Typed what/why/fix errors + progress (issue-037 style); `--json` for scripting.
- ER-4  Dry-run is implicit (the default); `--commit` is the only "do it" flag.

## Reuse (no rebuild)
`collect_docs` type-detect · `canonical_source_id` (G6) · Marker pdf pipeline · `ingest` runner ·
`garble_signal` / `classify_section` (ADR-0006) · `health` / `judge` + golden sets (issue-028) · `remove` (undo).

## Non-goals (v1)
- Image ingestion (.png/.jpg): excluded — no extractor exists; OCR/vision is separate new work. Flagged for a future `add` extension.
- Redesign of other CLI functions (query/extract/status/…): handled later, one function at a time.

## Defaulted decisions (override any)
image=out · chaptering=reuse existing split · slug=book-name (+ optional `--shelf`) · copy default ·
auto-rollback default · judge opt-in.
