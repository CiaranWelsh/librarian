# 017 — Multimodal extension

**Phase:** G · **Status:** Open · **Deps:** 016

## Goal

Figure-and-caption extraction for papers; multimodal embedder produces a third named vector (`figure`) on chunks that carry an image. Closes the gap from §6 phasing point 2 in the requirements.

## Acceptance criteria

- New `ChunkPayload::Figure(FigureMeta)` variant (don't piggyback on `PaperMeta` — figures are a distinct content shape with caption + image bytes + page).
- `adapter-extractor-pdf` extended (or new `adapter-extractor-multimodal`) to emit figure chunks with image bytes + caption.
- `adapter-embedder-multimodal` (CLIP-style or vendor of choice) produces a `figure` named vector.
- Indexer adds the `figure` named vector slot (additive to the existing collection — a fresh re-ingest is acceptable for v1).
- One reference fixture: a figure-rich paper, ≥3 figures.

## Test plan

- Integration: ingest the figure-rich paper; verify ≥3 figure chunks, captions captured, `figure` vector populated.
- Filtered search: `content_type=paper` AND has-figure-vector returns only figure chunks.
