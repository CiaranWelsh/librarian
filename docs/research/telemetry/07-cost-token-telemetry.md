# Topic 07: Cost and Token Accounting Telemetry per Request/User

## Findings
Production LLM observability tools (Langfuse, LangSmith, Traceloop) attach token/cost
to the smallest unit of work: a `generation`/`embedding` observation inside a trace.
Cost is either **inferred** from `(model, provider) -> per-token price` tables or
**ingested explicitly** from the provider response payload. Attribution to user/tenant/
feature is done by propagating tags (`userId`, `feature_id`, `tenant`) down the trace.
OpenTelemetry now standardizes this under the `gen_ai.*` namespace; FinOps practice adds
per-feature/per-tenant chargeback, budgets, and anomaly alerts on top.

## What to log
- Per generation: `input_tokens`, `output_tokens`, `total_tokens`, plus sub-types
  (`cached_tokens`, `reasoning_tokens`, `audio_tokens`) — sums need not match totals.
- `gen_ai.request.model` / `response.model`, `provider`, prompt version, finish reason.
- Computed cost per call (and which price table version produced it).
- Attribution tags: `user_id`, `tenant`, `feature`/operation-span name, session, release.
- Latency: operation duration, time-to-first-token (streaming).
- Caveat: reasoning models (o1) need ingested token counts — cost cannot be tokenizer-inferred.

## Metrics
- `gen_ai.client.token.usage` (histogram, by `gen_ai.token.type` = input/output).
- Cost per request / per user / per feature / per tenant; cache-hit ratio and savings %.
- Cost per successful answer (unit economics); model routing split (% to cheap vs frontier).
- Rolling daily spend with 7-day baseline for spike detection.

## How it is used
Feedback loop: pair cost with quality so optimizations don't backfire. Identify expensive
features/users -> route easy queries to cheaper models (40-90% savings reported), cache
static prefixes (~90% on hits), trim `max_tokens`/context. Showback weekly for 4-6 weeks,
then chargeback to cost centers. Enforce budgets at a gateway: tiered alerts (50/80/100%),
baseline-relative spike alerts, unauthorized-model alerts, and graceful degradation
(downgrade model at 80% budget) rather than hard 429 kills.

## Sources
- Langfuse Token & Cost Tracking: https://langfuse.com/docs/observability/features/token-and-cost-tracking
- Langfuse User Tracking: https://langfuse.com/docs/observability/features/users
- LangSmith Cost Tracking: https://docs.langchain.com/langsmith/cost-tracking
- LangSmith UsageMetadata: https://reference.langchain.com/python/langsmith/schemas/UsageMetadata
- OTel GenAI metrics: https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-metrics/
- OTel GenAI spans: https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/
- Traceloop cost-per-user: https://www.traceloop.com/blog/from-bills-to-budgets-how-to-track-llm-token-usage-and-cost-per-user
- zop.dev LLM FinOps per-feature budgets: https://zop.dev/resources/blogs/llm-finops-per-feature-token-budget/
- TrueFoundry cost attribution/chargeback: https://www.truefoundry.com/blog/llm-cost-attribution-team-budgets
- Morph LLM cost optimization levers: https://www.morphllm.com/llm-cost-optimization
