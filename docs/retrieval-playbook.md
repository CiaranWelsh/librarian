# Retrieval-quality playbook (librarian)

Synthesis of established RAG practice (5-agent web sweep, 2026-06-04) mapped to librarian:
qdrant + OpenAI `text-embedding-3-large` (3072-d, cosine, dense only), daemon with `search`
(top-k dense + `content_type` filter) and `extract` (contiguous chunk-index window). Goal:
**adopt what works, don't reinvent.** Each item is tagged `[query-usage]` (no code), `[daemon]`
(code/index), or `[ingest]` (re-index), with effort, the headline result, and a source.

Two wins already found empirically: questions beat keywords; locate→extract beats raising k.
Our fixed-window `extract` is the *primitive form* of "sentence-window / parent-document" retrieval.

## The pipeline (where the levers are)

`query transform → chunk (ingest) → embed (ingest) → retrieve + rank → assemble context → measure`

## 0. Do this FIRST — a measurement harness `[daemon, low effort]`

You can't tune what you can't measure; this makes every change below evidence-based.
- **MVP:** label ~50 `(question → relevant chunk-id(s))`, compute **hit-rate@k** and **MRR**
  over the daemon's `search` output — deterministic, no LLM, runs in the test harness.
- hit-rate@k = "is the right chunk even retrieved" (catches recall regressions from chunking/hybrid);
  **MRR** = "how high it ranks" (the number that moves when reranking / query-rewrite helps).
- Bootstrap labels with an LLM (or LlamaIndex `generate_question_context_pairs`), hand-correct (~1–2 h).
  Loop: baseline → change ONE knob → re-run the same 50 → compare. Defer nDCG (needs graded labels)
  and RAGAS/LLM-judge (for unlabeled-scale + retrieval-vs-generation fault localisation).
- Sources: LlamaIndex RetrieverEvaluator (hit_rate/mrr); Weaviate metrics blog; RAGAS (arXiv:2309.15217).

## Highest-leverage upgrades (sequenced)

| # | Technique | Stage | What / why | Headline | Source |
|---|---|---|---|---|---|
| 1 | **Question-phrasing + Query2doc / step-back** | `[query-usage]` | Phrase as questions (done). Optionally: client-side LLM writes a hypothetical answer and we embed *that* (Query2doc/HyDE), or "step back" to the general principle for conceptual queries. Closes the question→document gap. | Query2doc +3–15% on dense; step-back +7–27% on conceptual QA | arXiv:2303.07678, 2310.06117 |
| 2 | **Structure-aware chunking + heading-path metadata** | `[ingest]` | The DIRECT fix for `### Integration Testing` becoming a body-less chunk: two-stage split (header → recursive *within* section), keep heading WITH body (`strip_headers=False`), **merge sub-minimum chunks up**, and prepend `Book > Chapter > Section` to the embedded text. Start at **400 tokens, 10–20% overlap**, sized in tokens to the embedder. | Chroma: 88–89% recall@400 vs 85%@800 on our exact model | LangChain MarkdownHeaderTextSplitter; Chroma chunking eval; Unstructured `by_title` |
| 3 | **Hybrid search (dense + sparse BM25/BM42) + RRF** | `[daemon, qdrant-native]` | Add a sparse named vector; fuse dense+sparse with Reciprocal Rank Fusion via qdrant's Query API (`prefetch` + `{"rrf":{}}`). Restores exact-term recall (API names, error codes, equations, theorem names) that dense embeddings blur. One-time sparse re-index. | RRF is the robust default fusion; BM42 is qdrant's short-chunk baseline | qdrant sparse-vectors / bm42 / hybrid-queries docs |
| 4 | **Anthropic Contextual Retrieval** | `[ingest, LLM]` | LLM writes a 50–100-token "situating" blurb per chunk, prepended before embedding (+ contextual BM25). Fixes anaphora / missing scope ("this detector", "the above equation") in our books+papers. We already have Claude. | **−35% / −49% / −67%** retrieval failures (embed / +BM25 / +rerank); ≈$1/M doc-tokens with caching | anthropic.com/news/contextual-retrieval |
| 5 | **Parent-document / small-to-big extract** | `[daemon/index]` | Replace the fixed ±N window: attach `parent_id` + parent span per chunk at ingest; `extract` returns the *semantic parent* (section), not a fixed count. Snaps to natural boundaries, de-dups overlapping hits, separates match-unit (small) from read-unit (large). | The principled version of what we already do | LangChain ParentDocumentRetriever; LlamaIndex auto-merging |
| 6 | **Reranking (qdrant ColBERT multivector, or local bge-reranker)** | `[daemon]` | Retrieve wide (top-50–100), jointly re-score query–doc pairs down to top-5. Biggest *precision* lever; fixes "right chunk retrieved but ranked 12th." Native in qdrant via ColBERT multivector, or a local ONNX cross-encoder. Layer on last. | LlamaIndex: cut downstream latency 28s→4s, kept accuracy | qdrant fastembed rerankers; Cohere rerank |

