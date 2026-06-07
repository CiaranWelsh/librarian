# Topic 12: Detecting zero-result and low-confidence queries

## Findings
Production search splits failures into two classes. **Zero-result** (a.k.a. "null result", "no results rate") is the
trivially observable case logged by every search platform (Algolia, Adobe Live Search). The harder class is **"false
zeros"** (relevant content exists but the engine returns nothing) vs genuine catalog/corpus gaps — Tunkelang warns that
naively minimizing the null-rate "confuses causes with symptoms" and you must detect failure *at the source* (query
understanding), not downstream.

For RAG, the analogue of "low results" is **low retrieval confidence**, which triggers **abstention** rather than a
confident-but-wrong answer. Signals used: top-chunk similarity below a threshold, reranker margin (gap between top and
2nd passage), cross-retriever agreement (dense vs sparse overlap), and evidence-coverage ratio (% of answer claims with
citations). HALT-RAG sets per-task thresholds via calibrated NLI (t≈0.38–0.42) optimizing F1 subject to a precision
floor. Microsoft's confidence-aware RAG layers retrieval scoring + citation validation + LLM-judge abstention.

Classic IR has studied this for 20 years as **Query Performance Prediction (QPP)**: pre-retrieval predictors (Simplified
Clarity Score, AvICTF, max-IDF, σ-IDF) computable at index time, and post-retrieval predictors (Clarity Score, NQC, WIG)
based on the variance/magnitude of retrieved document scores. Observability tools (Langfuse, Arize Phoenix) log typed
retrieval spans with document counts + per-chunk relevance scores, then run LLM-as-judge evals to flag low-relevance
traces.

## What to log
- Query text, normalized form, result/hit count (`nbHits`), and a zero/low flag per query.
- Top-1 similarity score, reranker top-vs-2nd margin, dense/sparse retriever overlap.
- Pre-retrieval predictors (SCS, max-IDF) and post-retrieval predictors (NQC, score variance).
- Abstention/refusal event + reason ("only one source", "score below threshold", "outdated source").
- Evidence-coverage ratio and a green/yellow/red confidence label per answer.
- Query frequency + downstream action (reformulation, exit, click) to separate "false zeros" from real gaps.

## Metrics
- No-Results-Rate / Zero-Results-Rate (mature targets ~<10%; ecommerce best-in-class <2-3%).
- Abstention rate and the precision/recall (coverage) trade-off curve at varying thresholds.
- QPP correlation with average precision (validation that predictors track real quality).
- Per-query demand score = frequency × exit-rate × value, for triage prioritization.
- Retrieval relevance score distribution; fraction of traces below a relevance floor.

## How it is used (feedback loop)
- Weekly triage of high-frequency zero/low queries → add synonyms, rules, redirects, or fill corpus gaps.
- Tune the abstention threshold on a labeled eval set to hit a target precision/coverage.
- Low confidence triggers graceful fallback: ask to clarify, expand sub-queries (agentic refine), or abstain with an
  explainable reason instead of hallucinating.
- Mined zero-result terms reveal vocabulary mismatch and corpus gaps that feed re-indexing and content acquisition.

## Sources
- Tunkelang, "Making Sense of Null and Low Results": https://dtunkelang.medium.com/making-sense-of-null-and-low-results-a077f37bf8fc
- Algolia search-analytics metrics (no-results rate): https://www.algolia.com/doc/guides/search-analytics/concepts/metrics
- Wizzy, fixing zero-result searches: https://wizzy.ai/blog/zero-result-searches-solution/
- Microsoft, Confidence-Aware RAG (retrieval score + citation + abstention): https://techcommunity.microsoft.com/blog/azuredevcommunityblog/confidence-aware-rag-teaching-your-ai-pipeline-to-acknowledge-uncertainty/4515061
- HALT-RAG (calibrated NLI abstention, precision-constrained thresholds): https://arxiv.org/pdf/2509.07475
- He & Ounis, Query Performance Prediction (SCS, AvICTF, pre/post-retrieval): http://ir.dcs.gla.ac.uk/smooth/Is-special-final.pdf
- Langfuse, RAG Observability and Evals (typed retrieval spans, relevance scoring): https://langfuse.com/blog/2025-10-28-rag-observability-and-evals
