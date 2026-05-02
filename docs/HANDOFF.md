# `librarian` — Handoff

**Date:** 2026-05-02
**Status:** Skeleton committed (`c4935d5`). Requirements drafted, awaiting review.
Architecture pinned in conversation; not yet written up as a design doc.
**Audience:** Future-me, or anyone picking this up cold.

## 1. What this project is

`librarian` is a Python framework that turns a directory of source files (books, papers,
code, eventually multimodal content) into a queryable Qdrant **collection**, exposed via
an MCP server. One collection per subject area. The framework's job starts when the
content is on disk — it does not acquire material.

It exists because three prior prototypes each solved a slice of the problem in isolation
and have started to drift:

| Prior project | Path | What it does | What to take |
|---|---|---|---|
| `book-library` MCP | `~/Documents/books/mcp-server/` + `~/Documents/books/tools/` | Single Qdrant `library` collection with books + papers, text-embedding-3-large, MCP tools (`search_books`, `get_chapter_extract`, `list_books`). Active and useful. | The schema sketch (`SCHEMA.md`), the controlled tag vocabulary, the `add_paper.py` / `add_book.py` ingest pattern, and the chunking heuristics from `tools/chunk.py`. The MCP server shape from `server.py`. |
| `code-mcp` | `~/Documents/code-mcp/` | JavaParser monorepo indexed with Voyage-code-3 into a separate Qdrant on port 6336. Pilot. | The Voyage adapter pattern, the per-language extraction, and the docker-compose layout. |
| `paper_sweep` (in `ParticleDetectorPapers`) | `~/Documents/ParticleDetectorPapers/paper_sweep/` | Pipeline interfaces (`sources → enricher → fetcher → parser → chunker → embedder → ingestor`), Phases 1–4 implemented, never connected to Qdrant. Acquisition tooling that produced the 280 papers in `data/library/`. | The frozen-dataclass domain models (`pipeline/models.py`), the pipeline runner (`pipeline/runner.py`), the section-aware chunker (`pipeline/chunker.py`). The acquisition pieces stay where they are — they're corpus-specific, not framework. |

`librarian` consolidates these three. After v1, the books MCP and code-MCP can be
re-pointed at `librarian`-managed collections; the divergent infrastructure goes away.

## 2. Where we are right now

| Artifact | State | Path |
|---|---|---|
| Repo + git | Initialised | `~/Documents/librarian` |
| Skeleton package | Empty `__init__.py` files only — directory shape communicates architecture | `src/librarian/{domain,pipeline,adapters,manifest,cache,server,cli}` |
| Functional requirements | **Drafted, awaiting user review** | `docs/requirements.md` |
| Design document | Not yet written | `docs/design.md` (TODO) |
| ADRs | Not yet written; decisions captured here for now | `docs/adr/` (TODO) |
| Implementation plan | Not yet written | `docs/superpowers/plans/` (TODO) |
| First commit | `c4935d5` | `[L-001: draft] Skeleton, requirements draft for review` |

## 3. What's already decided (don't re-litigate)

These were discussed in the brainstorm and pinned. If you find yourself questioning one,
re-read this section before starting over.

| Decision | Resolution | Why |
|---|---|---|
| Scope of stages | **Narrow.** Framework starts at extract → chunk → embed → index → serve. Acquisition stays out. | Acquisition strategies are corpus-specific (see particle-physics-papers session — Unpaywall, EZproxy, manual etc). The downstream half is what's universal. |
| Collection topology | **One Qdrant collection per subject area** ("particle physics", "software architecture"). Heterogeneous content types live inside. | User's choice. Cleaner than books-mcp's "library = books + papers" conflation. |
| Single Qdrant or many | **One Qdrant instance, many collections.** | Operational simplicity; collections are independent in Qdrant anyway. |
| Embedding strategy | **Hybrid (option C from brainstorm).** Every chunk has a primary `text` vector (one unified search space across the whole collection). Code chunks additionally have a `code` vector for specialised retrieval. Multimodal added later as a third optional vector. | User wants both unified discovery AND specialised retrieval where it matters. |
| v1 embedders | **OpenAI `text-embedding-3-large` for text; Voyage `voyage-code-3` for code.** | What user has API access to today. Replaceable via adapter swap (e.g. local BGE-M3 on Turbo) when desired. |
| Caching | **Per-stage content-addressed cache on the ingest host's local disk (Turbo).** Stages: extracted-text, chunks, embeddings. Keys: `(source_hash, adapter_version)`. | Cache is hot-path I/O; keep it local. NAS reserved for snapshot backups. |
| State | **One SQLite manifest per collection**, single file. Source of truth for resumes/retries. | Simpler than the current `acquisition_state.json` JSON-blob pattern; queryable. |
| Deployment | **Everything stateful on Turbo** (Qdrant, supervisor, collection servers, cache, manifests, fleet registry). NAS receives snapshot backups via one-shot push. Clients (Claude Code on Mac) reach Turbo via Tailscale; ingest runs on Turbo. | Co-locates ingestion with GPU; single source of truth for QA-O2. |
| Query exposure (v1) | **MCP server only.** REST/HTTP frontend is designed-for but deferred. | Claude Code is the primary consumer right now. |
| Phasing | **Toolchain first, full ingestions last.** Implement adapters for `book` (text), `paper` (text), `code` against 1–2 test subjects per type. *Then* multimodal-papers. *Then* run full ingestions: books → particle-physics papers → HEP code. | Avoids re-processing source bytes when a new modality lands. User's explicit constraint. |

