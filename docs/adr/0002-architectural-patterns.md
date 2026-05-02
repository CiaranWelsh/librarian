# ADR-0002 — Architectural patterns

**Status:** Proposed · 2026-05-02
**Context:** Pattern selection given the ASRs (FRs that back the QA drivers) defined in `docs/requirements.md` §3 and §4.

## Context

`librarian` has three QA drivers: **Modifiability** (embedder/model swap),
**Fault tolerance** (per-document isolation with retry/fallback), and
**Operability** (multi-collection fleet on one host, remote clients).

We want the smallest pattern set that carries every ASR, with patterns chosen
deliberately rather than reached for out of habit. Where a pattern doesn't
quite fit, we prefer modifying with a tactic over adding a new pattern.

## Decision

Two patterns. Four tactics layered onto one of them. No more.

### Pattern 1 — Protocol-based stage adapters

A small fixed set of Python protocols defines the framework's plug points:
`Extractor`, `Chunker`, `Embedder`, `Indexer`. The domain (`Document`,
`Chunk`, `ChunkPayload`, the pipeline runner) depends only on these
protocols. Concrete adapters implement them and are wired in via
configuration.

This is the *idea* behind Hexagonal Architecture / Ports and Adapters
(Cockburn) and Clean Architecture (Martin) — the domain depends on
abstractions; infrastructure depends on the domain. We adopt the structural
discipline without the surrounding ceremony:

- **No ports for one-implementation dependencies.** Following Aniche's
  pragmatic rule (EST §7.5.6): create a protocol only where there is genuinely
  more than one implementation. The pipeline runner, manifest writer, and
  CLI entry point do not get protocols.
- **No DDD baggage.** No aggregates, no value objects ceremony, no
  "ubiquitous language" naming demands. Plain Python dataclasses for
  domain types.
- **No "Hexagonal Architecture" label in the codebase.** The structure is
  the structure; the label is folklore.

Justification by ASR:
- Modifiability — every embedder/model swap is a new `Embedder` adapter +
  one config line (QA-M1, QA-M2, F-2.3, F-3.3, F-3.4).
- Update without orphans — the `Indexer` protocol can require a "delete by
  source_id then upsert" contract, satisfied by every concrete indexer
  (F-1.8).
- Testability concern — every adapter is stubbable in-memory; `pytest`
  needs no Qdrant and no internet.

### Pattern 2 — Client-Server (runtime topology)

Across hosts: one Qdrant + one MCP server *per collection*, all running on
Turbo. Clients (Claude Code on Mac, future laptops) connect over Tailscale.
A small fleet registry on Turbo answers "which collections exist, on which
ports, running or stopped".

Justification by ASR:
- Operability — multi-collection fleet on one host (QA-O1, F-9.1–9.4).
- Single source of truth — no local replicas; all clients see the
  canonical Qdrant (QA-O2, F-4.1).
- Snapshot-driven backup — Qdrant's native snapshots, written to NAS,
  rolling retention (QA-O3, F-7.3).

The DSAP (§3.5.2.3) modifiability bonus — clients and server evolve
independently as long as the MCP API stays stable — is a free side benefit.

### Tactics layered on Pattern 1

| Tactic | Mechanism | ASRs carried |
|---|---|---|
| Sequential pipeline orchestrator | The runner inside the domain calls `extract → chunk → embed → index` in order, one Document at a time | F-1.5, F-2.1 |
| Content-addressed cache keying | Cache key = `sha256(source_hash ∥ stage_name ∥ stage_version ∥ config_hash)`; provenance recorded on every chunk matches the keys | QA-M1, QA-M2, F-1.4, F-1.7, F-M.6 |
| Fallback adapter chain | An outbound port may declare a fallback adapter; on recoverable failure the runner retries on the fallback | QA-F2, F-1.6 |
| Per-document fault boundary | Errors thrown by any stage are caught at the Document boundary; the manifest records the failure; the runner moves on to the next Document | QA-F1, F-1.3 |

### Pattern-to-ASR coverage

