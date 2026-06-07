# Topic 14: RAG evaluation metrics computable online from logs (RAGAS-style, reference-free)

## Findings
The dominant pattern is **reference-free LLM-as-a-judge** plus **implicit behavioural signals**, computed continuously on production traces. RAGAS established three self-contained metrics (no gold answers): **faithfulness**, **answer relevance**, **context relevance** — all derived by prompting an LLM over the (query, retrieved-context, answer) triple. Observability platforms (Langfuse, Arize Phoenix) attach these as scores to live traces, separating **retrieval** quality from **generation** quality so failures can be root-caused. The classic IR community supplies the other half: clicks, dwell time, query reformulation, and abandonment as cheap, large-scale relevance proxies.

## What to log
- The full triple per request: query, each retrieved chunk (+ retrieval/rerank score, rank), final answer.
- Per-span trace structure (retriever / reranker / generation) via OpenTelemetry; latency and token cost per stage.
- Judge outputs: numeric/categorical score + reasoning string, attached to trace or observation.
- Implicit user signals: clicks on cited sources, dwell time, copy/accept actions, query reformulation chains, abandonment, retries, thumbs up/down.

## Metrics (reference-free)
- **Faithfulness** = verifiable statements / total statements vs context (hallucination proxy).
- **Answer relevance** (cosine sim between original query and LLM-reverse-generated questions).
- **Context relevance / precision@k**, nDCG over retrieved chunks; per-chunk relevance average.
- **QA correctness** and **hallucination rate** (Phoenix LLM-evals).
- Behavioural: click-through, good-vs-bad abandonment, reformulation rate, satisfaction proxies.

## How it is used
Low scores trigger root-cause: bad retrieval (improve chunking/reranking/k) vs bad generation (fix prompt). Interesting/failing traces are promoted into evaluation **datasets**; offline experiments sweep parameters (chunk size 128/256/512, overlap) and compare average scores side by side before shipping. Reformulation/abandonment chains feed query-rewriting and learning-to-rank (query chains). Implicit signals are blended with static signals and decayed to fight click bias/noise.

## Sources
- RAGAS paper: https://arxiv.org/abs/2309.15217
- RAGAS metrics docs: https://docs.ragas.io/en/v0.1.21/concepts/metrics/
- Langfuse RAG observability & evals: https://langfuse.com/blog/2025-10-28-rag-observability-and-evals
- Langfuse observation-level evals: https://langfuse.com/changelog/2026-02-13-observation-level-evals
- Langfuse user feedback (implicit/explicit): https://langfuse.com/docs/observability/features/user-feedback
- Arize Phoenix RAG evals: https://phoenix.arize.com/evaluate-rag-with-llm-evals-and-benchmarking/
- Query Chains (learning to rank from implicit feedback): https://arxiv.org/pdf/cs/0605035
- Beyond Clicks: query reformulation as satisfaction predictor (MSR): https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/Hassan_CIKM13a.pdf
- Modeling Dwell Time to Predict Click-level Satisfaction: https://dl.acm.org/doi/pdf/10.1145/2556195.2556220
