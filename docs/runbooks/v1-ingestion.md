# v1 ingestion runbook

How to operate librarian against the real corpora (slice 018).

## Pre-flight

- Qdrant container running on Turbo: `docker run -d -p 6333:6333 -v /var/lib/qdrant:/qdrant/storage qdrant/qdrant:latest`.
- NAS reachable from Turbo (HTTPS or SCP target). Path mounted at `/mnt/nas/librarian/`.
- `librarian` and `librarian-collection` binaries on PATH (`cargo install --path crates/cli && cargo install --path crates/server`).
- API keys exported: `OPENAI_API_KEY` for the text embedder; `VOYAGE_API_KEY` if using the code embedder family.

## Configs (one TOML per collection)

Each collection has its own `~/.librarian/<name>.toml`.

```toml
collection = "software"

[qdrant]
url = "http://localhost:6333"

[paths]
cache = "/var/lib/librarian/software/cache"
manifest = "/var/lib/librarian/software/manifest.sqlite"
snapshots = "/mnt/nas/librarian/software/"

[embedder]
kind = "openai"
model = "text-embedding-3-large"
dimensions = 3072

[ingest]
content_type = "book"
extractor = "pdf"

[snapshot]
retention = 5
```

Variants:

- **`particle-physics.toml`**: same shape, `content_type = "paper"`, `extractor = "pdf"`. Cache and manifest paths under `particle-physics/`.

## Phase 1 — books corpus → `software` collection

```bash
librarian ingest --config ~/.librarian/software.toml ~/Documents/books/
```

Expected: structured progress lines (`ok\tsource=…\tchunks=…`) per file. Re-run is idempotent — second invocation hits the cache for unchanged files. Inspect:

```bash
librarian status --config ~/.librarian/software.toml
# collection: software
# points: <N>
# manifest: success=… cached=… failed=…
```

## Phase 2 — particle-physics papers → `particle-physics` collection

```bash
librarian ingest --config ~/.librarian/particle-physics.toml ~/Documents/ParticleDetectorPapers/data/library/
```

Diagnose any rows in `Failed` state via the manifest:

```bash
sqlite3 /var/lib/librarian/particle-physics/manifest.sqlite \
  "SELECT source_id, stage, error FROM manifest WHERE status='Failed';"
```

## Phase 3 — code overlay (HEP code, slice 016 capability)

The CLI's v1 dispatch only wires the single-vector path. To populate the `code` named vector slot for HEP code, use the dual-vector pipeline directly via a small driver script (see `crates/adapter-indexer-qdrant/tests/named_vectors.rs` for the shape) or wait for the runner-level dual-vector wiring (post-v1).

## Phase 4 — fleet up on Turbo

```bash
librarian start software        --config ~/.librarian/software.toml
librarian start particle-physics --config ~/.librarian/particle-physics.toml
librarian status                # ports listed; uptime ticks
```

Each `librarian-collection` exposes the MCP tools `search` / `list_documents` / `get_extract` over stdio for connecting clients.

## Phase 5 — snapshots + retention

```bash
librarian snapshot --config ~/.librarian/software.toml
librarian snapshot --config ~/.librarian/particle-physics.toml
```

`retention = 5` in each config means after 6 invocations only the newest 5 NAS files remain. Pruning is automatic post-snapshot; nothing else to do.

## Recovery drill (manual)

To verify a snapshot before something goes wrong:

```bash
# pick a snapshot id from the NAS dir
ls /mnt/nas/librarian/software/

librarian restore --config ~/.librarian/software.toml <snapshot_id>
librarian status --config ~/.librarian/software.toml   # points should match
```

## Smoke queries from Mac

Once the fleet is up on Turbo, point Claude Code (or any MCP client) at the running collections and try a few hand-crafted queries. Slice 018 acceptance is "interactive-time results across the three collections" — there is no hard latency budget in v1.

## What's deferred past v1

- Real PDF figure extraction (slice 017's `adapter-extractor-multimodal` is a stub) — surface area exists; semantics arrive when the corpus needs them.
- Real multimodal embedding (CLIP / vendor) — currently a deterministic stub.
- Runner-level dual-vector wiring for code chunks (slice 016 capability proven at the indexer; runner integration deferred).
- REST frontend, parallelism, hot-reload — explicitly post-v1 per the requirements doc.