## 4. The first concrete corpus

The first ingestion target is the **particle-physics papers** corpus already on disk:

- 280 verified PDFs at `~/Documents/ParticleDetectorPapers/data/library/`
- Filename scheme: `{LastName}{Year}-{title-slug}.pdf`
- Authoritative metadata in `~/Documents/ParticleDetectorPapers/data/acquisition_state.json`
  (one entry per paper_id, with title, authors, year, doi, pdf_path, verification status)
- Excel source of truth: `~/Documents/ParticleDetectorPapers/docs/Publications for Website.xlsx`

This corpus is the v1 driver. It's the "real second instance" that exercises the
abstractions beyond the books case.

The eventual `particle_physics` collection will combine:

- ~280 papers (this corpus)
- Selected books from `~/Documents/books/` that overlap (a subset, not all of them — pick based on subject)
- HEP/detector code corpus once `code_corpus/` is built out (currently Phase 1 discovery in `~/Documents/ParticleDetectorPapers/code_corpus/`)

## 5. Architectural shape (one paragraph)

Hexagonal core: pure domain types (`Document`, `Chunk`, `EmbeddingPlan`, `Point`) in
`src/librarian/domain/` depend on no external library. Pipeline stages — `Extractor`,
`Chunker`, `Embedder`, `Indexer`, optional `Verifier` — are Python protocols in
`src/librarian/pipeline/`. Concrete adapters (GROBID-PDF extractor, OpenAI embedder,
Qdrant indexer, …) live in `src/librarian/adapters/` and depend on the domain, never
the reverse. The runner orchestrates stages over a stream of Documents in pipes-and-filters
style; per-stage outputs are persisted to a content-addressed cache. A SQLite manifest
records every `(document, stage)` outcome. The MCP server is a separate thin layer that
opens a Qdrant connection and answers queries; ingestion and serving don't share state
beyond the collection itself.

Tactics from the SAIP / Clean Architecture / POSA1 / FSA library searches mapped 1:1 to
quality attributes — full table in `requirements.md` §"Tactic / Pattern Selection" once
`design.md` is written; for now it lives in the brainstorm transcript.

## 6. What to do next (in order)

1. **User reviews `docs/requirements.md`** and answers the three open questions in §8:
   - manifest *curation* in scope?
   - snapshot/restore in core or sidecar?
   - verification hooks in v1 or deferred?
2. **Write `docs/design.md`** based on the requirements + decisions in §3 above.
   Use the `superpowers:writing-plans` skill *after* the design is approved, not before.
3. **Write the first ADR** (`docs/adr/0001-narrow-scope-no-acquisition.md`) capturing
   the in/out of scope decision so it's not lost.
4. **Implementation plan** comes from `writing-plans`, broken into phases:
   - Phase 0: domain types + pipeline protocols, with stub adapters and tests.
   - Phase 1: PDF/text adapter family (extractor, chunker, OpenAI embedder, Qdrant indexer) — drive against ONE book or paper from the existing corpora.
   - Phase 2: code adapter family — drive against a small repo (probably the JavaParser one already used by code-mcp).
   - Phase 3: multimodal adapter family — drive against one figure-rich paper.
   - Phase 4: MCP server + CLI integration.
   - Phase 5: full ingestions.
5. Once the design is approved, **port the books-mcp's existing `tools/chunk.py`** as
   the first chunker adapter — don't reinvent. Same for the OpenAI embedding wrapper.

## 7. Cross-repo / cross-machine pointers

- **Mac (this machine):** dev box, has Claude Code, currently runs Qdrant on `:6333` (books) and `:6336` (code-mcp pilot). Will eventually only run query clients.
- **Turbo:** GPU server. Will host the canonical Qdrant for `librarian` collections. Currently used ad-hoc for embedding work via SSH (see `~/.claude/projects/.../memory/cw-ingest-paper-to-library.md`).
- **NAS:** snapshot backup target only. Snapshots are pushed via HTTPS / SCP from Turbo; not mounted. The per-stage cache lives on Turbo's local disk, not on NAS.
- **Books MCP** lives in `~/Documents/books/mcp-server`; its `SCHEMA.md` is the most useful schema reference.
- **Particle-detector papers session** ended with commit `2e0d046` on the `main` branch of `~/Documents/ParticleDetectorPapers/`. That session left behind the 280 PDFs, the acquisition tooling (which we are *not* lifting into `librarian`), and the conversational lessons captured in §3 above.

## 8. Open questions (recap from requirements §8)

1. **Manifest curation** — does `librarian` ship a "scan a directory → emit a starter manifest" tool? Currently treated as user's responsibility; small, would fit in `cli/`.
2. **Snapshot/restore** — core framework or sidecar `librarian-ops` package?
3. **Verification hooks (F-6)** — required for v1 or deferred to v1.1?

These are all things the user should sign off on before the design doc is written.

## 9. Things you do NOT need to re-investigate

- Whether to use Qdrant. (Yes. ASI standard.)
- Whether to use MCP for query. (Yes. Pattern across ASI.)
- Whether to acquire content. (No. User provides.)
- Whether one collection holds books + papers + code. (Yes — heterogeneous payloads, named vectors per modality.)
- Whether to support self-hosted local embedders eventually. (Yes — must be a clean adapter swap. Not v1.)
- Whether v1 needs a web UI. (No. MCP only.)

If a future conversation starts arguing one of these, paste the corresponding row from §3.
