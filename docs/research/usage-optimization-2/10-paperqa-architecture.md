# PaperQA2: Agentic Gather-Evidence Architecture and What It Implies for Task-Conditioned Librarian Usage

**Source:** Skarlinski et al., "Language agents achieve superhuman synthesis of scientific knowledge," arXiv:2409.13740 (FutureHouse, 2024); FutureHouse engineering blog; `Future-House/paper-qa` codebase defaults. Figures below are quoted from these.

## 1. The agent loop: RAG decomposed into tools

PaperQA2's defining move is to treat retrieval and generation not as a fixed pipeline but as a **multi-step agent task** over three tools the LLM can call in any order: **Paper Search → Gather Evidence → Generate Answer** (plus an optional fourth, **Citation Traversal**). The agent can re-issue searches with refined queries, accumulate evidence into a shared state, and inspect candidate answers before committing. This is the closest published analogue to our question of *how an assistant should drive a retrieval tool over multiple turns*.

Critically, the measured behaviour is **shallow, not deep**: the agent averages only **1.26 ± 0.07 searches per question** and **0.46 ± 0.02 citation traversals per question**. So despite an unbounded loop (`agent.max_timesteps = None`), the optimal trajectory on a hard literature-QA benchmark is roughly *one good search, gather, answer* — refinement is the exception, not the rule. The stopping rule is explicit in the prompt: "Once you have five or more pieces of evidence from multiple sources, or you have tried a few times, call [generate answer]." So the controller's stopping criterion is **evidence-count- and source-diversity-based**, with a try-budget escape hatch.

## 2. Gather Evidence = top-k retrieval + RCS (rerank + contextual summary)

Gather Evidence is two-phase. (a) Embed the query and rank document chunks by dense similarity, keeping **top-k**. (b) Run an "embarrassingly parallel," per-chunk LLM call — **Retrieval-augmented Contextual Summarization (RCS)** — that, given the query + citation metadata + chunk, emits a **relevance score (integer 1–10)** and a **contextual summary (≤ ~300 words / 200–400 tokens)**. Summaries are re-ranked by score before answering. Two effects matter for us:

- **Per-chunk LLM relevance scoring with a query-conditioned summary** is the value-add over raw top-k. It (i) drops irrelevant chunks the embedder surfaced, (ii) compresses a ~2,250-token chunk to 200–400 tokens, vastly improving answer-context density, and (iii) preserves metadata (citation count, journal-quality estimate) into the score. Summarization efficacy was flat across chunk sizes **750–3,000 tokens**.
- The **relevance score is also a control signal**: citation traversal only fires from papers with an RCS summary scoring **≥ 8**. So the LLM-emitted relevance number becomes a *gating threshold* for whether to expand the search — a concrete example of using a confidence/relevance label to drive a refinement decision.

## 3. Parameters: paper vs. shipped defaults (note the divergence)

| Quantity | Paper / best-accuracy | Codebase default |
|---|---|---|
| Dense top-k considered | 30 | — |
| Evidence pieces retrieved (`evidence_k`) | — | **10** |
| Evidence summaries used in answer (`answer_max_sources`) | 15 max; **5 gave highest accuracy** | **5** |
| RCS summary length | ≤ 300 words | "about 100 words" |
| Chunk size | 2,250 tokens, overlap 750 chars | — |
| Papers per search | 12 candidates | `search_count = 8` |

The headline number for us: **best accuracy came from feeding only ~5 evidence summaries to the answer**, not 15 — more retrieved context did not help and the shipped default agrees (`answer_max_sources = 5`). This independently corroborates our Round-1 "k=8 value point": a small, *reranked*, summarized evidence set beats a large raw one.

## 4. Measured superhuman claims (conditions attached)

- **LitQA2:** PaperQA2 precision **85.2 ± 1.1%** vs. human **73.8 ± 9.6%** → "superhuman precision" (t = 3.49, p = 0.0036). Accuracy **66.0 ± 1.2%** vs. human **67.7 ± 11.9%** — **statistically indistinguishable**. So the *superhuman* claim is about **precision (not making unsupported claims)**, not raw recall/accuracy. The human baseline had full internet + tools + unlimited time.
- **WikiCrow summaries:** precision **86.1%** vs. Wikipedia **71.2%**; "cited and unsupported" statements **13.5%** vs. **24.9%**; reasoning errors 12 vs. 26.
- **Contradiction detection:** **2.34 ± 1.99 per paper**, 70% human-validated.
- **Cost:** **\$1–3 per query**; WikiCrow article \$4.48 ± \$1.02, ~492 s.

The consistent through-line: the win is **factual precision via rerank-and-summarize-then-quote**, exactly the quote-first + abstention pattern Round-1 already found.

## 5. Actionable implications for task-conditioned librarian experiments

1. **Add an RCS layer and measure it head-to-head against raw top-k.** Our librarian returns top-k chunks + a confidence label; PaperQA2 shows the decisive lift comes from a *per-chunk LLM relevance score (1–10) + query-conditioned summary* placed before generation. Experiment: for synthesis tasks, compare (raw top-k=20) vs. (top-k=20 → LLM RCS → keep best 5 summaries). Hypothesis: precision rises, token cost falls.
2. **Test the "k=20 retrieve, ~5 answer" split per task.** PaperQA2 retrieves broad (30) but answers narrow (5). Decouple our retrieval-k from our answer-k and sweep them independently *by task*: synthesis/science may want broad-retrieve/narrow-answer; a single API/maths lookup may want k≈5/answer≈3.
3. **Make the confidence label a refinement trigger, not just a display.** PaperQA2 gates expansion on RCS ≥ 8. Define a librarian rule: if top summary's relevance < threshold, *re-search with a refined query*; else answer. Measure whether this beats fixed single-shot for hard/under-specified queries.
4. **Set a task-conditioned stopping rule benchmarked against ~1.26 searches.** Literature QA needed barely more than one search. Test whether synthesis/contradiction-finding tasks justify multi-search/breadth (citation-traversal analogue = "follow the breadcrumb to neighbouring chunks") while lookup/coding tasks should hard-stop at one search to avoid drift.
5. **Use source-diversity + evidence-count as the stop signal.** The prompt stops at "≥5 pieces from multiple sources." For our corpus, encode an analogous rule (e.g., ≥N chunks spanning ≥M distinct books/papers) and test it against a fixed-budget baseline, especially for synthesis where single-source over-reliance is the failure mode.
6. **Report precision separately from accuracy/recall.** PaperQA2's superhuman result is a *precision* result. Our task-conditioned eval should track unsupported-claim rate (precision) distinctly from coverage, since the abstention contract already drove hallucination 12%→0% — the next axis to optimise per task is recall without sacrificing that precision.
