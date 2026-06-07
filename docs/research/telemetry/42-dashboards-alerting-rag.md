# Topic 42: Dashboards and Alerting Design for RAG/Search Operations

## Findings
Production teams run a two-layer ops view. Layer 1 is classic SRE telemetry on traces/
spans (Prometheus + Grafana, or the platform's own panels): latency P50/P95/P99, error
and timeout rate, throughput/RPS, token cost. Layer 2 is quality telemetry attached to
the same traces: reference-free judge scores (RAGAS-style faithfulness, answer relevancy,
context relevance) plus retrieval IR metrics (hit rate, precision/recall@k, NDCG) and
behavioural signals (CTR, abandonment, thumbs-down). LangSmith, Langfuse and Arize/Phoenix
ship pre-built per-project dashboards (Phoenix bins retrieval relevance at document /
retriever-span / trace levels) and fire alerts via webhook/PagerDuty when a metric crosses
a threshold. IR/search practice treats NDCG as an SLI: a CI gate computes NDCG with
confidence intervals, and on-call runbooks tie each alert to a triage path (check recent
deploys, feature-store/index health, sample-query debug, rollback). The hardest design
problem is *segmentation* — global averages hide failures, so dashboards slice by query
intent, cohort, and embedding cluster (Phoenix surfaces problematic clusters via UMAP).

## What to log
- Per-query trace: query text + intent/locale, query-rewrite output (spell/synonym/
  expansion), retrieved chunk IDs with relevance scores, assembled context, answer,
  latency breakdown per stage, token cost, request ID + timestamp.
- Operational: error/timeout flags, RPS, queue depth, vector-DB/index health.
- Quality scores written back onto traces (judge faithfulness/relevance, retrieval hit rate).
- Behavioural: zero/low-result flag, CTR/click position, dwell, abandonment, reformulation,
  thumbs up/down. Tag the model/index/prompt version for slicing and deploy-correlation.

## Metrics
- Ops SLIs: P50/P95/P99 latency, error rate, RPS/throughput, cost per query.
- Retrieval: hit rate / zero-result rate, precision/recall@k, NDCG@k (+ confidence interval).
- Generation: faithfulness/groundedness rate, answer relevancy, refusal rate, hallucination rate.
- Engagement: CTR, abandonment/search-success rate, thumbs-down rate.
- Drift: score/latency trend vs. rolling baseline, per-segment deltas.

## How it is used
Dashboards aggregate per-query and per-segment metrics with version annotations so a
regression lines up with the deploy that caused it. Threshold alerts (e.g. "throughput
< 800 RPS for 10 min", "faithfulness below baseline", "NDCG regression in CI") page on-call
or block release; a runbook drives triage and rollback. Set thresholds against a rolling
baseline, not absolutes, to catch drift. NDCG-as-SLI plus CI gating cuts undetected
regressions and on-call noise. Avoid global-only views: alert per intent/cohort/cluster, and
funnel alert-triggered traces into human review and golden-set regression tests, closing the loop.

## Sources
- LangSmith observability (custom dashboards, P50/P99, webhook/PagerDuty alerts): https://www.langchain.com/langsmith/observability
- Langfuse, RAG Observability and Evals: https://langfuse.com/blog/2025-10-28-rag-observability-and-evals
- Arize Phoenix metrics dashboards (retrieval relevance at doc/span/trace, clustering): https://arize.com/docs/phoenix/tracing/llm-traces/metrics
- Arize/Phoenix online evals + threshold alerting: https://arize.com/docs/phoenix/user-guide
- OpenSearch, Measuring and improving search quality (NDCG/MAP/precision dashboards): https://opensearch.org/blog/measuring-and-improving-search-quality-metrics/
- NDCG as SLI, CI gate, incident runbook: https://dataopsschool.com/blog/ndcg/
- Production RAG monitoring (Prometheus/Grafana RPS alert thresholds, alert dimensions): https://dasroot.net/posts/2026/04/production-rag-monitoring-performance-quality/
- EvidentlyAI, RAG evaluation (segmented metrics, baselines): https://www.evidentlyai.com/llm-guide/rag-evaluation
- Glean, debugging enterprise search relevancy (per-incident telemetry, eval baselines): https://www.glean.com/perspectives/how-to-debug-enterprise-search-relevancy-issues
