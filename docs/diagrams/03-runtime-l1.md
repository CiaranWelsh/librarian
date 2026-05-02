# L1 Runtime View (C&C) — `librarian`

**Status:** Draft · 2026-05-02
**View:** Component-and-connector, level 1 (one collection process, hexagonal shape).
**Notation:** Per DSA Ch 4 — boxes are components (runtime instances); arrows are connectors labelled by mechanism. The "hexagonal" framing puts the domain runner at the centre, inbound adapters above, outbound adapters around, external systems beyond the hexagon's edge.

## Scope

This view shows the runtime structure of a *single* collection process. Two process types share this shape, differing only in which inbound adapter is active:

- **CLI process** (short-lived) — instantiates the runner, runs an ingest/remove/snapshot action, exits.
- **Server process** (long-lived) — instantiates the runner, handles MCP requests until stopped.

Cross-process / multi-host topology is the Client-Server view (separate document).

**Supervisor process (separate, third type).** A long-lived process on Turbo. Its runtime is trivial: it reads/writes the fleet registry (SQLite) and spawns / waits on / signals collection-server child processes via OS calls (`fork`/`exec`, `kill`, exit-status polling). No domain logic, no adapters in the hexagonal sense — just a registry component and an OS-process-management component. Drawn here as a paragraph rather than its own diagram because there is nothing the diagram would show that this paragraph doesn't.

## Diagram

```mermaid
flowchart TB
    %% Inbound adapters (driving side)
    subgraph IN[" Inbound adapters (driving side) "]
        CLI{{"CLI handler<br/>(active in CLI process)"}}
        MCP{{"MCP request handler<br/>(active in server process)"}}
    end

    %% Domain — the inside of the hexagon
    RUNNER(["**Domain core**<br/>· Pipeline Runner (ingest):<br/>&nbsp;&nbsp;per-doc fault boundary,<br/>&nbsp;&nbsp;fallback chain, cache lookup,<br/>&nbsp;&nbsp;provenance, manifest writes<br/>· Snapshot orchestrator:<br/>&nbsp;&nbsp;retention policy, manifest record<br/>· Query operations (search, list,<br/>&nbsp;&nbsp;scoped extract)"])

    %% Outbound adapters (driven side)
    subgraph OUT[" Outbound adapters (driven side) "]
        direction LR
        EXT["Extractor"]
        CHK["Chunker"]
        EMB["Embedder<br/>(may wrap fallback chain)"]
        IDX["Indexer"]
        CACHE["Cache"]
        MAN["Manifest store"]
        SNAP["Snapshotter<br/>(snapshot · restore · prune)"]
    end

    %% External systems (beyond the hexagon)
    QDRANT[("Qdrant<br/>(separate process,<br/>same host)")]
    EMB_API[("Embedding service<br/>(network)")]
    FS_LOCAL[("Local disk<br/>(cache)")]
    NAS_FS[("NAS<br/>(snapshot backup)")]
    SQLITE[("SQLite file<br/>(manifest)")]
    SRC[("Source tree<br/>(read-only files)")]

    %% Inbound → Domain
    CLI -->|"sync call<br/>(ingest, remove,<br/>snapshot, restore)"| RUNNER
    MCP -->|"sync call<br/>(query)"| RUNNER

    %% Domain → Snapshotter (snapshot/restore path)
    RUNNER -->|"trait call"| SNAP

    %% Domain → Outbound (trait dispatch)
    RUNNER -->|"trait call"| EXT
    RUNNER -->|"trait call"| CHK
    RUNNER -->|"trait call"| EMB
    RUNNER -->|"trait call"| IDX
    RUNNER -->|"trait call"| CACHE
    RUNNER -->|"trait call"| MAN

    %% Outbound → External
    EXT -->|"file read"| SRC
    IDX -->|"gRPC / HTTP<br/>(F-1.8: delete-by-source-id<br/>then upsert, atomic)"| QDRANT
    EMB -->|"HTTPS"| EMB_API
    CACHE -->|"file I/O"| FS_LOCAL
    MAN -->|"SQL"| SQLITE
    SNAP -->|"HTTP<br/>(snapshot API)"| QDRANT
    SNAP -->|"HTTPS / SCP<br/>(push, prune)"| NAS_FS

    classDef domain fill:#222,color:#fff,stroke:#000,stroke-width:3px;
    classDef external fill:#eee,stroke:#666,stroke-dasharray:3 3;
    class RUNNER domain;
    class QDRANT,EMB_API,FS_LOCAL,NAS_FS,SQLITE,SRC external;
```

## Component catalogue

| Component | Kind | Notes |
|---|---|---|
| **Pipeline Runner** | Domain object (no I/O of its own) | Owns the control flow: fetches cache, calls stages, writes manifest, catches per-doc errors. Trait-dispatches all I/O. |
| **CLI handler** | Inbound adapter | Active only in the short-lived CLI process. Parses args, instantiates runner + adapters, calls runner, reports exit. |
| **MCP request handler** | Inbound adapter | Active only in the long-lived server process. Receives JSON-RPC requests, calls into runner's query side (search / list / scoped extract), returns results. |
| **Extractor / Chunker / Embedder / Indexer / Cache / Manifest store** | Outbound adapters | Concrete implementations of the domain's traits. Each speaks one external protocol; the runner sees only the trait. |
| **Snapshotter** | Outbound adapter | Triggers Qdrant's native snapshot API and writes the resulting file to NAS; applies rolling retention. Invoked by the domain's snapshot orchestrator (not directly by inbound adapters — that would bypass the hexagon). Snapshot/restore acts on collections wholesale, not on documents, so the pipeline runner is not involved. |

## Connector catalogue

| Connector | Mechanism | Notes |
|---|---|---|
| Inbound → Runner | Synchronous in-process function call | Same address space, same thread. |
| Runner → Outbound adapter | Synchronous trait dispatch (static or dynamic) | All in-process. The runner is unaware of the adapter's underlying protocol. |
| Embedder → embedding API | HTTPS request/response | The fallback combinator — when present — is itself an `Embedder`, so retry logic is invisible to the runner. |
| Indexer → Qdrant | gRPC or HTTP (whichever the Qdrant Rust client uses) | Same host as the collection process for v1. |
| Cache → local disk | File I/O | Content-addressed reads/writes on the ingest host's local filesystem. |
| Snapshotter → NAS | HTTPS / SCP one-shot push | Snapshot files written to NAS for backup; old snapshots pruned per retention policy. No mount; no continuous I/O. |
| Manifest store → SQLite | Local SQL queries | One file per collection. |
| Extractor → source tree | File read | Read-only. |

## Properties (runtime)

- **Concurrency.** v1 runs serially per collection process (F-1.5). Multiple collection processes coexist on the host (F-9.x); each has its own runner, adapters, manifest, and Qdrant collection target.
- **Failure domain.** A panic in any adapter is caught by the runner's per-document fault boundary (QA-F1) and recorded in the manifest. The process survives.
- **State.** The runner is stateless across documents; all persistence lives in the cache and manifest. The MCP server holds a Qdrant client connection but no other long-lived state.

## Rules

- Outbound adapters do not call each other. All composition happens in the runner.
- Inbound and outbound adapters never share a runtime object. The runner is the only meeting point.
- The same code (one Cargo workspace, one build) produces both process types. The active inbound adapter is selected at startup, not at compile time.
