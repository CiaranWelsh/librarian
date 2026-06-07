# Topic 30: Query-Log Mining for Content/Corpus Acquisition Decisions

## Findings
Practitioners treat the query log as the primary demand signal for *what to acquire next*. The strongest, most-cited signal is the **zero-result (and zero-click) query**: "direct communication from a user telling you exactly what is absent" (Earley). Beyond raw misses, teams mine **reformulations**, **clicks + dwell**, and (in RAG) **retrieval-miss traces** where the relevant chunk was never fetched. Aggregation matters more than individual queries: log mining clusters queries into topics/subtopics and tasks (Microsoft Research, ACM TOIS) to surface *coverage demand* rather than one-offs. The modern LLM/RAG version (Langfuse + RAGAS) adds reference-free scoring of retrieval relevance, faithfulness, and context completeness, then clusters low-scoring traces to map where the knowledge base is thin. The recurring lesson: analysis only pays off when wired to content owners with authority to act on a regular cadence.

## What to log
- Raw query text + normalized/intent-classified form, with metadata (topic/area, user segment, timestamp).
- Result count per query; **zero-result** and **zero-click** flags.
- Clicked source IDs, rank of click, dwell time on result.
- Reformulation/refinement events within a session (add/remove term, substitution, spelling).
- RAG traces: retrieved chunk IDs + scores, retrieval-hit/miss vs. ground truth, answer faithfulness, explicit user feedback (thumbs, satisfaction).

## Metrics
- Zero-result rate; zero-click rate (segmented, trended weekly).
- Query refinement/reformulation rate; clicks-with-sufficient-dwell (satisfied-click) rate.
- Retrieval hit/miss rate; context recall/completeness; faithfulness/relevance scores.
- Top-N zero-result queries and bottom-N satisfaction items (the prioritized acquisition list).
- Gap-closure velocity: are gaps closing faster than new ones appear (quarterly)?

## How it is used
A cadenced feedback loop: **weekly** dashboards + alerts on zero-result spikes and satisfaction drops; **monthly** review of top-20 zero-result queries (create) and bottom-20 satisfaction (improve); **quarterly** structural assessment. Clustering converts scattered misses into a content-development backlog routed to owners. Glean uses interaction analytics to flag knowledge gaps (e.g., employees failing to find travel policy) and pushes toward predictive gap detection. Tooling closes the loop into evals/tests.

## Sources
- Earley, Mining Search Logs for Content Strategy — https://www.earley.com/insights/mining-search-logs-content-strategy
- Langfuse RAG evaluation guide — https://langfuse.com/guides/cookbook/evaluation_of_rag_with_ragas ; evals roadmap — https://langfuse.com/blog/2025-11-12-evals
- Huang & Efthimiadis, Query Reformulation Strategies (CIKM'09) — https://jeffhuang.com/papers/Reformulation_CIKM09.pdf
- Hassan et al., Query Reformulation as Predictor of Satisfaction (Microsoft, CIKM'13) — https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/Hassan_CIKM13a.pdf
- Hu et al., Mining Query Subtopics from Search Log Data (SIGIR'12) — https://www.microsoft.com/en-us/research/wp-content/uploads/2012/01/fp266-hu.pdf
- Beeferman & Berger, Agglomerative Clustering of a Search Engine Query Log (KDD'00) — https://dl.acm.org/doi/10.1145/347090.347176
- Silvestri, Mining Query Logs (Foundations & Trends in IR) — https://dl.acm.org/doi/abs/10.1561/1500000013
- Glean, Future of Workplace Search — https://www.glean.com/perspectives/the-future-of-workplace-search-how-ai-is-transforming-knowledge-discovery
- SingleGrain, LLM Query Mining — https://www.singlegrain.com/blog-posts/analytics/llm-query-mining-extracting-insights-from-ai-search-questions/
