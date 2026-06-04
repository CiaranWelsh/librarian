# ADR-0005 — Query service: stateless daemon + thin clients

**Status:** Proposed · 2026-06-03
**Relates to:** ADR-0004 (hexagonal), ADR-0002 (architectural patterns); requirements
`docs/requirements.md` Addendum 2026-06-03 (F-Q.1, F-Q.2, QA-Q1); diagram
`docs/diagrams/06-query-service.md` (runtime C&C).

## Context

The query side of `librarian` is still served the prototype way: the Python
`books/mcp-server` (now superseded by `librarian`), and — within `librarian` — a
per-collection MCP server (`crates/server` → `librarian-collection`) started via the
fleet, with the query logic living inside that server. There is no human-facing query
path; the only way to query is through an MCP client (Claude).

Two drivers force an evolution (per the requirements Addendum):

- **F-Q.1 / F-Q.2** — queries should be served by one *proper interface*, with the CLI
  and MCP as *thin clients* over it (not MCP-only, not logic-in-the-server).
- **QA-Q1 (concurrency)** — serve ~tens of concurrent network users; stateless; no
  horizontal scale-out; the shared embedder (one API key) is the contention point.

ADR-0004 §5 already anticipated this: "multiple inbound adapters … a future REST
frontend (C-4) … the design *already* admits arbitrary inbound adapters." This ADR
realises that anticipated frontend.

## Decision

Adopt a **single stateless HTTP query daemon** on turbo, fronting *all* collections, with
the CLI and MCP server as **thin clients** over its HTTP API. Concrete commitments:

**Components** (see diagram 06):
1. **`query-core`** (new lib crate) — the query logic: `search`, `list_documents`,
   `get_extract`. Depends only on the `Embedder` trait and a new **`Searcher`** outbound
   port (generics, no `Box<dyn>`, per ADR-0004). Two access modes: *vector-search*
   (embed → search) and *metadata-scroll* (documents/extract, no embed).
2. **`query-daemon`** (new bin `librarian-serve`) — async axum server wrapping
   `query-core`; owns `Arc<embedder>` + `Arc<qdrant_client>`; stateless handlers; config
   for bind address, embed-concurrency semaphore, request timeout.
3. **CLI client** — new `librarian query <collection> "<q>"` subcommand; a thin HTTP client.
4. **MCP adapter** — `crates/server` **demoted** to a thin MCP server that translates its
   three tools to daemon HTTP calls; no query logic of its own.

**HTTP API (v1)** — 1:1 with the MCP tools so the adapter is pure translation:

| Endpoint | In | Out | Backs |
|---|---|---|---|
| `POST /v1/search` | `{collection, query, limit?, content_type?}` | `{hits:[{score, source_id, content_type, chunk_index, text}]}` | `search` |
| `GET /v1/documents` | `?collection` | `{documents:[source_id,…]}` | `list_documents` |
| `POST /v1/extract` | `{collection, source_id, start?, end?}` | `{source_id, chunks:[{chunk_index, text}]}` | `get_extract` |
| `GET /v1/collections` | — | `{collections:[{name, points}]}` | — |
| `GET /healthz` | — | `{status:"ok"}` | — |

**Data flow (a search):** async handler → `query-core::search` → the existing sync embedder run via `spawn_blocking`
(semaphore-bounded) → `qdrant.search(collection, named-vector "text", vector, k)` → map →
JSON. `documents`/`extract` skip the embedder (scroll only).

**Embedder:** the daemon **reuses the existing sync `embed`** via
`tokio::task::spawn_blocking` (Programming Rust, Ch. 20: the endorsed way to run a blocking
call from async "without affecting responsiveness to other users"), bounded by a semaphore
(in-flight OpenAI calls → rate-limit protection). No native async embed is added — that
would be a second embed path to maintain (DRY/KISS); `adapter-embedder-openai` is untouched.
This refines the original ADR draft (which proposed a native async `embed`): QA-Q1 mandates
only *bounded, non-blocking* embeds, and `spawn_blocking` + semaphore satisfy that while
reusing tested code.

