# 013 — MCP server (read path)

**Phase:** E · **Status:** Open · **Deps:** 004, 005

## Goal

Per-collection MCP server binary exposing semantic search and structural queries. Read-only — ingest is the CLI's job. Backs **F-4.4, F-4.5, QA-O2**.

## Acceptance criteria

- `librarian-collection` binary in `crates/server`. Single config-path arg.
- MCP tools (via `rmcp` or hand-rolled JSON-RPC over stdio if `rmcp` is insufficient):
  - `search(query, k, filter?)` — semantic search over the collection.
  - `list_documents()` — every Document in the collection (from manifest).
  - `get_extract(source_id, chunk_index_range)` — scoped retrieval within a Document.
- Filter support: at minimum `content_type` and `work_id` per F-M.4.
- Server reads MCP port from a CLI flag (allocated by 015's supervisor at spawn time).
- No authentication in v1 — trust the local network on Turbo (per C-4 / deployment view).
- The server holds a Qdrant client and `ManifestStore` (concrete types via generics); no `Pipeline` (read path only).

## Test plan

- Integration: start server against a populated collection, drive via an MCP client (or raw JSON-RPC), assert tool responses.
- Tool spec contract test: each tool's input/output schema matches MCP specification.
