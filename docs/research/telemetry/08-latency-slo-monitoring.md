# Topic 08: Latency SLOs and Percentile Monitoring for ML Inference / Search Services

## Findings
Latency in inference/search is long-tailed, so practitioners monitor the full
distribution, not the mean. LLM APIs show especially heavy tails (P99/P50 ratios
of ~5-8x vs 2-4x for ordinary web APIs), driven by variable token counts, dynamic
batching, and cold starts. The relationship *between* percentiles diagnoses cause:
flat P50 with spiking P99 means queue contention/bursts; all percentiles elevated
means undersized hardware. In fan-out search ("Tail at Scale", Dean & Barroso),
one slow replica dominates aggregate latency, so tail control via hedged/replicated
requests is the core lever.

## What to log
- Per-request timing split into stages: queue/wait, TTFT (prefill), inter-token
  latency (ITL), end-to-end. Langfuse exposes `langfuse_latency` and
  `langfuse_time_to_first_token`.
- Trace/span hierarchy (Langfuse traces->observations; LangSmith "runs") timing each
  retrieval, rerank, tool call, and generation step.
- Request features that predict latency: input length, output length, model id,
  concurrency, cache hit/miss, per-host TTFT.
- Queue depth / requests waiting vs running (e.g. vLLM `num_requests_waiting`).
- Dimensions for slicing: model, prompt version, user, session, feature, geography.

## Metrics
- Percentiles P50/P90/P95/P99(/P99.9) per stage, from histograms
  (`histogram_quantile(0.99, ...)`).
- P99/P50 tail ratio; divergence alert "P99 > 3xP50 for 15m".
- SLO attainment % and error-budget burn rate.
- Throughput (tokens/s), error rate, cost per slice.

## How it is used
- Set P95 as primary SLO (stable); P99 for high-stakes paths; P50 for regressions.
- Drive Google-SRE multiwindow multi-burn-rate alerts (14.4x@1h/5m page; 6x@6h/30m;
  ticket on slow burn) off latency histograms.
- Feed tail diagnosis into fixes: hedged requests, caching/precompute, async
  isolation/circuit breakers, headroom/load-balancing. Adaptive (TTFT-calibrated)
  hedging cut P99 ~74% at ~20% overhead.

## Sources
- LLM Inference SLO Engineering (TTFT/ITL/P99 budgets): https://www.spheron.network/blog/llm-inference-slo-ttft-itl-latency-budget-guide-2026/
- P50 vs P95 vs P99 explained: https://oneuptime.com/blog/post/2025-09-15-p50-vs-p95-vs-p99-latency-percentiles/view
- BentoML key metrics for LLM inference: https://bentoml.com/llm/inference-optimization/llm-inference-metrics
- Langfuse metrics overview: https://langfuse.com/docs/metrics/overview
- Langfuse latency/TTFT fields (Mixpanel docs): https://docs.mixpanel.com/docs/tracking-methods/integrations/langfuse
- LangSmith observability (P50/P99 dashboards, alerts): https://www.langchain.com/langsmith/observability
- Google SRE Workbook, Alerting on SLOs (burn rate): https://sre.google/workbook/alerting-on-slos/
- Datadog, burn rate is a better error rate: https://www.datadoghq.com/blog/burn-rate-is-better-error-rate/
- InfoQ, adaptive hedged requests reduce P99 74%: https://www.infoq.com/articles/adaptive-hedged-requests-p99-latency/
- Why tail latency matters for vector search (Zilliz): https://zilliz.com/ai-faq/why-is-tail-latency-p95p99-often-more-important-than-average-latency-for-evaluating-the-performance-of-a-vector-search-in-userfacing-applications
