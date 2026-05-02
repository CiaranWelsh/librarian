# `librarian` — Functional Requirements

**Status:** Draft for review · 2026-05-02
**Audience:** Internal · informs the design and implementation plan that follow this document.

## 1. Purpose

`librarian` is a Python framework that turns a directory of heterogeneous source
material (books, papers, code, eventually multimodal content) into a queryable
**collection** — a Qdrant vector database scoped to a single subject area, exposed
through an MCP server for use from Claude Code or any MCP-aware tool.

It exists because three previous prototypes (`books/mcp-server`, `code-mcp`,
`paper_sweep`) each solved a slice of the problem in isolation and accumulated
redundant code, divergent schemas, and operational friction. `librarian` is the
deliberate, modular, reusable replacement.

## 2. Scope

### In scope

- Stages from **on-disk source files** to **populated Qdrant collection** to
  **MCP-served queries**: extract, chunk, embed, index, serve.
- Multiple content types within a single subject collection (heterogeneous
  payloads sharing a primary text vector space, plus optional specialised
  vectors per modality).
- A pluggable adapter surface so new content types, extractors, chunkers,
  embedders, and index backends can be added without touching the core.
- Per-stage artifact caching, content-addressed, on a shared (NAS) path.
- A manifest capturing the state of every source document × pipeline stage,
  enabling resumable / idempotent re-runs.
- Operational tooling: scheduled Qdrant snapshots to NAS; CLI for ingest,
  status, and serve.

### Out of scope

- **Acquisition** of source material — discovering papers, downloading PDFs,
  scraping the web, navigating paywalls. The user provides content; the
  framework consumes it.
- **Generation** — `librarian` is a retrieval substrate. Calling an LLM with
  retrieved context is the consumer's job (e.g. Claude Code).
- **Authentication / multi-tenant access control** — assumed to be handled by
  network-level controls (Tailscale, LAN). Not a v1 framework concern.
- **A web UI** — v1 exposes collections only over MCP. A REST/HTTP frontend is
  designed-for but not implemented.

## 3. Functional Requirements

> **Convention:** "Document" means a single source item — one PDF, one source
> file, one figure-with-caption, etc. — as it appears at framework input.

### 3.1 Ingestion

| ID | Requirement |
|---|---|
| **F-1.1** | The framework SHALL accept as input a directory tree of source files plus a manifest describing what each file is (content type, source identifier, optional metadata). |
| **F-1.2** | The framework SHALL NOT acquire, download, or otherwise fetch material. The contract is "files already on disk". |
| **F-1.3** | The framework SHALL process each Document independently. A failure on Document *i* SHALL NOT halt processing of Document *j ≠ i*. |
| **F-1.4** | Re-running ingestion on the same input SHALL be idempotent: stage outputs hit the cache, Qdrant points are upserted, no duplicates appear in the collection. |
| **F-1.5** | v1 SHALL process Documents serially (one at a time). Concurrency is deferred; the design must not preclude introducing parallel workers later, but no parallel execution is implemented in v1. |
| **F-1.6** | A stage adapter MAY declare a *fallback* adapter. On a recoverable failure (configurable per adapter — e.g. timeout, GPU OOM, transient HTTP 5xx), the framework SHALL retry on the fallback. The manifest SHALL record both attempts; status SHALL be `recovered_via_fallback` on success or `failed` (with all error messages preserved) on terminal failure. |
| **F-1.7** | `ingest` SHALL be incremental by default. Running `ingest` against an existing collection with a source tree containing *N* previously-ingested Documents and *k* new ones SHALL process only the *k* new Documents end-to-end. The *N* unchanged Documents SHALL hit the cache and produce no Qdrant writes. |
| **F-1.8** | When a previously-ingested Document is updated (its source bytes change → its `source_hash` changes), the next `ingest` run SHALL re-process it end-to-end and replace its chunks atomically. Specifically: because the new version may produce a different *number* of chunks than the prior version, simply upserting the new chunks is not sufficient — chunks from the prior version whose indices no longer exist in the new version would remain in Qdrant as orphans (referring to a document state that no longer exists, yet still surfacing in search results). The framework SHALL therefore, in a single ingest operation, delete every chunk belonging to the Document's prior version and upsert the chunks of the new version, leaving the collection containing exactly the new version's chunks and no orphans. |
| **F-1.9** | The framework SHALL provide a `remove <source_id>` operation that deletes all chunks belonging to a Document from the collection and marks the Document `removed` in the manifest. Removing a source file from disk alone SHALL NOT remove its chunks — explicit `remove` is required. |

### 3.2 Pipeline stages

