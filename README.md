# librarian

Build queryable, Qdrant-backed **collections** from heterogeneous source material —
books, papers, code, and (later) multimodal content. One collection per subject area.
Each collection is exposed through an MCP server for use from Claude Code or any
other MCP client.

**Status:** skeleton, pre-implementation. Architecture and requirements under
[`docs/`](docs/).

## Vocabulary

| Term | Meaning |
|---|---|
| **Collection** | A single Qdrant-backed knowledge store, scoped to one subject area (e.g. *particle physics*, *software architecture*). Holds heterogeneous content types together. |
| **Content type** | The kind of source material — `book`, `paper`, `code`, `figure`, etc. Each type has its own extractor, chunker, and per-modality embedder. |
| **Adapter** | A concrete implementation of one of the framework's interfaces (extractor, chunker, embedder, indexer). Adapters are pluggable. |
| **Cache** | Content-addressed on-disk store of stage outputs (extracted text, chunks, embeddings). Lives on the local filesystem of the ingest host (Turbo). NAS is used only for snapshot backups. |
| **Manifest** | The framework's record of every input document × pipeline stage × outcome. Source of truth for "what's been ingested". |

## Layout

```
docs/                    requirements, design, ADRs
src/librarian/
  domain/                pure types — Document, Chunk, Vector, Point, EmbeddingPlan
  pipeline/              stage protocols + the runner
  adapters/              concrete adapters (the "outside" of the hexagon)
    extractors/          PDF / code / multimodal -> structured text
    chunkers/            structured text -> chunks
    embedders/           chunks -> vectors (OpenAI, Voyage, local …)
    indexers/            vectors -> Qdrant
  manifest/              SQLite-backed state
  cache/                 content-addressed artifact storage
  server/                MCP server — one per collection
  cli/                   `librarian` entry point
tests/
```

## Quickstart (planned, not yet implemented)

```bash
pip install -e '.[dev]'

# Define a collection
cat > ~/collections/particle-physics.toml <<'EOF'
name = "particle_physics"
content_root = "/mnt/nas/corpora/particle-physics"
qdrant_url = "http://turbo.local:6333"
cache_dir  = "/var/lib/librarian/cache"
snapshot_dir = "/mnt/nas/librarian-snapshots"

[embedders]
text = "openai:text-embedding-3-large"
code = "voyage:voyage-code-3"
EOF

librarian ingest --config ~/collections/particle-physics.toml
librarian serve  --config ~/collections/particle-physics.toml
```
