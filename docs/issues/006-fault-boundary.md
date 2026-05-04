# 006 — Per-document fault boundary

**Phase:** C · **Status:** Open · **Deps:** 005

## Goal

A failure on one Document does not halt the batch. Manifest records the failure; remaining Documents process normally. Backs **F-1.3, QA-F1**.

## Acceptance criteria

- Pipeline runner catches errors at the Document boundary (not inside individual stages).
- On error: write `ManifestStatus::Failed` with the error message and stage of failure; move to next Document.
- No partial writes to Qdrant for a failed Document — either all chunks indexed or none.
- Public function `Pipeline::ingest_batch(&[Document])` exists and returns a per-Document outcome summary.

## Test plan

- Stub-based: stub `Extractor` errors on document index 2/5; runner completes, manifest shows 4 success + 1 failed, indexer has 4 documents' worth of chunks.
- Same with stub `Embedder` failing — assert no Qdrant writes for that doc.
