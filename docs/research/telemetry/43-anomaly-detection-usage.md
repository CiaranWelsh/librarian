# 43 - Anomaly Detection in Usage and Quality Time Series

## Findings
Production practice splits anomaly detection into two phases: (1) decompose a metric to its regular part (trend + seasonality) and isolate residuals, then (2) score residuals statistically and filter alerts to suppress noise (eBay's Moving Metric system, OpenSearch). Decomposition uses STL or, for robustness against contamination, a median-based trend (S-ESD / RobustSTL); residuals are scored with robust z-score (median + MAD, scaled 0.6745 / 1.4826) rather than mean/stddev, because the median tolerates up to 50% anomalous points. OpenSearch's managed detector uses Random Cut Forest; Datadog Watchdog flags relative spikes vs a historical baseline. For LLM/RAG, the same machinery is applied to *quality* signals: LLM-as-judge scores (relevance, faithfulness) become time series, with alerts on dropping rolling averages and on embedding/semantic drift (Arize Phoenix) when user queries move into topics the corpus does not cover.

## What to log
- Per-query: timestamp, query text/embedding, latency, result count, top score, click/landing vs orphan (no result used).
- Quality: LLM-as-judge relevance/faithfulness score, abstention flag, hallucination/toxicity flag.
- Volume/error: request rate, error rate, cost/tokens, per-stage (retrieve vs generate) latency.
- Embeddings of incoming queries for drift clustering.

## Metrics
- STL residual; robust modified z-score = 0.6745*(x-median)/MAD; thresholds z = 3 to 3.5 in production.
- EWMA / CUSUM control charts for gradual shifts.
- Embedding drift (distance of query distribution from corpus).
- Detection quality scored with IR metrics: precision, recall, F1.

## How it is used
Alerts fire on sustained breaches (e.g. avg relevance < 4.0 over 10 min), ranked by severity to limit alert fatigue. Embedding-drift alerts reveal coverage gaps that trigger corpus additions; quality drops trigger eval/regression datasets so the same failure is tested next time (monitor -> observe trace -> evaluate -> curate dataset loop).

## Sources
- https://arxiv.org/pdf/2004.02360 (eBay Moving Metric Detection and Alerting)
- https://docs.opensearch.org/latest/observing-your-data/ad/index/ (OpenSearch RCF spike/dip)
- https://www.datadoghq.com/blog/accelerate-incident-investigations-with-log-anomaly-detection/ (Watchdog relative spikes)
- https://thirdeyedata.ai/data-ai-industry-insights/anomaly-detection-with-robust-zscore (robust z-score / MAD)
- https://www.traceloop.com/blog/how-to-automate-alerts-for-llm-performance-degradation (static + baseline alerts on judge scores)
- https://www.examcert.app/blog/ai-observability-langfuse-arize-phoenix-2026/ (Phoenix embedding/RAG drift)
- https://arxiv.org/pdf/1606.05978 (M3A web-search query-log anomalies)
