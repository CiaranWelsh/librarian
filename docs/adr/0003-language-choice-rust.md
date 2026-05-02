# ADR-0003 — Implementation language: Rust

**Status:** Accepted · 2026-05-02
**Supersedes:** Implicit Python choice in the initial skeleton (commit `c4935d5`).

## Context

The initial skeleton (commit `c4935d5`) was Python: `pyproject.toml`,
`src/librarian/...` with empty `__init__.py` modules. No code beyond the
directory shape was committed.

Two things prompt revisiting the choice:

1. The architectural pattern selection (ADR-0002) leans heavily on
   protocol-based stage adapters and a typed `ChunkPayload` with a
   discriminated union. Both are first-class in Rust and second-class in
   Python.
2. Python is a personal-velocity drag for the only contributor. Tooling
   choice for a one-person framework should account for who's writing it.

## Decision

`librarian` is implemented in **Rust** (stable channel, current version at
time of writing).

The Python skeleton is removed. No code is migrated because no code exists
beyond empty package files.

## Why Rust fits this project

- **Type system carries the metadata model.** `ChunkPayload`'s
  common-core + per-content-type discriminated union (ADR-0001) is a
  natural Rust enum. In Python it would be a `Union[BookMeta, PaperMeta,
  CodeMeta]` discriminated by a string field, with manual validation. The
  domain stays honest in Rust because the compiler enforces it.
- **Traits replace Python protocols cleanly.** ADR-0002's
  protocol-based stage adapters become Rust traits with associated types
  where helpful. Trait objects (or generics) give the same swap-an-adapter
  flexibility with compile-time checking.
- **Qdrant is Rust-native.** Qdrant is written in Rust and ships a
  first-class Rust client. Our most-called external dependency is better
  supported in Rust than in Python.
- **Single static binaries map onto Operability (QA-O1).** A
  collection's MCP server is a single binary launched per config — no
  virtualenv, no Python version pinning, no per-host environment drift
  between Mac and Turbo. `systemd` / launchd integration is trivial.
- **Concurrency story matches eventual needs.** Even though v1 is serial
  (F-1.5), the moment we want bounded parallelism for embedding batches
  or the supervisor's port management, `tokio` is there. No GIL workaround.
- **Personal velocity.** The user writes Rust comfortably and finds
  Python frustrating. For a one-person project, the language one writes
  faster in is the right language.

## Risks acknowledged

| Risk | Mitigation |
|---|---|
| **PDF extraction ecosystem is thinner in Rust.** Python has PyMuPDF, GROBID, Marker, Nougat. Rust has `pdfium-render`, `lopdf`, `pdf-extract`. | Extraction is an outbound port (ADR-0002). Adapters MAY be subprocess wrappers around external tools (Python or otherwise) — the hexagonal port doesn't care what language the adapter shells out to. |
| **MCP SDK is not first-party in Rust.** Reference MCP SDKs are TypeScript and Python; `rmcp` is third-party. | Verify `rmcp` covers the MCP features we need (tools, stdio transport, async server) before committing the server crate. If it lags the spec, we either contribute upstream or write the JSON-RPC handling directly — MCP's wire format is small. |
| **Build/install friction for users without Rust toolchain.** | Distribute pre-built static binaries. v1 audience is one person on two known hosts; not a release-engineering problem yet. |
| **No code yet, so no migration cost.** | — (this is the "risk" being avoided by deciding now). |

## Consequences

**Good.**
- The metadata model (ADR-0001), pattern decisions (ADR-0002), and
  language choice now align: typed payloads enforced by the compiler,
  trait-based adapter swap, single-binary deploy.
- Operability (QA-O1) becomes simpler: the supervisor manages identical
  binaries differing only by config path.
- Tests can use stub adapters with no Qdrant and no internet — same
  property the Python plan had, with stronger compile-time guarantees.

**Costs.**
- The Cargo workspace layout, error-type hierarchy, and async-runtime
  choice (`tokio`) are decisions still to be made. None block this ADR.
- The PDF extractor will likely shell out to Python tools (GROBID, Marker)
  in v1. This is operationally fine but means at least one `python3` is in
  the runtime dependency graph, even if not in the build graph.
- `rmcp` is a third-party dependency. We accept upstream risk on the MCP
  surface.

## Affected requirements

- **C-2** ("Python ≥ 3.12") → updated to specify Rust (stable channel) as
  the primary implementation language. Python may be present in the
  runtime environment as an extractor-adapter dependency only.

## Out of scope for this ADR

- Cargo workspace layout (single crate vs workspace; how many bins).
- Async runtime choice (`tokio` is presumed but not formally pinned).
- Error-handling crate choice (`thiserror`, `anyhow`, custom).
- MCP server framing (one binary, multiple instances vs one binary per
  collection).

These are decisions for ADR-0004 onward, taken when the first crate is
actually being written.