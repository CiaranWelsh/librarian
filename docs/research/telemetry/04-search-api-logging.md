# Topic 04: Request Logging Best Practices for Search/Query APIs

## Findings
Practitioners log **whole sessions**, not isolated queries, because query-reformulation
chains carry relevance signal (a reformulated follow-up implies the first results
underperformed). Clicks are **informative but position-biased**, so raw clicks are treated
as *relative* preferences, not absolute judgments, and corrected via propensity estimation.
LLM/RAG-observability tools (Langfuse, LangSmith) converge on a nested **trace -> span ->
generation** schema: a trace = one request, with an explicit `retriever` span wrapping the
query (input) and retrieved chunks + scores (output). Both attach `user_id`/`session_id`/
tags/metadata that propagate to all spans, making every field a filter dimension. Single
metrics get gamed (e.g. zero-results pages backfilled with popular items), so teams combine
offline ranking metrics with online behavioral ones.

## What to log
- Query text, normalized/expanded query, query embedding; timestamp, session_id, user_id, tags.
- Results shown (impressions): top-K doc ids, similarity/relevance scores, ranks, retrieval latency.
- Retrieved chunk content (truncated) and source metadata; chunking config (size/overlap), embedding model + time.
- Click events with position; dwell time (task-dependent threshold), scroll, hover, text-selection.
- Reformulations within the session; zero-result / abandonment flags; final answer, tokens, cost.

## Metrics
- Offline ranking: NDCG (0-1, graded + position), MRR (first relevant rank), MAP.
- Online behavioral: CTR (balanced vs bounce/dwell), session abandonment rate, query-reformulation/re-query rate (lower better), zero-results rate (ZRR), query coverage.
- RAG/LLM-judge: retrieval relevance (per-chunk, averaged), faithfulness, groundedness, answer correctness; plus latency P50/P99, error rate, cost.

## How it is used
- Component-level scores isolate retrieval vs generation: low relevance -> tune chunking/embedding/top-K; low faithfulness -> stronger grounding/prompt.
- Reformulation chains and click co-occurrence mine implicit relevance pairs to train learning-to-rank and fix spelling/synonyms.
- ZRR and abandonment surface catalog/index gaps; A/B tests on the same traffic compare ranking changes; dashboards + alerts catch regressions.

## Sources
- https://langfuse.com/blog/2025-10-28-rag-observability-and-evals
- https://langfuse.com/docs/observability/data-model
- https://docs.langchain.com/langsmith/observability
- https://www.meilisearch.com/blog/search-relevance-metrics
- https://www.algolia.com/blog/engineering/a-b-testing-metrics-evaluating-the-best-metrics-for-your-search
- https://en.wikipedia.org/wiki/Evaluation_measures_(information_retrieval)
- https://dl.acm.org/doi/10.1145/1229179.1229181 (Joachims et al., clicks biased but relative preferences accurate)
- https://arxiv.org/pdf/cs/0605035 (Query Chains: learning to rank from implicit feedback)
- https://dev.to/missamarakay/why-i-love-zero-result-search-as-a-metric-and-you-should-too-3efj
