# ADR-0004 — Hexagonal architecture as the core structural pattern

**Status:** Accepted · 2026-05-02
**Relates to:** ADR-0002 (architectural patterns), ADR-0001 (metadata model), ADR-0003 (Rust).

## Context

ADR-0002 selected two patterns for `librarian`: *protocol-based stage
adapters* (the structural core) and *Client-Server* (the runtime topology).
The first pattern is Hexagonal Architecture / Ports and Adapters in
substance, with the surrounding ceremony (DDD, "Hexagonal" as a brand
name) deliberately stripped.

This ADR is the standalone justification for that choice — the case the
project would have to defend if a contributor asked "why this and not
something else?". ADR-0002 lists patterns; this ADR explains why
Hexagonal in particular *fits well*, by reference to the project's
specific drivers.

## Decision

`librarian` adopts Hexagonal Architecture as the structural pattern for
each collection process.

Concrete commitments:

- The domain — `Document`, `Chunk`, `ChunkPayload`, `Provenance`, the
  pipeline runner — depends on no infrastructure code. No imports of
  `qdrant-client`, HTTP libraries, OpenAI/Voyage SDKs, filesystem code,
  or MCP libraries from inside the domain crate.
- Stage interactions are mediated by **traits** (Rust's protocol form):
  `Extractor`, `Chunker`, `Embedder`, `Indexer`, plus a `Cache` trait.
  These are the project's outbound ports.
- Inbound ports are the operations the domain exposes for callers — the
  ingest pipeline runner, the manifest queries, the search interface.
  Multiple inbound adapters (CLI, MCP server, future REST) call the same
  inbound ports.
- We adopt the **pragmatic version** (Aniche, EST §7.5.6): a port
  exists only where there is genuinely more than one implementation.
  Ports are not created for one-implementation dependencies.
- We do **not** adopt the surrounding DDD ceremony — no aggregates, no
  value-object hierarchies, no "ubiquitous language" naming demands.
  Plain Rust structs and enums.
- The **label** "Hexagonal Architecture" is used in this ADR for
  unambiguous reference to the literature, but is not used in code,
  module names, or runtime artefacts. The structure is the structure;
  the brand is folklore.

## Why Hexagonal fits this project

Five concrete reasons, each tied to a driver from the requirements doc.

### 1. The pattern's seam matches the project's axis of variation

Hexagonal's seam is *technology vs. domain*. `librarian`'s axis of
variation is exactly that: which embedder, which extractor, which
chunker, which indexer. Every one of those is technology; what doesn't
vary is what a Document is, what a Chunk is, what the pipeline does in
the abstract.

Patterns succeed when their seam aligns with where changes happen.
Layered architecture's seam is *abstraction level*; that's misaligned for
us because every embedder swap would cut across layers. Hexagonal cuts
along the grain.

### 2. We pass Aniche's "more than one implementation" bar on every port

Aniche's pragmatic rule (EST §7.5.6) is: create a port only where there
is genuinely more than one implementation, present or imminent. We
satisfy this on every port:

| Port | Implementations |
|---|---|
| `Embedder` | OpenAI, Voyage, future BGE-M3 (local), in-memory stub for tests |
| `Extractor` | text PDF, code, multimodal PDF, future EPUB |
| `Chunker` | per content type, plus stubs |
| `Indexer` | Qdrant now, plausibly other backends later, in-memory stub |
| `Cache` | local filesystem (Turbo), in-memory stub |

Not pre-emptive abstraction — the alternatives are named and concrete.
The pattern earns its keep on day one, not "maybe in two years".

### 3. It is what makes the modifiability QA *cheap* rather than aspirational

QA-M1 and QA-M2 (embedder/model swap, only re-embed) require two
properties:

1. The new embedder drops in without touching extract/chunk/index code.
   This is impossible unless the domain owns a stable contract the new
   embedder implements — i.e. an outbound port.
2. The cache key encodes *which* embedder produced what, so changing the
   embedder invalidates only embeddings. This requires provenance
   metadata flowing through a typed boundary — i.e. the same outbound
   port carrying the adapter's identity.

Both properties depend on the domain controlling the contract. If the
runner imports OpenAI directly, there is no contract to swap behind, the
cache-key formula in F-5.1 has nothing principled to hash on, and the
provenance recorded in F-M.6 becomes either redundant or incorrect.
Hexagonal is what gives the cache-key formula and the metadata model
their teeth.

### 4. The fault-tolerance driver gets the fallback chain almost for free

QA-F2 / F-1.6 require: on a recoverable failure of (say) the GPU
embedder, retry on a CPU embedder. Under Hexagonal, this is a tiny
composite adapter:

```rust
struct FallbackEmbedder<P, F> { primary: P, fallback: F }
impl<P: Embedder, F: Embedder> Embedder for FallbackEmbedder<P, F> { ... }
```

The runner holds a `dyn Embedder` (or a generic `E: Embedder`) and
doesn't know fallback is happening. No new infrastructure, no event bus,
no circuit breaker. The pattern absorbs the requirement.

In a non-Hexagonal world (the runner calls OpenAI directly), retry-with-
fallback becomes a sprawl of `match`/`?` propagation duplicated at every
call site. Hexagonal compresses it to a single combinator.

### 5. The operability driver composes cleanly via multiple inbound adapters

QA-O1/O2 require multiple kinds of caller — CLI, MCP server, and a
future REST frontend (C-4) — to drive the same domain logic. Under
Hexagonal, each is just a different inbound adapter calling the same
inbound ports. None of them knows about the others.

This is why C-4's "design must not preclude a future REST frontend"
constraint is satisfied trivially rather than aspirationally — the
design *already* admits arbitrary inbound adapters, by virtue of the
pattern.

## What would have to be true for Hexagonal to be the wrong choice

Honest test. The pattern would be a poor fit if:

- We had only one embedder and only one extractor forever — no swap
  variation, no pattern justification. We don't; experimentation is the
  point.
- Extraction, chunking, and embedding were tightly coupled (e.g. an
  end-to-end neural pipeline that owns its own tokenisation) — no clean
  port could be drawn. They aren't; each stage is genuinely separable.
- We needed sub-millisecond per-call latency and trait-object dispatch
  was a real cost. We don't; performance is a concern, not a driver.
- The domain were a thin shell over a database (CRUD app) — Hexagonal
  would be over-engineering. Ours isn't; the domain has real behaviour
  (cache keying, provenance, fault-boundary semantics, the runner's
  serialisation contract).

None apply. The conditions under which Hexagonal would be ceremony are
absent here.

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| **Over-application: ports for everything.** | Aniche's rule is project policy, stated in this ADR. Pipeline runner, manifest writer, supervisor, and CLI argument parsing get no ports. If a port would have a single implementation, don't write it. |
| **Anaemic domain: logic drains into adapters.** | The runner — fault boundary, fallback handling, cache lookup, provenance assembly — lives in the domain crate. Adapters do exactly one thing each: speak to one external system. |
| **Naming-as-decoration: folder structure without dependency enforcement.** | The Cargo workspace will enforce the dependency rule: the `librarian-domain` crate has no dependency on `librarian-adapters`. The compiler will reject violations. |
| **DDD baggage creep.** | This ADR explicitly excludes it. If a future ADR proposes aggregates, value objects, or ubiquitous-language ceremony, that ADR must justify the addition against Aniche's rule and the pragmatic stance recorded here. |

## Consequences

**Good.**
- Every QA driver is carried by a property the pattern provides natively
  (modifiability via adapter swap, fault tolerance via composite
  adapters, operability via multiple inbound adapters).
- The metadata model (ADR-0001) and language choice (ADR-0003) align
  with the pattern: typed payloads enforced at compile time, traits as
  ports, single-binary deploy per collection.
- Tests can stub every outbound port. `cargo test` needs no Qdrant, no
  network, no filesystem beyond temp dirs.

**Costs.**
- Adapter authors carry the contract weight: every concrete embedder,
  extractor, chunker, indexer must implement the trait honestly,
  including provenance reporting.
- The runner's domain logic is non-trivial (fault boundary, fallback
  chain orchestration, cache lookup, manifest writes). This is real
  code, not a thin wrapper.
- Trait-object dispatch (or generic monomorphisation) has a small cost
  vs direct calls. Acceptable; performance is a concern, not a driver.

## Further reading (in the local library)

- **Effective Software Testing (Aniche), §7.5.6** — the source of the
  pragmatic "more than one implementation" rule. Read first.
- **Microservices Patterns (Richardson), Chapter 2**, sections "The
  Layered Architectural Style" and "About the Hexagonal Architecture
  Style" — clearest case for dependency-direction inversion.
- **Clean Architecture (Martin), Part V "The Clean Architecture"** and
  **Part VI "Ports and Adapters"** — synthesis and lineage.
- **POSA1, Chapter 2, Pipes and Filters, Consequences subsection** —
  the "error handling is the Achilles' heel" passage that motivates the
  per-document fault-boundary tactic in the runner.
