# Topic 46: Failure Taxonomy and Error Categorization for RAG from Logs

## Findings
Practitioners build a *domain-specific* failure taxonomy two ways. (1) **Top-down**: the Barnett et al. "Seven Failure Points" paper (CAIN 2024) names where in the pipeline failures occur — missing content, missed top-ranked docs, not in consolidated context, not extracted, wrong format, incorrect specificity, incomplete. (2) **Bottom-up**: Langfuse promotes qualitative-research "open coding" — read a trace sample, write free-text notes on the first thing that broke, cluster notes (an LLM can draft the taxonomy) into named categories, then label and measure. The TruLens "RAG Triad" localizes failures to a pipeline stage: low **context relevance** = retrieval fault, low **groundedness** = generation/hallucination, low **answer relevance** = right facts, wrong intent. IR search-log practice adds query-side categories: zero/low-result queries, vocabulary mismatch, reformulation, and good-vs-bad abandonment.

## What to log
- Per-trace spans: query, retrieved chunk IDs + scores, consolidated context, prompt, generated answer, latency, error status (retriever-type spans distinct from generation).
- Free-text "first failure" annotation + assigned taxonomy label per sampled trace.
- Tags/metadata: chain_type, user_segment, source IDs cited.
- Query-side signals: zero/low result count, query reformulation chains, abandonment (with click/dwell to distinguish good abandonment), thumbs feedback.

## Metrics
- Failure rate per taxonomy category (the headline chart).
- RAG Triad scores: context relevance, groundedness/faithfulness, answer relevance (context precision/recall too).
- Zero-result rate, reformulation rate, abandonment rate (good vs bad), per-query-frequency segmentation.
- Error rate / latency thresholds for alerting.

## How it is used
For each category decide: one-time prompt/code fix (fix-once bugs), build a recurring evaluator (faithfulness/relevance LLM-judge), or monitor. Failing traces are added to a regression dataset to verify fixes without regressions. Hybrid scoring: cheap heuristics on 100% of traffic, LLM-judge on a 10-20% sample, periodic human relabel to refresh ground truth. Threshold alerts (e.g. >5% error over 5 min) notify via Slack/webhook. Zero-result/long-tail queries are prioritized by frequency; overlap test separates index/synonym bugs from assortment gaps.

## Sources
- Seven Failure Points (Barnett et al., CAIN 2024): https://arxiv.org/abs/2401.05856
- Langfuse error analysis: https://langfuse.com/blog/2025-08-29-error-analysis-to-evaluate-llm-applications
- Langfuse RAG observability & evals: https://langfuse.com/blog/2025-10-28-rag-observability-and-evals
- TruLens RAG Triad: https://www.trulens.org/getting_started/core_concepts/rag_triad/
- LangSmith debugging / root-cause: https://apxml.com/courses/langchain-production-llm/chapter-5-evaluation-monitoring-observability/langsmith-debugging
- Tunkelang, "Making Sense of Null and Low Results": https://dtunkelang.medium.com/making-sense-of-null-and-low-results-a077f37bf8fc
- Detecting Good Abandonment in Mobile Search (Microsoft): https://www.microsoft.com/en-us/research/wp-content/uploads/2017/05/williams_www2016_good_abandonment.pdf