| ASR / QA | Pattern 1 (+ tactics) | Pattern 2 |
|---|---|---|
| QA-M1 / M2 modifiability | ✓ adapter surface + cache-keying | — |
| QA-F1 fault isolation | ✓ per-doc boundary tactic | — |
| QA-F2 retry / fallback | ✓ fallback-chain tactic | — |
| QA-O1 / O2 / O3 operability | — | ✓ |
| F-1.7 incremental add | ✓ cache-keying tactic | — |
| F-1.8 update no orphans | ✓ indexer port contract | — |
| F-1.3 / F-1.5 per-doc, serial | ✓ runner tactics | — |
| Testability concern | ✓ adapter surface | — |

Every ASR is carried by at least one pattern; every pattern carries
multiple ASRs.

## Patterns we considered and rejected

| Pattern | Why not |
|---|---|
| **Layered architecture** | Wrong axis of variation. Our axis is *stage* (pipeline) and *modality* (adapter), not abstraction level. Layered's well-documented drawbacks (single presentation/persistence layer, wrong-way-round dependencies — MSP §2.2) apply to us. |
| **Microservices** | Massive overkill. One process per *collection* is the right granularity; one per stage or per document would be ceremony. |
| **Event-driven / pub-sub** | No async or fan-out in v1 (F-1.5 is serial). Adds infrastructure with no QA payoff. |
| **Circuit Breaker** | Designed to prevent cascading failure across long-running networked services. We have one user, batch ingestion, no cascading risk. Bounded retry + fallback adapter is sufficient (DSAP §3.4.2.5 is explicit about when this pattern is warranted). |
| **Pipes and Filters** (as a top-level pattern) | The structure is real but lives *inside* the hexagon as the runner's orchestration shape — not a separate pattern in our case. Demoted to the "sequential pipeline orchestrator" tactic. |
| **Process Pairs / forward error recovery** (SAIP §4) | Availability is not a driver. |

## Consequences

**Good.**
- Two patterns, four tactics — small enough to keep in working memory.
- Most ASRs (Modifiability, Fault tolerance, all FRs around ingest CRUD) are carried by Pattern 1; Operability is carried by Pattern 2. Clean split.
- Every concrete design decision downstream (module decomposition, runtime view, ADR-3+) traces back to one of these six elements.

**Costs.**
- Every Embedder, Extractor, Chunker, Indexer adapter must implement the protocol cleanly. Adapter authors carry the contract weight.
- The fallback-chain tactic adds complexity to the runner: it must distinguish recoverable from terminal failures, record both attempts, and surface the right manifest status. This is real code, not free.
- The fleet registry (Pattern 2) is a small but real piece of operational software — config, port allocation, health checks. Worth its weight only because Operability is a driver.

## Further reading

The lineage of Pattern 1 is documented across several books in the local
library. Specific locations:

- **Clean Architecture (Robert C. Martin)** — Part V "Architecture",
  §"The Clean Architecture" (the synthesis) and Part VI "Details",
  §"Ports and Adapters" (the relationship to Cockburn's original).
  Most readable single source.
- **Microservices Patterns (Chris Richardson)** — Chapter 2,
  §"About the hexagonal architecture style" and the preceding
  §"The layered architectural style" (which makes the case against
  Layered, by contrast). Good for understanding *why* the dependency
  direction matters.
- **Effective Software Testing (Maurício Aniche)** — Chapter 7
  "Designing for testability", §7.5.6 "The Hexagonal Architecture and
  mocks as a design technique". This is the source of the pragmatic
  rule we adopt ("ports only where there's more than one
  implementation"). Read this one if you read only one.

The lineage of Pattern 2 (Client-Server) is in:

- **SAIP (3rd ed.)** — Chapter 8 "Modifiability", §"Client-Server Pattern".
- **DSAP** — Chapter 3 "Making Design Decisions", §3.5.2.3 "Client-Server Pattern".
  Concise; covers tradeoffs.

The pattern critique (P&F's error-handling Achilles' heel, motivating our
per-doc fault-boundary tactic) is in:

- **POSA1** — Chapter 2 "Architectural Patterns", §"Pipes and Filters",
  Consequences subsection ("Error handling. As we explained in step 5 of
  the Implementation section, error handling is the Achilles' heel of the
  Pipes and Filters pattern."). Read this *before* implementing the
  runner — it's the trap to avoid.