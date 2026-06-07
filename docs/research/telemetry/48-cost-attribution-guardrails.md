# 48 — Cost Attribution and Budget Guardrails per User/Query

## Findings
Production practice splits into two layers. (1) **Attribution** is done at the *observability* layer (Langfuse, Braintrust, Traceloop, Datadog) by tagging every request/trace with identity metadata, then rolling spend up by user/feature/tenant. (2) **Enforcement** is done at the *gateway/proxy* layer (LiteLLM, Portkey) because observability tools "can monitor but cannot reject requests that exceed limits" (Langfuse). The dominant failure mode is **runaway spend** — agent loops with no per-session ceiling: documented $47K (264h LangChain loop) and $48K (14h GPT-4o research-agent) incidents both had firing alerts but no in-loop kill switch. Lesson: "observability without enforcement is a dashboard, not a control." For RAG, the meaningful unit is **cost per *successful* answer**, not cost per query, and over-retrieval (3–8x more chunks than needed) is a common silent cost.

## What to log
- Identity tags propagated across all spans: `user_id`, `customer_id`/tenant, `feature`, `agent_run_id`, `session_id`, `prompt_version`, `model`, `deployment`.
- Token fields per span: `prompt_tokens`, `completion_tokens`, plus separate `prompt_cached_tokens` (reads) and `prompt_cache_creation_tokens` (writes); reasoning tokens explicitly (Langfuse cannot infer o1-style cost).
- Per-stage RAG cost fingerprint: embedding, vector-query, rerank, generation cost each.
- Iterations/tool-calls/retries per session; success/value signal for the answer.

## Metrics
- Cost per user per day; cost per feature request; cost per agent run (median + p99); cost per customer (B2B margin); **cost per successful eval/answer**.
- Avg iterations per session by intent bucket (>4 signals poor retrieval).
- 7-day moving average + spike detection; Isolation Forest / Prophet for anomaly/forecast.

## How it is used
- Tiered budgets: monthly cap **plus** daily guardrail (~10–15% of monthly); hard limits reject (`spend >= max_budget`), soft limits alert (Slack). Calibrate from 2–4 weeks baseline.
- Hierarchical caps (org > team > key > user/end-user) enforced at proxy via Redis-shared counters; multiple independent reset windows (1d/7d/30d).
- In-loop budget gate + circuit breaker + per-session threshold (e.g. $1/session) + timeouts kill runaway trajectories before the next call.
- Quality gate before shipping cost-cuts: eval pass-rate held, latency in budget, cost-per-successful-eval improves. Routing/semantic caching feed back (≈85% cost cut in one benchmark).

## Sources
- https://langfuse.com/docs/observability/features/token-and-cost-tracking
- https://langfuse.com/docs/observability/features/users
- https://www.braintrust.dev/articles/how-to-track-llm-costs-2026
- https://www.traceloop.com/blog/from-bills-to-budgets-how-to-track-llm-token-usage-and-cost-per-user
- https://docs.litellm.ai/docs/proxy/users
- https://docs.litellm.ai/docs/proxy/rate_limit_tiers
- https://apxml.com/courses/optimizing-rag-for-production/chapter-5-cost-optimization-production-rag/rag-cost-anomaly-monitoring
- https://towardsdatascience.com/rag-is-burning-money-i-built-a-cost-control-layer-to-fix-it/
- https://futureagi.com/glossary/runaway-cost/
- https://costhawk.ai/glossary/token-budget
