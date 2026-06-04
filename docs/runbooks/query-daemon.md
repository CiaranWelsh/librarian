# Runbook: query daemon (`librarian-serve`)

A single stateless HTTP service that fronts all qdrant collections. The CLI
(`librarian query`) and the MCP server (`librarian-collection`) are thin clients
over its HTTP API. Design: ADR-0005.

## Config

`librarian-serve` reads a TOML config. With no `--config`, it looks at
`$LIBRARIAN_SERVE_CONFIG`, then falls back to `~/.librarian/serve.toml`
(alongside the other librarian configs and `env`).

`~/.librarian/serve.toml`:

```toml
bind = "127.0.0.1:6700"
qdrant_url = "http://localhost:6334"      # qdrant gRPC
max_concurrent_embeds = 8                 # bounds in-flight OpenAI calls
[embedder]
kind = "openai"                           # or "stub" (fake 32-dim; smoke tests only)
model = "text-embedding-3-large"          # MUST match how the collection was built
dimensions = 3072
```

The OpenAI key is read from `OPENAI_API_KEY` (e.g. `~/.librarian/env`).

> **Gotcha:** the `[embedder]` must match how the collection was embedded.
> `software` and `particle-physics` use `text-embedding-3-large` at 3072 dims;
> a mismatch compares incompatible vectors and returns empty/garbage hits.
> `kind = "stub"` is a deterministic 32-dim fake — for smoke-testing the HTTP
> layer only, never real search.

## Run (on turbo — qdrant and key are local there)

```bash
cd /data/librarian
set -a; . ~/.librarian/env; set +a            # exports OPENAI_API_KEY
cargo build --release -p query-daemon -p librarian-cli
./target/release/librarian-serve              # uses ~/.librarian/serve.toml
curl -s localhost:6700/healthz                # {"status":"ok"}
```

Ctrl-C drains in-flight requests and shuts down gracefully.

## Query

CLI (`librarian query --help` for flags; default `--daemon http://localhost:6700`):

```bash
librarian query software "hexagonal ports and adapters" --limit 5
librarian query particle-physics "time of arrival calibration"
```

HTTP:

```bash
curl -s localhost:6700/v1/collections | jq
curl -s -XPOST localhost:6700/v1/search -H 'content-type: application/json' \
  -d '{"collection":"software","query":"async trait send bound","limit":3}' | jq
curl -s 'localhost:6700/v1/documents?collection=particle-physics' | jq
curl -s -XPOST localhost:6700/v1/extract -H 'content-type: application/json' \
  -d '{"collection":"software","source_id":"<id-from-a-hit>","start":0,"end":20}' | jq
```

## Run from a workstation (tunnel to turbo's qdrant + borrow its key)

```bash
ssh -L 6335:localhost:6334 asi@turbo -N &
export OPENAI_API_KEY=$(ssh asi@turbo 'set -a; . ~/.librarian/env; printf %s "$OPENAI_API_KEY"')
# a serve.toml with qdrant_url = "http://localhost:6335"
LIBRARIAN_SERVE_CONFIG=./serve.toml librarian-serve
```

## HTTP API

| Endpoint | In | Out |
|---|---|---|
| `POST /v1/search` | `{collection, query, limit?, content_type?}` | `{hits:[{score,source_id,content_type,chunk_index,text}]}` |
| `GET /v1/documents` | `?collection` | `{documents:[source_id]}` |
| `POST /v1/extract` | `{collection, source_id, start?, end?}` | `{source_id, chunks:[{chunk_index,text}]}` |
| `GET /v1/collections` | — | `{collections:[{name,points}]}` |
| `GET /healthz` | — | `{status:"ok"}` |

Errors are `{"error":{"code","message"}}` with status `400` (bad request),
`404` (unknown collection), `502` (embedder/search failed), or `503` +
`Retry-After` (embedder rate-limited or backend unavailable).
