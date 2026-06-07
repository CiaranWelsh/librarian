# Topic 45: Telemetry for LLM agent tool-use (frequency, success rate, error taxonomy)

## Findings
Practitioners treat each tool call as a first-class, traced operation, distinct
from generic APM because a 200-OK call can still be the wrong tool, wrong
arguments, or a hallucinated result. The emerging standard is OpenTelemetry's
GenAI semantic conventions: an `execute_tool {gen_ai.tool.name}` span (operation
`execute_tool`) carrying `gen_ai.tool.name`, `gen_ai.tool.call.id`,
`gen_ai.tool.type` (function/datastore), with arguments/outputs opt-in for privacy.
Observability tools (Langfuse, LangSmith, Arize, MLflow) nest these as child spans
under planning/agent spans, all correlated by run/step id. Benchmarks supply the
error taxonomy: BFCL classifies failures as wrong/hallucinated function name,
wrong function count, wrong format, missing required parameter, instruction-
alignment; SoMe's three buckets are format, tool-selection, parameter errors (plus
hallucinated tool response). LumiMAS defines `tool_success_rate = 1 - failures/tools_used`.

## What to log
- Per-tool span: `gen_ai.tool.name`, `tool.call.id`, tool type, run/step/trace id.
- Arguments + result (opt-in / redacted), output size, retry count, attempt number.
- Outcome: success/fail + `error.type` (format / selection / param / not-found /
  hallucinated / exec-error / timeout).
- Latency (`gen_ai.client.operation.duration`), tokens (`gen_ai.client.token.usage`).
- Tool-chain order, distinct tools used, total iterations; attached scores
  (LLM-judge, user feedback) per trace.

## Metrics
- Tool frequency / mix; per-tool error rate broken down by `error.type`.
- `tool_success_rate`; one-attempt success rate (OSR); tool-selection accuracy.
- Reliability: `pass^k` (all k trials succeed, p^k decay) vs `pass@k`.
- Latency/token cost per tool; retry rate; relevance/abstention (refusing a call).

## How it is used
Closed loop (Langfuse pattern): instrument every tool call → monitor dashboards
(error rate, latency, cost) and score live traffic via LLM-judge + thumbs; tail-
sample to keep all failures → drill nested traces to root-cause by `error.type` →
curate failing production traces into a versioned dataset (regression suite) →
run experiments on prompt/tool-spec/model changes, reading item-level diffs (not
averages, which hide spikes) → gate in CI/CD before shipping → feed new edge cases
back. Per-tool error breakdown points at fixes: format/param → tool schema &
descriptions; selection → routing/prompt; not-found → catalogue gaps.

## Sources
- OTel GenAI spans (`execute_tool`): https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/
- OTel GenAI metrics (duration, token.usage, error.type): https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-metrics/
- OTel MCP semconv: https://opentelemetry.io/docs/specs/semconv/gen-ai/mcp/
- BFCL (error categories, AST eval): https://openreview.net/pdf?id=2GmDdhBdDk
- τ-bench (pass^k reliability): https://arxiv.org/abs/2406.12045
- SoMe benchmark (OSR, failure cases): https://arxiv.org/pdf/2512.14720
- LumiMAS (tool_success_rate): https://arxiv.org/pdf/2508.12412
- Langfuse agent observability + feedback loop: https://langfuse.com/blog/2024-07-ai-agent-observability-with-langfuse
- Langfuse experiments/regression: https://langfuse.com/blog/2025-11-06-experiment-interpretation
- Uptrace OTel for AI (tail sampling): https://uptrace.dev/blog/opentelemetry-ai-systems
