# Collection Quality Standard

**Policy: no addition to a production collection without evaluating these metrics and
comparing against the baselines below.** The collections (`software`, `particle-physics`)
are production as of 2026-06-07. This document defines the metrics, how to read them
(including their failure modes), the current baseline values, and the gate procedure for
future additions.

Why this exists: the 2026-06 corpus rebuild showed that every stage of the pipeline can
fail *silently* — Marker can loop, bookmarks can lie, caches can serve stale vectors, and
a retrieval score can flatter or slander the corpus depending on its labels. Each lesson
below was paid for. Quality is observable per stage, so when a number moves you know
which stage to blame.

---

## Stage 1 — Extraction quality (is the markdown faithful to the source?)

| Metric | Definition | Healthy | Investigate |
|---|---|---|---|
| **garble value** | U+FFFD replacement chars per kc + letter-spacing runs ("P a r t") per kc (F-EQ.2) | ~0 | > 1.0 |
| **gzip degeneracy** | `len(gzip(md)) / len(md)` | 0.25–0.45 (prose) | < 0.12 |
| **split coverage** | Σ chapter-piece pages / book pages (when chapter-splitting) | 80–102% | < 80% or > 102% |

**How to read them — the hard-won part:**
- *Garble* catches encoding/OCR damage. It does **not** catch repetition loops (valid
  words). Always pair it with the gzip lens — they detect orthogonal failure modes.
- *Degeneracy* < 0.12 means "extremely redundant text". That is a **defect** in prose
  (Marker repetition loop, duplicated extraction — identical compressed sizes across
  "different" files are a near-certain duplicate signature) but **expected** for three
  legitimate genres, which must be human-confirmed once and then left alone:
  1. hardware pin/pad tables (`domain_Timepix4-Manual__IO-PADs-Position.md`, 0.026)
  2. hardware/register manuals (`serval__Timepix3_v2.0.md`, 0.094)
  3. formal-proof appendices (`oxide-weiss-2019__Pages-0061-0126`, 0.074–0.116)
  The hardware-manual case matters most here: detector manuals are core domain content —
  never auto-delete on this metric.
- *Coverage* > 100% means overlapping page ranges = the PDF "bookmarks" are junk (figure
  anchors, broken links), not chapters. Bookmark **count** lies; coverage doesn't.
  Too-few-pieces (≤ 2 for a full book) is the same disease.

**Tools:** `experiments/inventory_quality.py` (full scan); the manifest `quality` stage
rows record garble per source at ingest time.

## Stage 2 — Ingest quality (did the pipeline accept good content?)

Per-source, per-stage status in the manifest (`extract/quality/chunk/embed/index`):
- **Failed @ embed with `http 429`** — transient rate limit; always retry before
  investigating anything else (cost us 6 "failures" that were nothing).
- **Flagged** — garble above threshold at ingest; advisory, never blocks.
- **Skipped** — section filter (Index/Contents/Cover…). Beware: index sections split
  into single-letter files (`…__I.md` … `…__W.md`) evade name-based filtering — the
  gzip lens catches them instead.

**Read after every ingest:** failed count and flagged count, then chase or accept each.

## Stage 3 — Retrieval quality (does search actually work?)

| Tier | Metric | What it means | Healthy (vs baseline) |
|---|---|---|---|
| 0 (always-on) | per-query `confidence` + label | triage only — dense-retrieval QPP correlates weakly; trust LIKELY-NO-ANSWER for abstention, not the decimals | — |
| 2 (per ingest) | `hit-rate@10` | % of golden probes with a relevant book in top-10 | = 100% |
| 2 | `MRR` | mean 1/rank of first relevant hit | ≥ baseline − noise |
| 2 | `fragment-rate@5` | % of top-5 chunks that are tiny/headings — a *chunking* quality signal | ≤ baseline |
| 2 | `mean-top` | cosine of top hit — a drift signal; compare across runs, never read absolutely | stable |
| 1 (on demand) | judge context-relevance 0–2 | the accurate read: LLM grades whether each top-k chunk actually answers | mean ≥ ~1.4/2, ≥ 50% direct |

**How to read them:**
- Hit-rate/MRR are only as fair as the golden labels. Ours are **judge-widened** (a book
  is "relevant" iff one of its chunks scores 2): `~/.librarian/golden_software.json`
  (pre-widened original kept as `.orig`) and `golden_pp.json`. The pp probes were
  generated *from* the corpus, so pp's perfect score is a regression baseline, not a brag.
- A hit-rate drop after an ingest = the new content displaced relevant results, or the
  labels need widening for genuinely-new coverage. Judge a few misses to tell which.
- Histories append to `~/.librarian/health_{software,pp}.jsonl` — drift is read there.
- qdrant `points_count` is only comparable **after async deletes settle** (the optimizer
  vacuums removed points minutes-to-hours after a replace/remove storm).

**Tools:** `librarian health <col> --golden … --history …`, `librarian judge`, and the
full suite in `/data/books/.staging/quality_work.py` on turbo.

---

## Baselines — 2026-06-07 (production sign-off)

| | software | particle-physics |
|---|---|---|
| corpus files | 1805 | 282 |
| qdrant points (settled) | 141,224 | 12,766 |
| live sources | 1931 | 282 |
| failed (residual boilerplate/junk) | 22 | 0 |
| extraction: garbled | 1 (borderline 1.03, letterspacing) | 0 |
| extraction: degenerate | 6 (all legitimate genres above + 1 stray Contents piece) | 0 |
| hit-rate@10 (widened goldens) | **100%** | **100%** |
| MRR | **0.780** | **1.000** (self-referential) |
| fragment-rate@5 | **7%** | **0%** |
| mean-top | 0.672 | 0.754 |
| Tier-1 mean context-relevance | **1.45/2** (63/120 direct) | **1.49/2** (70/120 direct) |

Chunking: recursive + markdown breadcrumbs everywhere (median chunk ~850 chars software,
~780 pp; breadcrumb prefixes present). One known content gap: `HeadFirstAlgebra`
Chapter-06. One cosmetic: `box__AdvancedComputingInElectronMicroscopy__Contents` slipped
the section filter (indexed boilerplate; harmless, prune at leisure).

## The gate — run before accepting any addition

1. **Extraction scan** the new markdown (garble + gzip). Investigate every flag;
   classify degenerates as defect vs legitimate genre. Chapter splits: check coverage %.
2. **Ingest**, then read the manifest: retry 429s; resolve or explicitly accept every
   Failed/Flagged.
3. **Health** on the collection vs this table / the jsonl history: hit-rate must hold
   at 100%; fragment-rate must not rise; MRR within noise. If new topics were added,
   consider widening probes/labels first — then re-baseline.
4. **(large additions)** Tier-1 judge sample; mean must hold ≥ ~1.4/2.
5. Update this document's baseline table (and re-run the NAS backup —
   `/data/books/.staging/backup_corpus_to_nas.sh`).

The corpus is the asset; the index is derived; these numbers are the contract.
