# OTel / Distributed Tracing for LLM & RAG Apps

## Findings
The OpenTelemetry **GenAI semantic conventions** are the de-facto schema. A request is one **trace**; spans nest hierarchically: a top-level `invoke_agent` span parents `chat` (inference), `embeddings`, `execute_tool`, and `retrieval` spans. Langfuse mirrors this with typed observations (`generation`, `retriever`, `embedding`, `tool`, `evaluator`, `guardrail`). Content (prompts/docs) is opt-in via attributes or span events because of size and privacy concerns. Per-trace evals run on 5-20% sampled live traffic, while cheap heuristics run on 100%.

## What to log (per request)
- **Identity/context:** `trace_id`, `session_id`, `user_id`, tags, app version (propagated to all spans).
- **Inference span:** `gen_ai.operation.name`, `gen_ai.request.model`/`response.model`, temperature/top_p, `gen_ai.usage.input_tokens`/`output_tokens`, `cache_read.input_tokens`, `gen_ai.response.finish_reasons`, `error.type`, opt-in `input.messages`/`output.messages`.
- **Retrieval span:** `gen_ai.data_source.id`, `gen_ai.request.top_k`, `gen_ai.retrieval.query.text`, retrieved doc IDs + relevance scores, latency.
- **Embedding span:** `gen_ai.embeddings.dimension.count`, encoding format.
- **Quality proxies:** thumbs up/down, retry/regenerate, refusal.

## Metrics
TTFT (p95 SLO ~800ms; ~200ms chat), time-per-output-token / tokens-per-sec, end-to-end latency (p95/p99 over median), input/output tokens, cost per request and per session. RAG-quality: contextual precision/recall/relevancy (retriever), faithfulness/groundedness and answer relevancy (generator), scored offline and online.

## How it is used
Traces feed a **data flywheel**: production failures and low-scoring traces are sampled into datasets, labelled by LLM-as-judge then human review, and become regression tests. Dashboards alert on metric drift (e.g. context-relevance drop) before users notice. Feedback (ratings, corrections) drives index/retrieval fine-tuning; Pistis-RAG reports 6-7% gains from human feedback.

## Sources
- https://opentelemetry.io/docs/specs/semconv/gen-ai/
- https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/
- https://opentelemetry.io/blog/2026/genai-observability/
- https://langfuse.com/docs/observability/data-model
- https://langfuse.com/blog/2025-10-28-rag-observability-and-evals
- https://www.confident-ai.com/blog/rag-evaluation-metrics-answer-relevancy-faithfulness-and-more
- https://tianpan.co/blog/2025-11-01-llm-observability-production
- https://docs.langchain.com/langsmith/evaluation
- https://arize.com/blog/top-llm-tracing-tools/
- https://www.machinelearningplus.com/gen-ai/feedback-loop-rag-improving-retrieval-with-user-interactions/
