# TOPIC 09: Query Performance Prediction (QPP)

## Findings
QPP estimates retrieval effectiveness for a query *without* relevance labels. Two
families: **pre-retrieval** (query + corpus stats only, cheap, computed at index time)
and **post-retrieval** (analyse the ranked list / score distribution, more accurate but
retrieval-model-dependent). Classic pre-retrieval signals: AvgIDF/MaxIDF (term
specificity), SCQ, query-term variance (VAR). Classic post-retrieval: **Clarity**
(KL divergence between top-k language model and corpus), **WIG** (top-k mean score
minus corpus score), **NQC** (std-dev of top-k scores, normalised by corpus score).
Post-retrieval generally beats pre-retrieval; linear-regression *ensembles* of multiple
predictors beat any single one. RAG work extends this to a 3-stage taxonomy:
pre-retrieval, post-retrieval, and **post-generation** (next-token distribution of the
answer). Retrieval can *degrade* answer quality on a meaningful fraction of queries, so
predictors drive "selective RAG" (skip retrieval when it won't help). Production
observability tools (Langfuse + RAGAS) log per-stage scores so failures can be traced.

## What to log
- Pre-retrieval: query terms, AvgIDF/MaxIDF/SCQ, query length, ambiguity/specificity.
- Post-retrieval: top-k retrieval scores (for NQC std-dev, WIG mean), Clarity score,
  reranker (cross-encoder) scores, score gap between rank 1 and rank k.
- Routing/intent classification confidence and chosen data source.
- Post-generation: answer-token confidence, faithfulness/groundedness (LLM-judge).
- Trace metadata: chunk ids, k cutoff, retriever/model version, latency.

## Metrics
- QPP accuracy = correlation of predictor vs ground-truth effectiveness:
  Pearson / Kendall-tau / Spearman against nDCG, MAP.
- For RAG: correlation of predictor with answer correctness ("RAG gain").
- RAGAS reference-free: faithfulness, context relevance, answer relevance.
- Ensemble lift; cross-configuration consistency (predictor rank stable across BM25/E5,
  across LLMs).

## How it is used
- **Selective RAG / routing**: predicted low utility -> skip retrieval, fall back, or
  route to another source.
- **Adaptive pipeline**: trigger query reformulation / expansion when QPP is low.
- **Agentic RAG**: score each generated sub-query to decide "am I on the right track".
- **Monitoring**: online eval on sampled (~2%) traces; alert on score drift; drill into
  low-scoring traces to isolate retrieval-vs-generation fault.

## Sources
- IR overview / taxonomy: https://www.emergentmind.com/topics/query-performance-prediction-qpp
- Pre-retrieval predictors survey (CIKM): https://dl.acm.org/doi/10.1145/1458082.1458311
- Clarity (predicting query performance): https://www.researchgate.net/publication/2476171_Predicting_Query_Performance
- QPP survey (ad-hoc to conversational): https://arxiv.org/pdf/2305.10923
- Neural QPP (ICTIR 2023): https://www.dei.unipd.it/~ferro/papers/2023/ICTIR2023-CFFFLMP.pdf
- RAG retrieval/answer-quality prediction: https://arxiv.org/html/2601.14546
- Agentic RAG QPP ("Am I on the right track?"): https://arxiv.org/html/2507.10411v1
- Langfuse RAG observability and evals: https://langfuse.com/blog/2025-10-28-rag-observability-and-evals
- Per-stage decision logging (RAG observability 2026): https://futureagi.com/blog/what-is-rag-observability-2026