Deferred / blocked: **Late chunking** and **Contextual Document Embeddings** (need token-level or
trained embeddings — blocked by the OpenAI embedding API); **proposition indexing**, **MMR**,
**relevance feedback**, **query decomposition/multi-query** (situational, revisit after the above).

## Recommended path (efficient sequencing)

1. **Eval harness** (measure — the enabler).
2. **Query-usage patterns** in `cw:librarian` (Query2doc / step-back) — free, measure on the harness.
3. **Structure-aware chunking + heading-path** (re-ingest) — fixes the fragmentation root cause.
4. **Hybrid + RRF** (qdrant-native ranking foundation; our corpus is identifier-heavy).
5. **Contextual Retrieval** (re-ingest; the best-measured single win).
6. **Parent-document `extract`**, then **reranking** — assemble + precision, last.

Rationale: measure first so each step is evidence-based; the two ingest changes (3, 5) bound
everything downstream and fix the fragmentation observed in real use; hybrid+RRF and reranking
are qdrant-native; query-usage is zero-code. Each is an ADR/issue-sized chunk of work.

## Measured on our corpus (2026-06-04 exploratory experiments)

Against the live `software` collection (qdrant **1.17.1**, 408,614 pts, dense `text` vector only — **no sparse**):
- **Chunk size: median 156 chars, p25 52; ~36% low-value** (junk 0.8% · headings 14.6% · tiny <80ch 20.4%; usable 64%). Field standard is ~400 tokens (≈1,500–2,000 chars) — **we are ~10× smaller than standard.** Junk (single chars, `}`) is rare (0.8%), concentrated in code-heavy Rust books (code-block over-splitting).
- **`chunk_index` is per-source contiguous** (Effective Software Testing Ch.1 = 0..213) → parent-document / window `extract` is viable as-is.
- Question-phrasing reduces heading-rate in top-5 (DI 100→40%, CAP 80→40%, Rust 40→0%, confirmed). Search latency ~0.3–1.8s (query-embed-bound) → query-time LLM tricks roughly double it. Dense whiffs on some exact technical terms (BM25, RRF scored 0.47/0.51) → mild hybrid justification.
- Prose chunks are mostly self-contained (only mild anaphora) → **Anthropic Contextual Retrieval is NOT our bottleneck; demoted from the generic #1.**

### Methodology: adopt the standard, deviate only with evidence
We diverge from the field in two concrete, sub-standard ways: (1) a bespoke blank-line chunker yielding ~10×-too-small, heading-orphaning chunks; (2) dense-only retrieval where the standard is hybrid. The fix is to **adopt** the field-standard approaches, not bespoke-tune ours — experiments serve to *diagnose divergence* and (via the eval harness) *validate the standard wins on our data*. Burden of proof is on deviating, not adopting (avoids the local optimum that plateaus below the field).

### Revised sequence for OUR corpus
1. **Eval harness** (~50 labeled Q→chunk-id, hit-rate@k + MRR) — enables evidence-based adoption.
2. **Adopt a standard chunker** (recursive + markdown/structure-aware + min-size merge, ~400 tok / 10–20% overlap) replacing the blank-line chunker → re-ingest → validate it beats current. **Top lever for us.**
3. **Hybrid dense+sparse + RRF** (qdrant-native; we're currently dense-only).
4. **Parent-document `extract`** (contiguity confirmed) + **reranking**.
5. ~~Contextual Retrieval~~ — demoted; our chunks aren't anaphora-poor.

## Sources (fetched)
- Anthropic Contextual Retrieval — https://www.anthropic.com/news/contextual-retrieval
- HyDE arXiv:2212.10496 · Query2doc arXiv:2303.07678 · Step-back arXiv:2310.06117
- qdrant: hybrid-queries, sparse-vectors, bm42, search-relevance, fastembed-rerankers (qdrant.tech)
- Chunking: Chroma chunking eval (trychroma.com), LangChain splitter docs, Unstructured.io chunking, Pinecone chunking
- Context assembly: LangChain ParentDocumentRetriever, LlamaIndex auto-merging/sentence-window, Late Chunking arXiv:2409.04701
- Eval: LlamaIndex RetrieverEvaluator, Weaviate retrieval metrics, RAGAS (arXiv:2309.15217), TruLens RAG triad, LangSmith RAG eval
- RRF: Cormack et al. SIGIR 2009
