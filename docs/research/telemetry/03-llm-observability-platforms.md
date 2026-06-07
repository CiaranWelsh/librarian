# Topic 03: What LLM Observability Platforms Log and Surface

LangSmith, Langfuse, Arize Phoenix, and Helicone (all open-source-leaning) converge on a
common model: a hierarchy of **trace -> observation/span -> score**, increasingly grounded in
OpenTelemetry / OpenInference so any framework plugs in.

## Findings
- Langfuse: `session -> trace -> observation (span/generation/event) -> score`. Trace attrs
  (`user_id`, `session_id`, tags, metadata) propagate to children. Typed observations make
  telemetry queryable (e.g. "all generations with total_tokens > 2000").
- LangSmith: auto-traces every run (LLM/tool/custom) from env vars alone; auto-captures
  tokens + cost (incl. reasoning-token cost). Feedback is first-class via `create_feedback()`.
- Phoenix: OpenInference spans; exports retriever spans into query/document dataframes;
  evals written back onto spans as annotations. Embeddings clustered to find weak subsets.
- Helicone: proxy/gateway logs raw request+response with cost/latency/TTFT/tokens, segmented
  by user, model, and prompt; has Custom Properties, Sessions, Eval Scores, User Feedback.

## What to log
- Inputs/outputs per step: prompt, completion, retrieved docs (+ doc count, relevance).
- Token usage (prompt/completion/total, reasoning tokens), cost, latency incl. TTFT.
- Model + params (name, temperature), prompt version, app/release version.
- Correlation IDs: `trace_id`, `session_id`/thread, `user_id`; tags + custom metadata.
- Scores: numeric/boolean/categorical from heuristics, LLM-as-judge, and human annotation.

## Metrics
- Latency P50/P99, error rate, cost per user/session, token volume.
- RAG: context/retrieval relevance, faithfulness/groundedness, Q&A correctness, hallucination.
- Quality scores aggregated per experiment (accuracy, pass rate); per-failure-category rates.

## How it is used (feedback loop)
Observe production traces -> run online evals + systematic **error analysis** (open-coding
traces into a domain-specific failure taxonomy, then measuring per-category rates) -> route
hard/failed traces to **annotation queues** for expert scoring + corrected outputs -> promote
those traces into curated **regression datasets** -> test prompt/model fixes offline against
them -> deploy -> monitor again. Human labels also calibrate the LLM-as-judge. LangSmith's
Insights Agent auto-clusters failure modes across millions of traces.

## Sources
- https://langfuse.com/docs/observability/data-model
- https://langfuse.com/blog/2025-08-29-error-analysis-to-evaluate-llm-applications
- https://langfuse.com/docs/evaluation/evaluation-methods/annotation-queues
- https://docs.langchain.com/langsmith/evaluation
- https://www.langchain.com/articles/llm-evals
- https://arize.com/docs/phoenix
- https://arize.com/blog/from-production-traces-to-better-ai-agents-automating-the-llmops-feedback-loop/
- https://docs.helicone.ai/
- https://www.helicone.ai/blog/llm-observability
