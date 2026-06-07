# Topic 38: Per-user / per-cohort usage analytics and personalization signals

## Findings
Production search/RAG systems attach a stable `user_id` (and derived cohort tags) to every query trace so cost, latency, and quality can be sliced by user, session, cohort, model, and prompt version (Langfuse, LangSmith). Cohorts solve cold-start: when individual history is sparse, signals from similar users (role, team, intent cluster) fill the gap (Microsoft Research; Elastic cohort-aware ranking). Glean derives cohort/personalization signals from org structure — role, department, team co-authorship, interaction history — layered on top of hybrid retrieval, and self-tunes ranking from sparse click feedback. Raw thumbs up/down is too coarse alone; mature loops triangulate explicit feedback with implicit signals and automated metrics to localize faults (567-labs; apxml).

## What to log
- Stable `user_id` + cohort tags (role, dept, team, intent cluster) on every trace/session.
- Query, retrieved doc IDs + scores, rank positions, final answer, prompt/model version.
- Explicit feedback: thumbs up/down, edits, per-aspect (retrieval vs generation) ratings.
- Implicit signals: clicks + click position, dwell time, abandonment, query reformulations, follow-ups, copy/paste actions.
- LLM-as-judge scores on sampled queries; latency, cost, zero-result flag per session.

## Metrics
- Ranking: MRR (first-relevant), NDCG@K (graded full-list), recall@K.
- Behavioral: zero-result rate, session abandonment rate, CTR + click position, query reformulation rate, session success (dwell-based).
- Per-cohort retention/engagement curves; quality/cost/latency broken down by cohort.
- RAG: faithfulness, answer relevancy tracked per cohort over time (drift detection).

## How it is used
Feedback closes a loop: correlate downvotes with low retrieval precision to localize the fault (corpus gap, embedding/chunking, re-ranking, or generation), then fix that component. Per-cohort click feedback retrains self-tuning rankers (Glean) or tunes per-cohort boost weights (Elastic, ~0.05–0.20, BM25 dominant). Cohort-based A/B tests validate changes: offline MRR/NDCG predict, online CTR/abandonment confirm (Spotify). Alerts fire on per-cohort metric degradation to catch silent drift. Caveats: position bias must be debiased before clicks become LTR labels; feedback-givers are unrepresentative.

## Sources
- https://www.microsoft.com/en-us/research/publication/cohort-modeling-enhanced-personalized-search/
- https://www.elastic.co/search-labs/blog/ecommerce-search-relevance-cohort-aware-ranking-elasticsearch
- https://langfuse.com/docs/metrics/overview
- https://www.langchain.com/langsmith/observability
- https://www.glean.com/blog/enterprise-search-is-hard-why-its-so-behind-and-what-itll-take-to-catch-up
- https://567-labs.github.io/systematically-improving-rag/workshops/chapter3-1/
- https://apxml.com/courses/optimizing-rag-for-production/chapter-6-advanced-rag-evaluation-monitoring/user-feedback-rag-improvement
- https://www.evidentlyai.com/ranking-metrics/ndcg-metric
- https://en.wikipedia.org/wiki/Evaluation_measures_(information_retrieval)
- https://arxiv.org/pdf/1711.02927
