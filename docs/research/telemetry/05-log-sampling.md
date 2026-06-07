# Topic 05: Sampling Strategies for High-Volume Request Logging

## Findings

High-volume observability splits into two decisions: **head sampling** (decide at request start, cheap, low overhead, but blind to outcomes) and **tail sampling** (decide after the request completes, so you can keep errors/slow/rare cases). Production tracers (Datadog, OpenTelemetry, Grafana) combine both: a flat probabilistic floor for representativeness plus biased retention of interesting traces. For a fixed-size, single-pass, constant-memory sample of an unbounded stream, **reservoir sampling** (Vitter Algorithm R, O(1)/item, each item kept with prob k/n) is the standard; weighted (A-ExpJ) and stratified variants let you over-sample errors or specific categories. LLM-observability tools (Langfuse, LangSmith) sample to control cost because each span carries large payloads (prompts, retrieved chunks, completions); a noted hazard is sampling to 0.1% which discards the rare failures observability exists to catch. IR research uses **importance sampling / inverse propensity scoring (IPS)** with clipped/capped estimators to reuse logged click data for unbiased offline evaluation.

## What to log

- Per-request: trace/span IDs, latency, error status/type, service/endpoint, query, result count.
- Sampling metadata: the **sampling decision**, sampling rate applied, and **ingestion reason** tag (`auto`/`error`/`rare`/`manual`) so cost is attributable.
- For importance sampling: the **logging-policy propensity** (probability the shown result/list was chosen) per impression, enabling later unbiased reweighting.
- Full payload only on retained (tail-sampled) traces; counters/RED metrics on 100% of traffic.

## Metrics

- Effective sampling rate and retention rate per reason/service.
- Ingested spans/bytes (cost), reduction ratio (often 90-99%).
- RED (request rate, error rate, duration) computed pre-sampling on 100% of traffic for accuracy.
- Estimator variance / bias for IPS-based offline evaluation; anomaly-retention ratio.

## How it is used

- Tail policies guarantee capture of errors and high-latency outliers for debugging.
- Reservoir/stratified samples feed dashboards and a representative review/eval set.
- Logged propensities + IPS let you A/B-test or re-rank offline without new live traffic (off-policy evaluation).
- Adaptive sampling raises rates on rare/anomalous classes; LLM-as-judge runs on a small sampled fraction to bound eval cost.

## Sources

- Datadog, Mastering Distributed Tracing / efficient sampling: https://www.datadoghq.com/architecture/mastering-distributed-tracing-data-volume-challenges-and-datadogs-approach-to-efficient-sampling/
- OpenTelemetry, Sampling concepts: https://opentelemetry.io/docs/concepts/sampling/
- Logz.io, Sampling in Distributed Tracing guide: https://logz.io/learn/sampling-in-distributed-tracing-guide/
- OneUptime, Log Sampling for High-Volume Systems: https://oneuptime.com/blog/post/2026-01-25-log-sampling-high-volume/view
- Reservoir Sampling (algorithm/variants): https://www.emergentmind.com/topics/reservoir-sampling
- Pydantic, AI Observability Pricing (sampling cost trade-off): https://pydantic.dev/articles/ai-observability-pricing-comparison
- LangSmith observability/eval sampling: https://docs.langchain.com/langsmith/evaluation
- Li et al., Offline Evaluation of Ranking Policies with Click Models (KDD 2018): https://arxiv.org/abs/1804.10488
- Hofmann et al., Probabilistic Method for Inferring Preferences from Clicks (CIKM 2011): http://www.cs.ox.ac.uk/people/shimon.whiteson/pubs/hofmanncikm11.pdf
