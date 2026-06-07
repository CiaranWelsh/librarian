# Standard Chunking for RAG — Findings & Recommendation for librarian

*Date: 2026-06-04. Synthesis of a three-agent literature sweep + inspection of our own
chunker. Evidence is cited; the recommendation for librarian is inference built on that
evidence and is marked as such.*

## TL;DR

There **is** a de-facto standard, and we are not following it.

- **The standard chunker** = a *recursive, structure-aware* splitter that keeps the
  largest natural unit (paragraph → sentence → word) intact **up to a size budget**, then
  recurses. Canonically LangChain's `RecursiveCharacterTextSplitter`
  (separators `["\n\n", "\n", " ", ""]`); LlamaIndex's equivalent is `SentenceSplitter`.
- **The standard size** = **~512 tokens (~2000 chars)** with **modest overlap (0–20%)**.
  This is the convergent recommendation across Azure AI Search, Pinecone and Weaviate, and
  the empirical sweet spot in independent evaluations.
- **Our `BlankLineChunker` has the right boundary but no size budget** — it emits one
  chunk per blank-line block and only ever *splits* (never *merges*). Result: median
  ~156 chars (~40 tokens), ~10× too small. **The fix is to pack blocks up to a ~512-token
  target**, not to invent a new algorithm.
- **Skip** semantic chunking, propositions, and (for now) late chunking — the evidence
  says they don't reliably beat a well-sized recursive splitter, and they cost far more.

## 1. What "the standard chunker" is (primary-source evidence)

| Tool | default size | default overlap | unit | separators |
|---|---|---|---|---|
| LangChain `RecursiveCharacterTextSplitter` (base `TextSplitter`) | 4000 | 200 | **characters** | `["\n\n","\n"," ",""]` |
| LlamaIndex `SentenceSplitter` (the default node parser) | 1024 | 200 | **tokens** | `" "`, para `"\n\n\n"` |

The recursive algorithm (LangChain docs, quoted): it "tries to split on them in order until
the chunks are small enough … keep all paragraphs (and then sentences, and then words)
together as long as possible." The unit differs by library — **LangChain counts characters,
LlamaIndex counts tokens** — which matters when configuring ours.

Vendor guidance converges on a *tuned* size rather than the raw library default:
- **Azure AI Search** (verbatim): "start with a chunk size of 512 tokens (~2,000 characters)
  and an initial overlap of 25%." Rule of thumb: **~4 chars/token**.
- **Pinecone** (verbatim): "Fixed-sized chunking will be the best path in most cases, and we
  recommend starting here and iterating only after determining it insufficient." Test band
  128/256 vs 512/1024 tokens.

## 2. What the empirical evidence says about size & overlap

- **Chroma, *Evaluating Chunking Strategies for Retrieval* (Smith & Troynikov, 2024)** —
  token-level eval (Recall / Precision / IoU). `RecursiveCharacterTextSplitter` @ **200
  tokens, zero overlap** was "consistently high performing across all evaluation metrics."
  **Chunk size mattered more than method** (800→200 moved IoU 1.5%→6.9%). **Overlap *hurt***
  token-IoU. The OpenAI default (800/400) was a measurable under-performer.
- **Qu, Tu & Bao, *Is Semantic Chunking Worth the Computational Cost?* (NAACL 2025
  Findings, arXiv:2410.13070)** — "fixed-size chunking remains a more efficient and reliable
  choice for practical RAG applications"; semantic chunking's gains were inconsistent and
  "overshadowed by … the quality of embeddings." (Caveat: used context-free sentence
  embeddings.)
- **Rethinking Chunk Size (arXiv:2505.21700, 2025)** — optimal size is domain-dependent:
  factoid/concise → 64–256 tokens; narrative/contextual → 512–1024. **512 is the defensible
  single starting point.**

## 3. The frontier — what (little) is worth adopting

Ranked verdict from the survey (Agent 3), for *our* corpus (markdown-converted technical
books & papers, OpenAI `text-embedding-3-large`, citation-grounded):

1. **Structure-aware / heading-based chunking — ADOPT.** Largest clean, evidence-backed
   gain on structured docs (**5–10 pts**, Snowflake finance-RAG benchmark); deterministic,
   near-free, **preserves source traceability**.
2. **Parent-document / small-to-big (match small → return the larger parent/window) —
   ADOPT.** Strongest hit-rate/MRR gains (**+10–20%**, LlamaIndex evals). **We already built
   the "return parent" half: `librarian extract`.** Skip the auto-*merge* machinery.