**Error → HTTP:** `400` malformed; `404` unknown collection; `502` embedder failure;
`503 + Retry-After` on embedder `429`; `503` Qdrant down; structured `{"error":{code,message}}`.

**Crate layout:** new `query-core`, `query-daemon`; add a `QdrantSearcher` (async, direct
`qdrant-client`) to `adapter-indexer-qdrant` and a `MemSearcher` to `adapter-indexer-mem`,
both behind the new `Searcher` port; refactor `crates/server` to the thin MCP adapter;
`crates/cli` gains `query` (client) and `serve` (launch daemon). The per-collection fleet
collapses to supervising the one daemon. **Ingest and `adapter-embedder-openai` are untouched.**

**Testing (TDD):** `query-core` unit-tested against `adapter-embedder-stub` +
`adapter-indexer-mem` (no network) for ranking / documents / extract scoping; `query-daemon`
in-process axum integration tests + a **concurrency gate** (N concurrent requests succeed);
thin clients tested for args→HTTP translation against a mock daemon.

## Why this fits the drivers

- **QA-Q1 is carried natively by "stateless + async."** No per-request/session state →
  concurrency is the tokio worker pool; "tens of users" needs no scale-out. The only
  bounded connector (daemon→OpenAI, via semaphore) is exactly the documented contention point.
- **F-Q.1/F-Q.2 fall out of the component split.** Logic lives once in `query-core`; the
  daemon's HTTP API is the "proper interface"; CLI + MCP are translation-only. Mirroring the
  MCP tool shapes in the HTTP API keeps the adapter from accreting logic.
- **Consistent with ADR-0004.** `Searcher` is a new outbound port with >1 implementation
  (Qdrant + in-memory stub), passing Aniche's bar; the daemon/CLI/MCP are the "multiple
  inbound adapters" §5 promised; `query-core` is framework-free domain logic.

## What would make this the wrong choice

- If there were only ever **one local user**, the daemon would be overkill (a library would
  do). We don't — networked multi-user is the explicit QA.
- If query logic were **trivial pass-through**, `query-core` would be ceremony. It isn't:
  ranking, two access paths, async/semaphore, scoped extract.
- If we needed **>100s of users / HA**, this single daemon would be wrong. We explicitly
  don't (QA-Q1: tens, no scale-out).

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| Embedder rate-limit contention at tens of users | Semaphore bounds in-flight embeds; `429`→`503 + Retry-After`. |
| Blocking embed call stalls the async runtime | Embed runs on `spawn_blocking` (Tokio blocking pool), never on a reactor worker — Programming Rust Ch. 20. |
| HTTP shape drifts from MCP tools → adapter grows logic | API designed 1:1 with the tools; adapter stays translation-only (enforced by tests). |
| Demoted `server` crate loses behaviour in the move | Lift logic into `query-core` under its existing/new tests before deleting it from `server`. |

## Consequences

**Good.** One queryable interface for humans (CLI) and Claude (MCP); concurrency for free
via stateless async; `query-core` testable with zero network; retires `books/mcp-server`;
realises ADR-0004's anticipated REST/multi-inbound frontend.

**Costs.** A daemon to run/supervise (one service, replaces the per-collection fleet); one
network hop per query; adapter authors must keep the `Searcher` port honest; each embed
costs one `spawn_blocking` thread-hop (negligible at tens of users; the embedder stays
single-pathed). `Retry-After` is conveyed at the HTTP boundary only — the thin CLI/MCP
clients flatten daemon errors (JSON-RPC has no retry-after notion), so automated backoff is
an HTTP-client concern, not surfaced to the model/terminal.

## Further reading (local library)

- **ADR-0004** — the hexagonal/ports basis this extends (inbound vs outbound adapters).
- `docs/requirements.md` Addendum (2026-06-03) — F-Q.1/F-Q.2/QA-Q1.
- `docs/diagrams/06-query-service.md` — the runtime C&C view.
