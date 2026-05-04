# Roadmap — librarian v1

Eighteen slices, eight phases. Each slice is a vertical cut that lands runnable, tested software and depends only on prior slices.

## Phases

| Phase | Slices | Goal |
|---|---|---|
| **A — Skeleton** | 001, 002 | Domain compiles; first end-to-end ingest with all in-memory adapters. |
| **B — Real persistence** | 003, 004, 005 | Replace in-memory cache, manifest, indexer with filesystem / SQLite / Qdrant. |
| **C — Robustness** | 006, 007, 008 | Per-doc fault boundary, idempotent re-ingest, atomic update + remove. |
| **D — Real content** | 009, 010, 011 | PDF text extractor, OpenAI embedder, fallback adapter chain. |
| **E — Frontends** | 012, 013 | CLI, MCP server (read path). |
| **F — Operations** | 014, 015 | Snapshotter, supervisor + fleet registry. |
| **G — More content** | 016, 017 | Code adapters; multimodal extension. |
| **H — Production** | 018 | Full ingestions on real corpora. |

## Slices

| # | Title | Phase |
|---|---|---|
| 001 | Domain skeleton | A |
| 002 | Walking skeleton — in-memory end-to-end | A |
| 003 | Filesystem cache adapter | B |
| 004 | SQLite manifest adapter | B |
| 005 | Qdrant indexer adapter | B |
| 006 | Per-document fault boundary | C |
| 007 | Idempotent re-ingest (cache reuse) | C |
| 008 | Update and remove (F-1.8, F-1.9) | C |
| 009 | PDF text extractor | D |
| 010 | OpenAI embedder | D |
| 011 | Fallback adapter chain (F-1.6, QA-F2) | D |
| 012 | CLI commands | E |
| 013 | MCP server — read path | E |
| 014 | Snapshotter (F-7.3, QA-O3) | F |
| 015 | Supervisor + fleet registry (F-9.x, QA-O1) | F |
| 016 | Code adapter family | G |
| 017 | Multimodal extension | G |
| 018 | Full ingestions | H |

## Working through them

Stack lives at `docs/unstaged/STACK.yaml` (gitignored). Bottom frame = current focus. One issue popped per merge.
