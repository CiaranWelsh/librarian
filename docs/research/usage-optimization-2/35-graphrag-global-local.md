# GraphRAG Global vs Local Query Modes: Task-Conditioned Retrieval for the Librarian

**Scope.** Round 1 settled single-query mechanics (verbatim query, k=20 best / k=8 value point,
quote-first generation, abstention contract). This note imports the most directly relevant external
evidence for the *task-conditioned* question: which kinds of work need **corpus-global summarization**
(reason over large portions of the corpus) versus **local chunk retrieval** (answer lives in a few
nearby passages), and what the measured cost/quality trade-offs are. GraphRAG is the canonical study
of this split, and its benchmarks give us numbers — though the librarian today is a *pure local*
system (top-k chunks, no community graph), so the lesson is mostly about **routing and stopping**, not
about adopting a graph index.

## The local/global split is a real, measurable axis — not a continuum we can ignore

Microsoft's GraphRAG draws the foundational line: **local queries** are answered "using a small number
of text regions, sometimes even a single region," and resemble the query in embedding space, so they
are retrieved as nearest neighbours; **global queries** "require reasoning over large portions of or
even the entirety of a dataset" and concern qualities "not explicitly stated in the text" — e.g. "What
are the main themes?" ([Edge et al., *From Local to Global*, arXiv 2404.16130](https://arxiv.org/abs/2404.16130);
[GraphRAG query overview](https://microsoft.github.io/graphrag/query/overview/)). The mechanism that
makes vector RAG *fail* on global queries is precisely top-k similarity: the answer to a thematic
question is not lexically near the question, so the right passages are never retrieved. This is the
single most important reason the librarian's top-k design will systematically underperform on
synthesis-shaped questions, regardless of k.

BenchmarkQED formalises this into a usable **2x2 taxonomy** (AutoQ): a *scope* axis (local vs global)
crossed with a *source* axis (activity vs data), yielding data-local, data-global, activity-local,
activity-global, plus data-linked multi-hop local queries ([BenchmarkQED, Microsoft Research](https://www.microsoft.com/en-us/research/blog/benchmarkqed-automated-benchmarking-of-rag-systems/)).
This is the right frame for a librarian usage study: classify each real query into the spectrum and
condition strategy on it, rather than treating all questions as the same retrieval problem.

## Measured results: global summarization wins on synthesis, loses badly on cost and on lookup

- **Synthesis questions: global beats local-style RAG.** On two ~1–1.7M-token corpora, GraphRAG global
  conditions beat vector RAG on **comprehensiveness (72–83% win rate, p<.001)** and **diversity of
  perspectives (62–82%)** in LLM-judged pairwise comparison; an independent claim count corroborated
  this (31–34 claims/response vs 25–26 for vector RAG) ([arXiv 2404.16130](https://arxiv.org/abs/2404.16130)).
- **Lookup questions: local/vector wins, or graph actively hurts.** Semantic search beat global
  conditions on specific lookup queries in the same paper. The independent "When to use Graphs in RAG"
  benchmark (1,018 college-level CS questions over 20 textbooks — close to our software corpus) found
  graph augmentation can *reduce* accuracy on multiple-choice/fact questions because "retrieval noise
  can interfere with the model's decision-making" — the model already knew the answer and the extra
  context distracted it ([Han et al., arXiv 2506.05690](https://arxiv.org/abs/2506.05690)). Graphs help
  on "complex reasoning and contextual summarization"; the gap "narrows for simple fact retrieval where
  vector search alone is sufficient." A parallel systematic study agrees: "RAG is consistently effective
  for single-hop, detail-oriented queries... GraphRAG is more advantageous for multi-hop,
  reasoning-intensive QA" ([RAG vs GraphRAG, arXiv 2502.11371](https://arxiv.org/abs/2502.11371)).
- **The cost gap is enormous and structural.** Average tokens per query: **879 (vector) vs 38,707
  (graph-local) vs 331,375 (graph-global)** on a novel; similar on a medical corpus — roughly **350–375x**
  ([arXiv 2506.05690](https://arxiv.org/abs/2506.05690)). Worse, prompt size *grows* with task
  difficulty (7,800 -> 40,000 tokens), and "excessive token accumulation often introduces redundant
  information, which in turn degrades context relevance" — i.e. on hard tasks the cost buys *negative*
  quality. This is the same "more is not better / lost in the middle" failure seen in
  [30-redundancy-diversity-sources.md], now at the whole-corpus scale.

## The fix is routing and hybrid modes, not "always go global"

- **Query-complexity routing.** Adaptive-RAG trains a small (T5-Large) classifier to route each query to
  *no retrieval / single-step / multi-step*, matching always-retrieve accuracy while cutting cost; the
  Oracle (perfect routing) bounds the ceiling ([Jeong et al., NAACL 2024, arXiv 2403.14403](https://arxiv.org/abs/2403.14403)).
  The lesson for the librarian: the *number and depth of searches should be conditioned on a cheap
  up-front complexity judgement*, not fixed.
- **Hybrid global+local beats either alone on mixed queries.** DRIFT search seeds a local search with
  community context (broad start -> local refinement) and reportedly beats standard local search by
  15–25% on queries needing both breadth and depth ([DRIFT, Microsoft Research](https://www.microsoft.com/en-us/research/blog/introducing-drift-search-combining-global-and-local-search-methods-to-improve-quality-and-efficiency/)).
  Dynamic community selection cuts global-search token cost **77%** using a cheap rater model
  (GPT-4o-mini) to prune before the expensive map-reduce, with quality maintained
  ([Microsoft Research](https://www.microsoft.com/en-us/research/blog/graphrag-improving-global-search-via-dynamic-community-selection/)).
  Both confirm: breadth is best obtained by a *cheap broad pass that steers a focused deep pass*, not by
  retrieving everything.

## Actionable implications for librarian usage experiments

1. **Add a query-class label as the first experimental variable.** Tag each real librarian query with the
   AutoQ 2x2 (data/activity x local/global) plus "multi-hop." Hypothesis: pure top-k is near-optimal for
   data-local lookup (definition, threshold, API, single fact) and degrades monotonically toward
   data-global synthesis ("what are the main approaches to X across the corpus"). Measure win-rate vs a
   stronger baseline *per class*, not pooled.
2. **Simulate "global" without a graph index via map-reduce over many local retrievals.** The librarian
   has no community summaries, but a thematic question can be approximated by issuing several
   complementary verbatim sub-queries, deduplicating by breadcrumb root, then summarizing. Test this
   "poor-man's global" against single top-k for synthesis tasks; the GraphRAG result predicts large
   comprehensiveness/diversity gains here and *only* here.
3. **Route on a cheap complexity judgement, set per-class search budgets.** Following Adaptive-RAG: for
   data-local, 1 search at k=8 and stop; for multi-hop, 2–4 chained searches; for global synthesis, a
   broad fan-out (Round 1's k=20) then summarize. Report accuracy *and* tokens/searches per class to find
   the librarian's own value points.
4. **Watch for the "graph/breadth hurts lookup" failure on our own corpus.** The CS-textbook benchmark
   showed extra context *lowering* accuracy on questions the model already knew. For factual/MC-style
   software questions, test whether broader retrieval underperforms a tight k=8 (or no-retrieval) —
   this directly extends Round 1's abstention contract into a "don't over-retrieve" contract.
5. **Map task type -> mode.** Working hypothesis from the evidence: *coding API lookup, maths definitions,
   single-fact science* -> local, shallow, stop early. *Literature synthesis, "compare the approaches",
   "what's the consensus", survey/writing framing* -> broad fan-out + summarize. *Multi-hop reasoning*
   (derive X from facts in 2+ sources) -> chained local searches. Measure each cell.
6. **Use a cheap broad pass to steer a focused deep pass.** DRIFT and dynamic community selection both
   show this beats brute breadth. For the librarian: one wide retrieval to surface candidate breadcrumb
   regions, then targeted re-queries into the promising regions — and budget the deep pass, since gains
   saturate (consistent with the self-consistency saturation in [30-redundancy-diversity-sources.md]).

**Caveats.** GraphRAG's win-rates are LLM-judged on podcast/news corpora (1–1.7M tokens) and the cost
figures come from graph systems the librarian does not run, so the *direction* of every finding
transfers but the *magnitudes* do not. The most transferable, corpus-matched evidence is the
CS-textbook benchmark (arXiv 2506.05690): graphs/breadth help reasoning and summarization, can hurt
fact lookup. Treat the 350x cost and 77% pruning numbers as motivation for routing, not as targets to
reproduce on our pure-local stack.
