# 50 — Industry RAG / search usage-data case studies

## Findings
Production RAG and enterprise-search teams treat the system as *living*: usage
data is the primary fuel for tuning. Three recurring patterns emerge.

- **Glean** (enterprise search) runs *self-tuning ranking* over an enterprise
  knowledge graph. Its Activity API captures document views/edits/comments and
  search-result views, clicks, and explicit feedback; a low-clicked result is
  promoted next time without overfitting. Crucially, enterprise feedback is
  *sparser and less reliable* than web search, and signals are gated behind
  privacy thresholds (multiple users must share a datapoint).
- **Perplexity** (answer engine) logs clicked citations, dwell time, copy
  events, thumbs up/down, and follow-up prompts. Sources repeatedly skipped or
  downvoted are reportedly dropped from future answers within ~a week — a
  lagging, downvote-dependent loop (caveat: third-party analysis, not official).
- **Langfuse / LangSmith** (LLM observability) instrument every query as a
  trace of spans (retrieval, embedding, LLM call), capturing inputs/outputs,
  retrieved docs, latency (P50/P99), cost, token counts, errors, and scores.
- **IR research** warns that clicks are biased by *position*; counterfactual /
  propensity-weighted LTR is needed before treating clicks as relevance.

## What to log
- Query text + parsed intent; retrieved doc/chunk IDs with rank and scores.
- Clicks/citations opened, rank of click, dwell time, copy events.
- Explicit feedback (thumbs, ratings, annotations); follow-up/reformulation.
- Abstentions / "no good result"; per-span latency, cost, tokens, errors.
- Eval scores (faithfulness, context/answer relevance) attached to the trace.

## Metrics
- Ranking: NDCG@k, MRR, Precision@k (graded, position-discounted).
- RAG triad: context relevance, answer relevance, groundedness/faithfulness.
- Operational: P50/P99 latency, cost/query, error rate, click-through rate.
- Targets seen in the wild: answer accuracy >75%, retrieval precision >80%.

## How it is used
1. Trace + score every query; build a golden dataset from real questions.
2. Detect low-relevance / downvoted / abstained queries → de-index weak
   sources, tune chunking/reranking, fix prompts.
3. Feed (debiased) clicks into self-tuning rank models; retrain embeddings on
   corpus/drift; alert on quality degradation. Both component- and end-to-end
   evaluation are required — good retrieval ≠ good answers.

## Sources
- Glean ranking/feedback: https://www.glean.com/blog/enterprise-search-is-hard-why-its-so-behind-and-what-itll-take-to-catch-up
- Glean Activity API: https://developers.glean.com/api/client-api/activity/overview
- Perplexity feedback loop: https://ziptie.dev/blog/how-perplexity-ai-answers-work/ and https://www.trysight.ai/blog/how-perplexity-ai-selects-sources
- Langfuse RAG observability: https://langfuse.com/blog/2025-10-28-rag-observability-and-evals
- LangSmith observability: https://www.langchain.com/langsmith/observability
- Unbiased LTR / position bias: https://arxiv.org/abs/1608.04468 and https://arxiv.org/abs/2506.06989
- NDCG (Järvelin & Kekäläinen): https://en.wikipedia.org/wiki/Discounted_cumulative_gain
