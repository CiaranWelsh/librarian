# Telemetry Topic 02: Production Request/Response Logging Schema for RAG

## Findings

Production RAG observability converges on a **hierarchical trace model**: one user query = one trace, with child spans for retrieval, (re)ranking, embedding, and generation. Langfuse, Arize Phoenix (OpenInference/OTel), and LangSmith all share this shape. Phoenix fixes span *kinds* (Chain, Retriever, Reranker, LLM, Embedding, Agent, Tool); retrieved docs live under `retrieval.documents` with `document.content` + `document.score` so each doc explodes to a row for eval. IR literature (Joachims; Radlinski & Joachims "Query Chains") adds the older, durable lesson: log queries, the *ranking shown*, clicks, dwell time, and **query chains** within a session — these become pairwise relevance preferences for learning-to-rank, after de-biasing for position.

## What to log

- **Request**: trace/request id, timestamp, raw query, session/thread id, user/tenant + metadata.
- **Retrieval**: retrieved chunk/doc ids + URIs, per-chunk relevance scores, top score, k, embedding model, ranking shown.
- **Generation**: model name, prompt sent, response, prompt/completion/total tokens, cost.
- **Performance**: retrieval / generation / end-to-end latency (P50, P95).
- **Feedback**: explicit (thumbs up/down, annotations) + implicit (click, dwell, rephrase, abandon, copy).
- **Eval scores attached back to spans**: relevance, faithfulness/groundedness, hit-rate.

## Metrics

Context precision & recall; retrieval hit-rate / percent-relevant; top-score distribution (drift); hallucination rate (LLM-judge, target <5%); P50/P95 latency; tokens/cost per query; click/CTR and dwell from logs.

## How it is used

Online evals (LLM-as-judge) score live traces to detect drift; score drops below threshold trigger alerts/fallback. Logged query→ranking→click chains become LTR training preferences. A/B comparison of chunking/embedding/retriever changes uses the same logged metrics for regression detection. Low-confidence retrievals are filtered or routed to broader search rather than answered.

## Sources

- Langfuse RAG observability: https://langfuse.com/blog/2025-10-28-rag-observability-and-evals ; https://langfuse.com/docs/observability/overview
- Arize Phoenix span types & retrieval schema: https://docs.arize.com/arize/llm-tracing/how-to-tracing-manual/instrumenting-span-types ; https://arize.com/docs/phoenix/retrieval/quickstart-retrieval ; https://arize.com/docs/phoenix/tracing/how-to-tracing/importing-and-exporting-traces/extract-data-from-spans
- LangSmith run schema & online evals: https://reference.langchain.com/python/langsmith/schemas/Run ; https://docs.langchain.com/langsmith/online-evaluations
- Production RAG logging practice: https://blog.premai.io/building-production-rag-architecture-chunking-evaluation-monitoring-2026-guide/ ; https://towardsai.net/p/machine-learning/production-rag-the-chunking-retrieval-and-evaluation-strategies-that-actually-work
- IR / learning-to-rank from logs: https://arxiv.org/abs/cs/0605035 (Query Chains) ; https://www.semanticscholar.org/paper/Accurately-interpreting-clickthrough-data-as-Joachims-Granka/3ce4e4df850d8aeb85d68b3a2bcf1937ec49d74b
