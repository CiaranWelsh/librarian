# 007 — Idempotent re-ingest (cache reuse)

**Phase:** C · **Status:** Open · **Deps:** 006

## Goal

Re-running ingest on the same input produces zero new work for unchanged Documents and zero duplicate Qdrant points. Backs **F-1.4, F-1.7, QA-M1, QA-M2**.

## Acceptance criteria

- **Cache key ownership:** the runner derives `CacheKey` for each stage via `cache_key::derive(source_hash, adapter.name(), adapter.version(), adapter.config_hash())`. Adapters expose `AdapterIdentity` only — they never compute their own keys.
- Runner consults the cache *before* every stage; cache hit → load output, skip adapter call, write `ManifestStatus::Cached`.
- Cache key derivation matches `provenance` recorded on chunks (F-M.6).
- Adding a *k*'th new Document to a tree of *N* already-ingested → exactly *k* embed calls, *N* zero embed calls.
- Changing only the embedder version → extract+chunk hit cache; embed+index re-run.

## Test plan

- Integration with FS cache: ingest fixture, instrument embedder call count, re-run → 0 embed calls.
- Modify embedder version → re-run → all embed calls fire, zero extract/chunk calls.
- Add one new file to the tree → exactly one new embed call.
