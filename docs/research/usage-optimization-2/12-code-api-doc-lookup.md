# Coding-Assistant Documentation Retrieval: API-Lookup Patterns, Retrieval-vs-Parametric Trade-offs, and Task-Conditioned Usage Strategies

**Sources:** RustEvo² (arXiv:2503.16922); CloudAPIBench / DAG++ — "On Mitigating Code LLM Hallucinations with API Documentation" (arXiv:2407.09726); CodeRAG-Bench (arXiv:2406.14497); CodeRAG-Bigraph (arXiv:2504.10046); APIHulBench/MARIN (arXiv:2505.05057); Cursor docs (`@Docs`, codebase-indexing); GitHub Copilot docs/blog (Spaces, retrieval upgrade, CLI `/research`); Context7 MCP (Upstash; `resolve-library-id` / `get-library-docs`); MindStudio "Is RAG dead for coding agents". Figures are quoted from these.

## 1. The core finding: for coding, retrieval beats parametric knowledge precisely where the model is *stale or sparse*

Coding has the cleanest published evidence for *when* docs retrieval beats in-weights knowledge, because correctness is executable and the failure mode (a wrong function name, parameter, or import) is unambiguous. The signal is recency/frequency, not difficulty:

- **RustEvo²** (evolving Rust APIs): no-API-info baseline **34.9%** accuracy → **48.4%** with doc-RAG → **57.7%** with ground-truth API injected. RAG closes **~60% of the gap** between no-info and perfect-info; the headline lift is **+13.5%** absolute, entirely on post-cutoff/changed APIs.
- **CloudAPIBench** (AWS/Azure): GPT-4o makes only **38.58%** valid invocations on *low-frequency* APIs. Documentation-Augmented Generation (DAG) raises that to **47.94%** — but with a *sub-optimal retriever it drops high-frequency-API performance by 39.02% absolute*. The fix (DAG++) is **confidence-thresholded, selective retrieval**: only retrieve when API-invocation confidence is low / index-lookup says the API is rare.
- **APIHulBench/MARIN**: plain RAG is beaten by structure-aware retrieval — MARIN cut hallucinated-element count (MiHN) **−67.5%** and hallucination rate (MaHR) **−73.6%** vs. plain RAG, and **+107%** exact-match.
- **CodeRAG-Bench**: retrieval "dramatically benefits weaker models or those untuned for specific scenarios" and **less-frequent libraries**, with smaller/compounded returns for strong models on common tasks. CodeRAG-Bigraph adds **+40.9 / +37.8 Pass@1** (GPT-4o / Gemini-Pro) at repo level.

**Implication for the librarian:** retrieval's value is conditional on the query landing *outside* the model's well-trodden parametric region. A definition of a standard pattern (well-covered in training) gains little; a rare API, a version-specific signature, or a corpus-specific detector convention gains a lot — exactly our particle-physics/detector niche.

## 2. The retrieval-can-hurt warning is the strongest task-conditioning argument

CloudAPIBench's −39% on common APIs, plus the documented **"context dominance"** failure (LLMs adopt incorrect retrieved statements even when their parametric answer was right), means *unconditional* retrieval is a net negative on some tasks. This is the empirical basis for **selective / confidence-gated retrieval** rather than always-on RAG. It maps directly onto our abstention contract: the same confidence label that gates *abstention* should also gate *whether to retrieve at all* and *whether to trust the chunk over priors*.

## 3. How production coding assistants inject documentation context

Three distinct patterns, each a design point for task-conditioned usage:

