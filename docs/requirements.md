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
| **F-5.1** | Each pipeline stage SHALL persist its output to a content-addressed cache on a shared filesystem path. Keys SHALL include the input content hash and the producing adapter's version identifier. |
| **F-5.2** | A subsequent run MUST be able to re-use cache entries. Adding a new modality (e.g. a `multimodal` embedder) SHALL re-run only the affected stage on already-extracted, already-chunked content. |
| **F-5.3** | The framework SHALL maintain a **manifest** (SQLite, single file per collection) with one row per `(document, stage)` pair: status (success / failure / skipped / cached), timestamp, error message, output reference. |
| **F-5.4** | The manifest SHALL be the source of truth for resuming a partial run. |

### 3.6 Verification

| ID | Requirement |
|---|---|
| **F-6.1** | The framework SHALL admit a configurable **verification hook** between any two stages. Verifiers receive a Document plus stage output and may flag the document as failed. |
| **F-6.2** | A reference verifier SHALL provide title-match validation for PDF-derived content, mitigating the failure mode where the wrong PDF was acquired (lesson carried over from `paper_sweep`). |

### 3.7 Operations

| ID | Requirement |
|---|---|
| **F-7.1** | The framework SHALL provide a CLI with at minimum: `ingest`, `status`, `serve`, `snapshot`, `restore`. |
| **F-7.2** | The CLI SHALL accept a single config file (TOML) describing one collection. No global state. |
| **F-7.3** | The framework SHALL provide a snapshot mechanism that triggers Qdrant's native snapshot API and copies the resulting file to a configurable path (typically a NAS mount). |
| **F-7.4** | All long-running operations SHALL emit structured progress (one line per Document or stage transition) suitable for tail-grepping. |

## 4. Quality Attributes (drivers)

| ID | QA | Concrete acceptance scenario |
|---|---|---|
| **QA-1** | **Modifiability** | Adding a new content type (e.g. *lab notebooks*) involves implementing 2–3 protocols and registering them; **no changes to core or pipeline**. ≤ 1 day of work for a new contributor. |
| **QA-2** | **Modifiability** | Swapping an embedder (OpenAI → local BGE-M3 on Turbo) requires a new adapter class plus one config line. Re-running uses cache for extraction and chunking; only embedding and indexing run. |
| **QA-3** | **Testability** | Each stage runs as a pure function in unit tests using in-memory stub adapters. **No Qdrant, no internet** required for `pytest`. Integration tests run against containerised Qdrant. |
| **QA-4** | **Reusability** | A new subject collection requires writing one TOML config and pointing at a content directory. Zero code changes. |
| **QA-5** | **Reliability** | One malformed PDF in a 500-document batch results in one manifest row marked failed; the other 499 complete and are queryable. No partial writes corrupt the collection. |
| **QA-6** | **Deployability** | A user on Mac runs ingestion, a user on Turbo runs ingestion, both reach the same Qdrant on Turbo and the same shared cache on NAS. Same code, same config style, no environment-specific branches. |
| **QA-7** | **Observability** | The manifest answers "what happened to document X" in a single SQL query: which stages ran, which cache hit, which failed and why. |

## 5. Constraints

- **C-1** Qdrant is the only vector backend supported in v1. (ASI standard.)
- **C-2** Python ≥ 3.12.
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
3. Is **F-6 (verification hook)** required for v1, or deferred? Carrying it over from `paper_sweep` lessons argues for v1; YAGNI argues for later.
