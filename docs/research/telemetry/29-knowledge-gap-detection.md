# 29. Knowledge/Corpus Gap Detection from Unanswered or Low-Confidence Queries

## Findings
Practitioners treat low-confidence and zero-result queries as a demand signal that points
directly at corpus deficiencies. The first job is diagnosis: separate a true **content gap**
(answer exists nowhere) from a **retrieval failure** (answer exists but isn't surfaced) via an
overlap test — does any document match the intent at all (Tunkelang; Logicbroker). Vocabulary
mismatch, typos, and over-specified queries are retrieval problems, not gaps. Because LLM-judge
and reference-free scoring need no ground truth, these checks run on live traffic, so rare
queries that score poorly flag coverage holes (Evidently). Gaps are most actionable when
**clustered into topics** rather than handled as isolated misses, then ranked by demand-vs-supply
mismatch — frequent user need with zero/minimal coverage (Mind the Gap, arXiv 2509.13626).
Confidence-aware RAG turns the same signal into an abstention: when retrieval confidence or a
groundedness judge falls below threshold (e.g. 0.6), return "insufficient information" instead of
hallucinating, and log it (Microsoft). Each abstention is a gap candidate.

## What to log
- Query text + normalized/embedding, timestamp, result count (flag zero-result).
- Top-k retrieval scores, max score, score spread/gap, reranker score.
- Confidence/answerability verdict; abstention flag + reason (no-docs vs unsupported).
- Groundedness/faithfulness judge score; "no answer in context" label.
- Reformulation chains, abandonment, and post-failure recovery path.

## Metrics
- Zero-result rate (KPI; aim <2-3%, >10% = broken discovery — Algolia/ExpertRec).
- Low-confidence / abstention rate per topic cluster.
- Demand-vs-supply gap score per cluster (query frequency vs content availability).
- Query Performance Prediction: pre-retrieval (specificity, collection similarity) and
  post-retrieval (clarity score, NQC, score distribution) difficulty estimates.
- Accuracy-coverage trade-off (precision at a given abstention rate).

## How it is used
Aggregate failures, cluster into topics, rank by frequency x impact (LangSmith Insights, Phoenix
trace clustering/anomaly detection). High-demand uncovered clusters drive **corpus augmentation** —
author or synthesize targeted content (Mind the Gap). Retrieval-side gaps drive synonyms, hybrid
search, reranking, and threshold tuning. Continuous monthly review loop closes detection ->
content/config fix -> re-measure.

## Sources
- Mind the Gap (KB alignment, demand-driven gap detection): https://arxiv.org/pdf/2509.13626
- RAG for Uncovering Knowledge Gaps: https://arxiv.org/pdf/2312.07796
- Evidently — RAG evaluation on live traffic: https://www.evidentlyai.com/llm-guide/rag-evaluation
- Tunkelang — Making Sense of Null and Low Results: https://dtunkelang.medium.com/making-sense-of-null-and-low-results-a077f37bf8fc
- Logicbroker — Null search results as signal: https://logicbroker.com/null-search-results/
- Algolia — Null results optimization: https://www.algolia.com/ecommerce-merchandising-playbook/null-results-optimization
- Microsoft — Confidence-Aware RAG (threshold + abstention): https://techcommunity.microsoft.com/blog/azuredevcommunityblog/confidence-aware-rag-teaching-your-ai-pipeline-to-acknowledge-uncertainty/4515061
- LangSmith / Arize Phoenix clustering & gap detection: https://medium.com/@kanerika/llmops-observability-langsmith-vs-arize-vs-langfuse-vs-w-b-f1baeabd1bbf
- QPP overview (pre/post-retrieval predictors): https://www.emergentmind.com/topics/query-performance-prediction-qpp