| Tool | Mechanism | Granularity / control |
|---|---|---|
| **Cursor** | `@Docs` over pre-crawled + user-added doc indexes; codebase indexed as vector-embedded chunks (functions/classes), synced ~5 min; semantic search **fused with grep** | Explicit user `@`-mention OR an **Explore subagent** in its own context window that runs *many parallel searches with a fast model* and returns only findings. Hybrid (semantic+grep) = **+12.5%** accuracy over grep alone, gain largest on 1,000+-file repos |
| **GitHub Copilot** | `@github` participant; **Spaces** (curated code+docs+specs per task); custom instructions / prompt files / memory; CLI `/research` agent fans out over repo+web with citations | Sept-2025 retrieval upgrade: **+37.6% retrieval quality, 8× smaller index, 2× throughput** |
| **Context7 (MCP)** | Two-tool design: `resolve-library-id` (name → versioned ID, ranked by trust score + coverage) then `get-library-docs` (ID + topic filter → chunks, default **5000 tokens**, configurable) | Ingests `llms.txt`/markdown/OpenAPI; **version-pinned**; 33k+ libraries. Explicitly framed as a *staleness fix* — "function calls that don't exist, deprecated patterns" |

The common architecture is **resolve-then-fetch with a topic filter and a token budget** (Context7) or **broad-search-in-a-subagent then return-only-findings** (Cursor Explore). Both isolate retrieval cost from the main context — a strong analogue for how the librarian CLI should behave as a *tool the assistant calls*, not a pipeline.

## 4. Usage strategy: shallow, selective, structured — the opposite of literature synthesis

Coding/API lookup is the *low-search-count, high-precision* end of the task spectrum:

- **Breadth vs depth:** shallow. The unit of work is one correct call; the right move is a **single resolve-then-fetch with a tight topic filter**, not multi-hop breadth. CloudAPIBench's "retrieve only when confidence is low" is a per-call gate, not a research loop.
- **Refinement trajectory:** *structured before semantic*. The "is RAG dead?" synthesis converges on **match strategy to data type** — exact API/symbol lookups want deterministic lookup (function-calling, index/grep, OpenAPI), semantic vector search is for "where is X / how do I" prose questions, and only heterogeneous mixed sources justify a uniform vector layer. ARKS/CodeAgent reformulate queries iteratively *only* when the first structured lookup fails.
- **Stopping rule:** confidence/validity-based and aggressive. Stop as soon as a high-relevance chunk pins the signature; do **not** accumulate. Extra context here *dilutes* (mirrors PaperQA2's "5 summaries beat 15" and Round-1's k=8 value point).

## 5. Actionable implications for task-conditioned librarian experiments

1. **Make retrieval *conditional*, not default, for lookup-type queries.** Replicate DAG++: gate librarian retrieval on a parametric-confidence / rarity signal. Hypothesis: for API/definition lookups, always-on retrieval underperforms confidence-gated retrieval (the −39% common-API regression is the risk to measure). This is the single most transferable coding result.
2. **Sweep depth by task with coding as the shallow anchor.** Set coding/maths-lookup to **single-search, hard-stop, k small (≈5–8)** and contrast against synthesis tasks (broad-retrieve k=20, multi-search). Use coding as the calibration floor for "how few searches is enough."
3. **Add a structured/exact-match path alongside semantic search.** Breadcrumbs already give us structure; expose a deterministic "find this symbol/section" lookup (grep/breadcrumb filter) and fuse it with dense search — Cursor's hybrid gave **+12.5%**, largest on big corpora. Test fused vs. pure-semantic per task.
4. **Pin "version"/edition like Context7 pins library versions.** For a detector/method that differs across editions or papers, resolve-then-fetch with an explicit source filter beats broad top-k. Experiment: source-pinned retrieval vs. unfiltered, measured by wrong-edition/wrong-paper error rate.
5. **Quantify the parametric-vs-retrieval crossover on our corpus.** Build a RustEvo²-style three-arm eval (no-context / librarian-RAG / ground-truth-chunk-injected) over corpus-specific facts. The gap RAG closes (~60% in Rust) tells us where the librarian *adds* value vs. where the model already knows — the literal definition of task-conditioned usefulness.
6. **Trust calibration as a first-class metric.** Track context-dominance directly: rate of cases where a retrieved chunk *flipped a correct parametric answer to wrong*. The abstention contract handled the no-knowledge case (12%→0%); the next failure mode coding research exposes is *bad-retrieval-overriding-good-priors*, which only shows up under task conditioning.
