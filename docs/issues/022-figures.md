# 022 — Figures: decide and implement-or-drop

**Phase:** J (post-v1) · **Status:** Open · **Deps:** 021

## Goal

Resolve the figure story honestly. Slice 017 shipped `ChunkPayload::Figure(FigureMeta)` + a `figure` named-vector slot in the indexer + a stub byte-hash multimodal embedder, but it never delivered the two pieces that make figures *real*:

1. A real PDF figure extractor (image XObject discovery via lopdf, paired with caption text via heuristic).
2. A real multimodal embedder (CLIP / Voyage-multimodal / etc.) instead of a SHA-256 stub.

After running 021, decide based on whether the particle-physics corpus actually has figure-bearing queries that would be answered better with figure vectors than text-only.

## Acceptance criteria — option A: implement

- New `adapter-extractor-multimodal` crate. Walks PDF pages via lopdf, finds Image XObjects, extracts bytes + format. Pairs each with the nearest "Figure N: …" caption text on the same page (heuristic — no need for spatial layout for v1).
- Returns figure records that the runner converts into `Chunk { payload: ChunkPayload::Figure(_), text: caption }`.
- New `adapter-embedder-multimodal-vendor` (or upgrade the stub) calling a real multimodal API. Voyage's image-text endpoint or a hosted CLIP — pick one based on availability/cost.
- Wired through the runner's dual-vector path (slice 020) — code+figure can co-exist, indexer's three-slot capability already supports it.
- Reference fixture: a figure-rich paper from the particle-physics corpus, ≥3 figures extracted with non-empty captions.

## Acceptance criteria — option B: drop

- `FigureMeta` and the `figure` slot stay in domain (cheap, future-compatible).
- `adapter-embedder-multimodal-stub` removed (or marked `#[deprecated]`).
- Slice 017's issue file rewritten to flag figures as v1.1 surface area, not v1.
- Roadmap updated.

## Test plan

If option A: integration test that ingests the reference paper, asserts ≥3 figure chunks with captions and `figure` named vector populated.
If option B: existing tests cleaned up; no new tests.

## Decision log

(filled in once the call is made — pasted from a conversation, not a meeting)
