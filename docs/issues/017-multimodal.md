# 017 — Multimodal extension

**Phase:** G · **Status:** Open · **Deps:** 016

## Goal

Figure-and-caption extraction for papers; multimodal embedder produces a third named vector (`figure`) on chunks that carry an image. Closes the gap from §6 phasing point 2 in the requirements.

## Acceptance criteria

- New `ChunkPayload::Figure(FigureMeta)` variant (caption + page + figure_number).
- `adapter-embedder-multimodal-stub`: deterministic byte-hash embedder so the pipeline shape is exercised without a real CLIP/vendor model. Real model integration deferred until a corpus drives it.
- Indexer supports multiple named-vector slots via `open_with_slots(...)`. `figure` joins the existing `text` (and optional `code`) slots.
- **Deferred to slice 018**: real PDF figure extraction (image XObject discovery via lopdf + caption-pairing heuristics) and the figure-rich reference fixture. Both ride along with the actual particle-physics corpus ingestion where their behaviour can be tuned against real input.

## Test plan

- Integration: ingest the figure-rich paper; verify ≥3 figure chunks, captions captured, `figure` vector populated.
- Filtered search: `content_type=paper` AND has-figure-vector returns only figure chunks.