3. **Late chunking (Jina, arXiv:2409.04701) — OPTIONAL.** Real but small (~1–2 nDCG),
   free at inference, *only* on a long-context mean-pooling embedder — which OpenAI's API
   embedder is **not**, so **N/A for us** without changing embedders.
4. **Semantic chunking — SKIP.** 2–4 pt inconsistent gain at ~14× ingest cost.
5. **Propositions / Dense-X (arXiv:2312.06648) — SKIP.** Wins confined to weak retrievers
   on entity-centric Wikipedia factoids; **rewrites source text → destroys traceability**.

## 4. Our current chunker: the diagnosis

`crates/adapter-chunker-blankline/src/lib.rs` (`BlankLineChunker`):

- Splits each span on `"\n\n"` and emits **one chunk per blank-line block** (line 112).
- `max_chars = 20_000` is only an **upper** bound (embedder token limit); a block is
  windowed *only* if it exceeds it. There is **no minimum, no target, and no merging**.
- So a heading line, a one-sentence paragraph, a list item each become a standalone chunk
  → measured median ~156 chars / ~40 tokens, 14.6% heading-only chunks.

The boundary (blank line ≈ paragraph) is fine — it *is* a crude structure-aware split. The
missing piece is the standard recursive splitter's **pack-to-budget** step.

## 5. Recommendation for librarian *(inference, grounded in §1–§4)*

Modify `BlankLineChunker` from *split-only* to **pack-to-target** (keep it simple — a
modification, not a new dynamic-dispatch abstraction; per project rule, no `Box<dyn>`):

1. **Target ~2000 chars (~512 tokens).** Greedily concatenate consecutive blank-line blocks
   until adding the next would exceed the target; emit, then continue. A heading block stays
   with the body that follows it.
2. **Recurse on oversized blocks** as today, but lower the windowing threshold from 20k to
   the target so a single huge block is split to ~512-token windows (not 5k-token ones).
3. **Overlap small (~10%, ~1 block).** Because `extract` reconstructs the window on demand
   (the small-to-big read path), we don't need large baked-in overlap — and Chroma found
   overlap hurts precision. Lean low.
4. **Optional, high-value:** prefix each chunk with its **markdown heading breadcrumb**
   (e.g. `Chapter 9 > Integration Testing`) — the structure-aware metadata that posted the
   biggest clean gain, cheap, and improves both retrieval and citation.

This is the standard recursive/structure-aware splitter at the standard size, expressed as a
minimal change to what we already have. It directly attacks the measured root cause (10×
under-sized chunks) and composes with `extract` as the context-assembly layer.

**Open design decision (for the human):** the exact size target (256 vs 512 tokens — smaller
favors precision since `extract` recovers context; 512 is the safe standard) and whether to
add heading breadcrumbs in this pass or a follow-up.

## 6. Validation plan

We already have the eval harness and a baseline to beat:
- Baseline (current chunker): **hit-rate@10 89%, MRR 0.638, fragment-rate@5 20%**.
- Re-ingest a held-out sample into `eval_*` with the new chunker; re-run `eval/run_eval.py`.
- **Adopt if** fragment-rate@5 drops materially **and** hit-rate@10 / MRR hold or improve.
  (fragment-rate@5 is the metric the chunker swap is designed to move.)

## Sources

- Smith, B. & Troynikov, A. (2024). *Evaluating Chunking Strategies for Retrieval.* Chroma.
  https://research.trychroma.com/evaluating-chunking
- Qu, R., Tu, R. & Bao, F. S. (2024/2025). *Is Semantic Chunking Worth the Computational
  Cost?* Findings of NAACL 2025. arXiv:2410.13070 — PDF in this directory.
- *Rethinking Chunk Size for Long-Document Retrieval* (2025). arXiv:2505.21700.
- Günther et al. (2024). *Late Chunking.* arXiv:2409.04701 — PDF in this directory.
- Chen et al. (2024). *Dense X Retrieval.* EMNLP 2024. arXiv:2312.06648 — PDF in this directory.
- LangChain `RecursiveCharacterTextSplitter`; LlamaIndex `SentenceSplitter` (source code).
- Microsoft Azure AI Search; Pinecone; Weaviate; Snowflake finance-RAG — chunking guidance.
- Our corpus: `Agentic-Design-Patterns/Chapter-14-Knowledge-Retrieval-RAG.pdf` (#12–#16).
</content>
</invoke>
