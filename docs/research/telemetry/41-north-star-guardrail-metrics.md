# Topic 41: North-star and guardrail metrics for search/RAG products

## Findings
A north-star metric (NSM) captures the core user value (e.g. successful searches / search-to-conversion); ranking-quality wins are treated as *input* metrics feeding it. Guardrail (counter) metrics are non-negotiable boundaries that catch gaming and regressions while you push the NSM. In search/IR, the classic success signal is the SAT/long click (dwell >= 30s) and "1 - abandonment". The dominant guardrail is latency: bounce probability rises ~32% as load goes 1s->3s. RAG products inherit this and add quality guardrails (faithfulness, groundedness, hallucination rate) since an LLM returns HTTP 200 even when wrong. Online evaluation (A/B, interleaving, CUPED) is how candidate changes are validated; interleaving gives Airbnb ~50x sensitivity, Bing's CUPED ~50% variance reduction.

## What to log
- Per-query: query text/embedding, retrieved chunk IDs + scores, ranks, final answer, model/index/prompt versions.
- Per-stage spans: retrieval latency, LLM latency, total latency, context length, token/cost.
- Outcome signals: clicks + dwell (SAT vs short), zero/null-result events, query reformulations, abandonment, session boundaries.
- Explicit feedback (thumbs up/down) joined to the trace; LLM-as-judge scores on sampled traffic.

## Metrics
- NSM: successful sessions / SAT-click rate; input = NDCG, MRR, CTR@1.
- IR online: CTR, abandonment rate, reformulation rate, time-to-first-click.
- RAG quality: faithfulness, context precision/recall, answer relevance (RAG Triad).
- Guardrails: p95 latency, zero-result rate, hallucination/harmful-result rate, cost/query, CSAT/NPS.

## How it is used
Closed feedback loop: production traces + thumbs-down mine failure queries -> add as eval test cases (regression suite) -> validate fixes offline, then online via interleaving (fast candidate screen) and A/B (business-metric confirmation), gated by guardrail thresholds before ship. CUPED/variance-reduction shortens experiments. Guardrails auto-alert and block releases that breach latency, quality, or harm bounds.

## Sources
- Eppo, Counter (guardrail) metrics: https://www.geteppo.com/blog/counter-metrics
- Meta metrics practice (NSM + guardrail spam example): https://www.jasonshen.com/154/
- Langfuse, RAG Observability and Evals (spans, feedback loop): https://langfuse.com/blog/2025-10-28-rag-observability-and-evals
- LLMOps observability comparison (Langfuse/LangSmith/Arize): https://medium.com/@kanerika/llmops-observability-langsmith-vs-arize-vs-langfuse-vs-w-b-f1baeabd1bbf
- Counterfactual estimation of click metrics (arXiv 1403.1891): https://arxiv.org/pdf/1403.1891
- SAT click / dwell as success (knowledge-gain study, arXiv 1805.00823): https://arxiv.org/pdf/1805.00823
- Airbnb interleaving + counterfactual eval (arXiv 2508.00751, KDD 2025): https://arxiv.org/abs/2508.00751
- Effective Online Evaluation for Web Search (Yandex OEC/sensitivity): https://www.researchgate.net/publication/334582161_Effective_Online_Evaluation_for_Web_Search