| ID | Requirement |
|---|---|
| **F-2.1** | The pipeline SHALL consist of these stages, in order: **extract → chunk → embed → index**. Each is independently invokable and independently cacheable. |
| **F-2.2** | The framework SHALL support multiple **content types** in a single collection. v1 must implement at least: `book`, `paper`, `code`. Multimodal-paper figures are added in a later iteration. |
| **F-2.3** | Each stage SHALL be defined by a stable Python protocol so concrete adapters can be swapped via configuration. |
| **F-2.4** | The framework SHALL provide at least one reference adapter per stage per content type planned for v1. |

### 3.3 Embedding

| ID | Requirement |
|---|---|
| **F-3.1** | Every chunk SHALL receive a **primary text vector** so the entire collection is searchable from one unified semantic space. |
| **F-3.2** | The framework SHALL support **per-modality additional named vectors** (e.g. a `code` vector specialised for code chunks) alongside the primary text vector on the same point. |
| **F-3.3** | Embedder choice SHALL be configurable per content type. v1 ships with: OpenAI `text-embedding-3-large` for text, Voyage `voyage-code-3` for code. |
| **F-3.4** | The framework SHALL allow a future swap to a self-hosted local model (e.g. running on the user's GPU server) by registering a new Embedder adapter and changing one config line. No core changes. |

### 3.4 Storage and retrieval

| ID | Requirement |
|---|---|
| **F-4.1** | All collections SHALL live in a single Qdrant instance (multiple collections, one Qdrant). |
| **F-4.2** | Each subject collection SHALL be one Qdrant collection holding heterogeneous content types together, distinguished by a `type` payload field and (optionally) different vector slots populated. |
| **F-4.3** | Point IDs SHALL be deterministic, derived from `(source_id, chunk_index)`, so re-ingestion upserts. |
| **F-4.4** | The framework SHALL expose a populated collection via an MCP server exposing semantic search and a small set of structural queries (list documents, fetch by ID, scoped retrieval within a content type). |
| **F-4.5** | The MCP server SHALL be one process per collection, configured via the same TOML/YAML file used at ingestion time. |

### 3.5 Caching and state

| ID | Requirement |
|---|---|
| **F-5.1** | Each pipeline stage SHALL persist its output to a content-addressed cache on the local filesystem of the host where ingest runs. Keys SHALL include the input content hash and the producing adapter's version identifier. The cache is not shared across hosts. |
| **F-5.2** | A subsequent run MUST be able to re-use cache entries. Adding a new modality (e.g. a `multimodal` embedder) SHALL re-run only the affected stage on already-extracted, already-chunked content. |
| **F-5.3** | The framework SHALL maintain a **manifest** (SQLite, single file per collection) with one row per `(document, stage)` pair: status (success / failure / skipped / cached), timestamp, error message, output reference. |
| **F-5.4** | The manifest SHALL be the source of truth for resuming a partial run. |

### 3.6 Metadata

| ID | Requirement |
|---|---|
| **F-M.1** | Every chunk SHALL carry a typed payload conforming to a single `ChunkPayload` schema defined in the framework core. The schema SHALL be the source of truth for what is stored in Qdrant; no adapter may write payload fields outside it. |
| **F-M.2** | The payload SHALL distinguish five kinds of metadata: **source** (intrinsic to the document), **structural** (location within the document), **provenance** (which adapter + version + config produced it), **grouping** (`work_id`, `content_type`, tags), and **operational** (manifest state). Operational metadata SHALL live in the manifest only and SHALL NOT enter the Qdrant payload. |
| **F-M.3** | The payload SHALL consist of a **common core** present on every chunk (including `chunk_id`, `work_id`, `content_type`, `source_hash`, `provenance`) plus a **content-type-specific block** (e.g. `BookMeta`, `PaperMeta`, `CodeMeta`) selected by `content_type`. Adding a new content type SHALL be additive — no existing payloads change. |
| **F-M.4** | The framework SHALL fix the set of Qdrant-indexed (filterable) payload fields at collection-creation time. At minimum these SHALL include `content_type` and `work_id`. |
| **F-M.5** | A **Work** is a named grouping of Documents that share provenance (e.g. a book and its example-code repository). Works are metadata-only: Documents MAY declare a `work_id` in the ingest manifest; same-Work Documents SHALL be processed independently per F-1.3 and joined only at query time via payload filters. There SHALL be no Work-level pipeline stage. |
| **F-M.6** | Every chunk's `provenance` SHALL record the adapter name, version, and config hash for each upstream stage. The recorded provenance SHALL match the cache keys that produced the chunk (per F-5.1), so a chunk's payload unambiguously identifies what would need to invalidate to re-derive it. |

### 3.7 Operations

| ID | Requirement |
|---|---|
| **F-7.1** | The framework SHALL provide a CLI with at minimum: `ingest`, `remove`, `status`, `start`, `stop`, `restart`, `snapshot`, `restore`. |
| **F-7.2** | The CLI SHALL accept a single config file (TOML) describing one collection. No global state. |
| **F-7.3** | The framework SHALL provide a snapshot mechanism that triggers Qdrant's native snapshot API and copies the resulting file to a configurable NAS path. Snapshots SHALL use Qdrant's incremental form where supported. A rolling retention policy SHALL be configurable (e.g. retain last *N* snapshots), and the framework SHALL prune older snapshots accordingly. |
| **F-7.4** | All long-running operations SHALL emit structured progress (one line per Document or stage transition) suitable for tail-grepping. |

### 3.8 Fleet management

| ID | Requirement |
|---|---|
| **F-9.1** | The framework SHALL maintain a registry of collections on the host: name, config path, MCP port, last-known status, and last-ingest timestamp. The registry SHALL be a single small file on the host (e.g. SQLite under `/var/lib/librarian/`). |
| **F-9.2** | The CLI SHALL provide `start <collection>`, `stop <collection>`, `restart <collection>`, and `status`. `status` SHALL list all registered collections with running/stopped state, port, uptime, and Qdrant target. |
| **F-9.3** | Starting a collection SHALL allocate a non-conflicting MCP port (deterministic from config, or dynamically assigned and recorded in the registry) without affecting other running collections. |
| **F-9.4** | `start` and `stop` SHALL be idempotent: starting a running collection is a no-op; stopping a stopped collection is a no-op. Both SHALL return non-zero on actual failure. |

## 4. Quality Attributes (drivers)

Three drivers shape the architecture. Testability and modularity are universal
software-engineering concerns and are not listed as drivers; performance,
observability, and deployability are tracked as concerns in §4.4.

Each driver is captured as one or more six-part scenarios per SAIP §3.3:
*source · stimulus · environment · artifact · response · response measure*.

### 4.1 Modifiability — embedder/model swap

The reason `librarian` exists. The user must be able to experiment with
different embedding models on the same corpus without re-extracting or
re-chunking source material.

**QA-M1 · Swap to a different hosted embedder**

| | |
|---|---|
| Source | Researcher (the operator) |
| Stimulus | Decides to replace the current text embedder with a different provider/model (e.g. OpenAI `text-embedding-3-large` → Voyage `voyage-3`) |
| Environment | A populated collection exists; cache and manifest are intact |
| Artifact | The framework's embedder adapter layer + config |
| Response | Operator implements (or selects an existing) Embedder adapter and changes one config line; re-running ingestion reuses cached *extract* and *chunk* outputs and re-runs only *embed* and *index* |
| Response measure | No core or pipeline code changes. Cache hit rate for extract+chunk = 100 % on unchanged documents. Wall-clock for re-ingest is bounded by embedding throughput, not by re-extraction. |

**QA-M2 · Swap to a self-hosted local model**

| | |
|---|---|
| Source | Researcher |
| Stimulus | Decides to switch from a hosted API to a local model running on Turbo's GPU (e.g. BGE-M3) |
| Environment | Local model is reachable on Turbo; Qdrant and the local cache are unchanged |
| Artifact | Embedder adapter layer |
| Response | A new Embedder adapter is registered; one config line changes; ingest proceeds |
| Response measure | Zero changes to core, pipeline, manifest, or cache key formula. Other content types' embedders are unaffected. |

### 4.2 Fault tolerance — per-document isolation with retry and fallback

A failure on one document SHALL NOT poison the rest of the batch, SHALL be
flagged in the manifest, and SHALL be retried automatically along a configured
fallback path.

**QA-F1 · Single corrupt document in a large batch**

| | |
|---|---|
| Source | Source corpus |
| Stimulus | One malformed PDF in a 500-document ingest |
| Environment | Normal serial ingestion (per F-1.5) |
| Artifact | Pipeline runner + manifest |
| Response | The malformed document is flagged in the manifest with status `failed` and an error message; the remaining 499 documents are processed and indexed; no partial point writes appear in Qdrant for the failed document |
| Response measure | Other documents' success rate unaffected. Manifest answers "what failed and why" in one query. |

**QA-F2 · Transient infrastructure failure with fallback**

| | |
|---|---|
| Source | Embedder backend |
| Stimulus | GPU embedder fails on a document (e.g. CUDA OOM, transient API 5xx) |
| Environment | A fallback path is configured (e.g. CPU embedder, alternative provider) |
| Artifact | Pipeline runner + embedder adapter chain |
| Response | The framework retries on the fallback; on success, the manifest row is marked `recovered_via_fallback` with both attempts recorded; the document's chunks are indexed normally |
| Response measure | Zero operator intervention required. Recovery happens within the same ingestion run. If the fallback also fails, status becomes `failed` with both error messages preserved. |

### 4.3 Operability — multi-collection fleet, remote clients, single source of truth

`librarian` runs as a managed fleet on a single host (Turbo). Multiple
collections (e.g. `software`, `particle-physics`) coexist. There are no local
database replicas — clients reach the shared store over the network.

**QA-O1 · Start a new collection alongside running ones**

| | |
|---|---|
| Source | Operator (possibly remote, over Tailscale/SSH) |
| Stimulus | Runs `librarian start particle-physics` while `software` is already running |
| Environment | Turbo is up; Qdrant is running; `software` MCP server is on its assigned port |
| Artifact | The librarian fleet supervisor on Turbo |
| Response | A new MCP server process for `particle-physics` is started on a non-conflicting port, registered, and health-checked; the running `software` server is unaffected |
| Response measure | Operation completes in < 5 s. `librarian status` lists both collections with correct ports, uptime, and Qdrant target. No port collisions. |

**QA-O2 · Remote client access without local replicas**

| | |
|---|---|
| Source | MCP client (Claude Code on Mac, future laptop) |
| Stimulus | Issues a search query against an MCP server on Turbo |
| Environment | Client is on the same Tailscale network; no local copy of Qdrant or cache |
| Artifact | MCP server + Qdrant on Turbo |
| Response | Query is served from the canonical Qdrant on Turbo; results reflect the latest ingestion |
| Response measure | All clients see identical results within the same ingestion session. No data file is copied to client hosts. |

**QA-O3 · Backup-driven storage efficiency**

| | |
|---|---|
| Source | Scheduled backup job |
| Stimulus | Periodic snapshot of a collection |
| Environment | Collection has had incremental ingest since last snapshot |
| Artifact | Snapshot tooling + NAS storage |
| Response | The framework writes a new snapshot artifact and retains a rolling window of prior snapshots; storage growth on NAS is bounded by the rolling-window policy, not by linear accumulation |
| Response measure | NAS storage occupied by snapshots stays within a configurable budget (e.g. last *N* snapshots retained). Restore from any retained snapshot succeeds. |

### 4.4 Concerns (tracked, not architecture-driving)

- **Performance.** No latency target for v1. GPU memory limits force serial document processing (F-1.5); no other performance properties shape the architecture.
- **Observability.** Falls out of having a manifest (F-5.3). Single-SQL-query answer to "what happened to document X" is sufficient.
- **Testability.** Universal concern; satisfied by the adapter-protocol surface (F-2.3) which already permits in-memory stub implementations.
- **Cost.** Embedding API spend is acceptable as-is; no architectural decisions hang on it.

## 5. Constraints

- **C-1** Qdrant is the only vector backend supported in v1. (ASI standard.)
- **C-2** Implementation language: Rust (stable channel). Python may be present at runtime only as an extractor-adapter dependency (e.g. shelling out to GROBID/Marker). See ADR-0003.
- **C-3** Must run unchanged on macOS and Linux (Mac, Turbo).
- **C-4** Query exposure is via MCP server in v1. A REST/HTTP frontend is not in v1 but the design must not preclude one being added later.

## 6. Phasing of v1 development

The toolchain must be complete (across all v1 content types) before running full ingestions, so that no source bytes are processed twice when a new modality lands.

1. **Tooling pass** — define interfaces; implement reference adapters for `book` (text PDF), `paper` (text PDF), `code`. Validate each on 1–2 test subjects per content type.
2. **Multimodal extension** — figure-and-caption extraction and the multimodal embedder, validated against 1–2 test papers. (Still v1; comes after step 1 but before any full ingestion.)
3. **Full ingestion** — run the now-stable toolchain across the user's complete corpora: books → particle-physics papers → HEP code, in that order.

After step 3, v1 is done.

## 7. Glossary

| Term | Meaning |
|---|---|
| **Collection** | A single Qdrant collection scoped to one subject area. Output of `librarian`. |
| **Document** | A single input source item — one PDF, one source file, one figure. Unit of pipeline progress. |
| **Chunk** | A retrieval-sized fragment of a Document, enriched with structural metadata (chapter, heading, function name, …). Unit indexed in Qdrant. |
| **Adapter** | A concrete implementation of a stage protocol (e.g. `GrobidPdfExtractor`, `VoyageCodeEmbedder`). |
| **Manifest** | SQLite database tracking every `(document, stage)` outcome for a collection. |
| **Cache** | Content-addressed filesystem store of stage outputs, keyed by input hash + adapter version. |
| **Subject area** | The scope of one collection — e.g. *particle physics*, *software architecture*. User-defined. |

## 8. Open questions (review-time)

1. Is the **manifest curation step** (turning a directory of files into the input manifest the framework consumes) in scope? Currently treated as the user's responsibility (out of scope), but a thin "scan a directory and emit a starter manifest" tool is small and might belong here.
2. Is **F-7.3 (snapshot/restore)** core-framework or operational sidecar? Currently in scope as core; could be a separate `librarian-ops` package.
