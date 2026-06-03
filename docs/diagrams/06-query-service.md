# 06 — Query service (runtime C&C view)

Component-and-connector view of the query-side evolution (PoC → production). Components
are runtime units (processes/services); connectors are labelled with protocol + style.
Requirements: `docs/requirements.md` Addendum (2026-06-03) — **F-Q.1 / F-Q.2 / QA-Q1**.

## L1 — components & connectors

```mermaid
flowchart LR
  classDef ext fill:#eee,stroke:#999,stroke-dasharray:4 3;
  classDef svc fill:#dde8f7,stroke:#2b6cb0;
  classDef cli fill:#e8f7dd,stroke:#2f855a;

  claude["«client»<br/>Claude Code"]:::ext
  term["«client»<br/>user shell"]:::ext

  subgraph hosts["client host(s) — Mac / laptops"]
    mcp["«process»<br/>MCP adapter<br/>(thin)"]:::cli
    cli["«process»<br/>librarian query (CLI)<br/>(thin)"]:::cli
  end

  subgraph turbo["turbo — source of truth"]
    daemon["«service · long-running»<br/>query-daemon<br/>axum · stateless"]:::svc
    qdrant[("«service»<br/>Qdrant<br/>all collections")]:::svc
  end
  openai["«external service»<br/>OpenAI Embeddings"]:::ext

  claude -- "MCP (JSON-RPC) · req/reply" --> mcp
  term  -- "exec · stdout" --> cli
  cli   -- "HTTP/JSON · req/reply" --> daemon
  mcp   -- "HTTP/JSON · req/reply" --> daemon
  daemon -- "HTTPS REST · req/reply (semaphore-bounded)" --> openai
  daemon -- "gRPC :6334 · req/reply" --> qdrant
```

## L2 — inside query-daemon (one process, async runtime)

```mermaid
flowchart TB
  classDef ext fill:#eee,stroke:#999,stroke-dasharray:4 3;

  subgraph daemon["query-daemon — single process"]
    http["HTTP listener (axum)<br/>port :PORT"]
    handlers["request handlers<br/>concurrent async tasks<br/>/v1/search · /v1/documents · /v1/extract"]
    core["query-core (library)<br/>search · list_documents · get_extract"]
    sem(["embed semaphore<br/>N in-flight"])
    embc["embedder client (async)"]
    qc["qdrant client (async)"]

    http --> handlers --> core
    core -- "embed (vector-search path)" --> sem --> embc
    core -- "search / scroll" --> qc
  end

  embc -. "HTTPS" .-> openai["OpenAI Embeddings"]:::ext
  qc   -. "gRPC :6334" .-> qdrant[("Qdrant")]:::ext
```

## Legend & notes
- **«client»** external actor process · **«process»** thin client we ship · **«service»**
  long-running server · **«external service»** third-party.
- **Connectors** are request/reply (synchronous). The only bounded one is daemon→OpenAI
  (the `embed` semaphore) — the shared-key rate limit is the contention point at QA-Q1's
  "tens of users", not the daemon itself.
- **Stateless:** handlers hold no per-request/session state; shared `Arc<embedder>` +
  `Arc<qdrant_client>` only → concurrency = the axum/tokio worker pool (QA-Q1).
- **Two access paths in `query-core`:** vector-search (embed → search) and metadata-scroll
  (documents/extract, no embed — skips the semaphore).
- **Ingest is out of view** (CLI-on-turbo, unchanged).
