# ADR-0001 — Metadata model

**Status:** Proposed · 2026-05-02
**Context:** v1 design, before module decomposition and runtime diagrams.

## Context

Every chunk in a `librarian` collection carries metadata. Without a fixed model
we will end up with inconsistent payload shapes across content types,
un-filterable fields in Qdrant, and a cache key that either re-embeds too much
or serves stale embeddings. Three prior prototypes already drifted on this; the
point of `librarian` is to stop that.

We also need metadata to support the *Work* concept (a book PDF and its example
code share a logical grouping but process independently — see F-1.3).

## Decision

### 1. Five metadata layers, two homes

| Layer | Set at | Lives in |
|---|---|---|
| **Source** — title, author, year, file hash, language | extraction | Qdrant payload |
| **Structural** — chapter, section, page range, heading path, line range | chunking | Qdrant payload |
| **Provenance** — extractor / chunker / embedder name + version + config hash | each stage | Qdrant payload |
| **Grouping** — `work_id`, `content_type`, tags | ingest config | Qdrant payload |
| **Operational** — manifest state, retry count, last error | manifest writes | SQLite manifest |

Operational metadata never enters the vector payload.

### 2. Typed payload, common core + discriminated union

A single `ChunkPayload` dataclass in `domain/` is the source of truth. Every
indexer serialises through it. Shape:

```python
ChunkPayload:
    # common core — present on every chunk
    chunk_id: str
    work_id: str
    content_type: Literal["book", "paper", "code", ...]
    source_hash: str
    language: str | None
    provenance: Provenance       # extractor/chunker/embedder name + version + config_hash
    # type-specific — exactly one populated, matching content_type
    book:  BookMeta  | None
    paper: PaperMeta | None
    code:  CodeMeta  | None
```

`BookMeta` carries `title`, `author`, `chapter`, `section`, `page_start`, `page_end`.
`PaperMeta` carries `doi`, `venue`, `year`, `section`, `page_start`, `page_end`.
`CodeMeta` carries `repo`, `path`, `commit_sha`, `line_start`, `line_end`, `symbol`.

Adding a new content type = adding one `XMeta` class and one literal value. No
existing payloads change.

### 3. Qdrant-indexed fields fixed at collection creation

Indexed (filterable) at create-time:

- `content_type`
- `work_id`
- `language`
- `book.chapter`, `paper.doi`, `code.repo` (the natural per-type filters)

Everything else is stored-only. Retrofitting payload indices on a populated
collection is expensive — get this right at `create_collection`.

### 4. Cache key formula

The cache is content-addressed. The key for a stage output is:

```
sha256(source_hash || stage_name || stage_version || config_hash)
```

- `source_hash` — sha256 of the input bytes for that document.
- `stage_version` — adapter implementation version (bumped on behavioural change).
- `config_hash` — sha256 of the canonicalised stage config (model name, chunk
  size, etc.).

Same formula at every stage. The `Provenance` recorded on each chunk is exactly
the tuple that went into its cache keys upstream — so a chunk's payload tells
you, unambiguously, what would need to invalidate to re-embed it.

### 5. The `Work` concept

A `Work` is a named grouping of Documents that share provenance (e.g. a book
plus its example-code repository). Works are **metadata-only**:

- Declared in the ingest manifest: each Document optionally names a `work_id`.
- Documents in the same Work process independently (F-1.3 holds).
- Joins happen at query time via Qdrant payload filters on `work_id`.

There is no Work-level pipeline stage and no cross-Document ordering.

## Consequences

**Good.**
- One payload schema across content types, validated before upsert.
- Adding content types is additive; existing data is untouched.
- Cache invalidation is principled — bump a `stage_version` or `config_hash`
  and only affected chunks recompute.
- "Book + its code" works without coupling pipelines.

**Costs.**
- Payload indices must be planned at collection creation; changing them later
  means re-creating the collection.
- Every adapter must populate `Provenance` honestly. Cheating breaks the cache
  contract silently.
- The discriminated union forces a small amount of boilerplate per content
  type (one class, one literal, one branch in the indexer serialiser).

## Alternatives considered

- **Free-form dict payloads.** Rejected: this is exactly the failure mode of
  the prior prototypes.
- **One flat union of all fields.** Rejected: filters become ambiguous
  (`page_start` means different things for code vs books) and the schema grows
  without bound.
- **Operational state in the payload.** Rejected: retries and failure state
  are write-heavy and don't belong in a vector store.