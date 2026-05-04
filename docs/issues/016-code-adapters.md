# 016 — Code adapter family

**Phase:** G · **Status:** Open · **Deps:** 015

## Goal

Adapter trio for `ContentType::Code`: extractor, chunker, Voyage embedder. Validates F-2.2 (multiple content types in one collection) and F-3.2 (per-modality named vectors).

## Acceptance criteria

- `adapter-extractor-code`: walks a directory tree of source files. One `Document` per file. Skips binaries / vendored deps based on a configurable allowlist.
- `adapter-chunker-code`: language-aware chunking — respect function / class boundaries where possible. Initial implementation can be line-window-based with a TODO for tree-sitter.
- `adapter-embedder-voyage`: `voyage-code-3` over HTTPS. Same error-classification convention as the OpenAI embedder (010).
- Indexer adds a `code` named vector slot alongside `text` (`open_with_extra_slot`) and exposes `upsert_named` to write both on code chunks (F-3.2). Wiring this through `BatchRunner` (a 7th generic and ripple through ~10 callers) is deferred to slice 018 when an actual code corpus needs it.
- `CodeMeta` populated per F-M.3: file path, language. Repo URL / commit / symbol detection deferred until slice 018 — a tree-sitter pass would land them naturally.
- One reference fixture: a small repo of 10–50 source files in `tests/fixtures/code/`.

## Test plan

- Integration: ingest the code fixture; assert chunk count, language detection, both `text` and `code` vectors populated on code chunks.
- Cross-content-type query: search returns mixed `book`/`paper`/`code` results from the same collection, filterable by `content_type`.
