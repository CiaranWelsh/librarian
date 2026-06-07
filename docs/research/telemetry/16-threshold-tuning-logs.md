# Topic 16: Tuning decision thresholds from logged score distributions

## Findings

Raw retrieval/similarity scores are **poorly calibrated** to true relevance, so a hardcoded
cutoff (the ubiquitous "0.7 from a tutorial") is unreliable: the right value depends on the
embedding model, the corpus, and query specificity. Two camps tune it from logged data.
(1) **Empirical / labeled:** run a labeled query set through the retriever, plot precision-recall
or F1 vs threshold, take the operating point at the F1 knee (Youden's J on ROC, or a target-recall
/ cost-constrained point). (2) **Distribution-fitting:** compute the score distribution and set the
cutoff from its statistics. Practitioners use μ, μ±σ candidates over pairwise cosine distances;
IR research fits parametric mixtures (normal-relevant / exponential-nonrelevant, EM) or, in Google's
*Surprise* (SIGIR), a Generalized Pareto tail from **Extreme Value Theory** to emit a calibrated
per-query "surprise" score for result-list truncation. Assumption-free learners (BiCut, *Choppy*
cut-transformer) learn the cut directly from the ranked-score vector. A key empirical pattern in
logs: similarity drops sharply after the top chunk, approaching ~0 by the 5th — a signal that a
fixed top-k is wrong. Warning signs: too-high thresholds (0.8) starve context and degenerate RAG
into a bare LLM; bimodal quality distributions reveal under-served query patterns.

## What to log

- Per-query, per-result raw similarity/distance scores (full top-k vector, not just top-1).
- Score gaps / decay between consecutive ranks; top-1 score; count above candidate cutoffs.
- "No-context" / abstention rate per candidate threshold.
- Labeled relevance judgments on a holdout query set (for PR/ROC curves).
- Downstream outcomes: faithfulness/hallucination flags, click/feedback, latency percentiles.
- Trailing-window percentile of each raw score (e.g. 6-week moving average) for drift.

## Metrics

- Precision-recall and F1 curves vs threshold; **F1-knee**, **Youden's J** (max sensitivity+specificity-1),
  target-recall and cost/utility operating points.
- Score-distribution stats: μ, σ, percentiles; shape checks (bimodality).
- Calibration: Brier / log-loss / reliability plots (note: monotonic recalibration leaves AUC unchanged).
- Drift: percentile shift of score distribution over trailing window; recall proxies (avg neighbor distance) + golden-query recall.

## How it is used

Feedback loop: log scores → build distribution (labeled set or corpus internal distances) →
pick threshold at F1/Youden/cost point, validating on a **separate** holdout to avoid overfitting →
deploy → monitor score-distribution percentiles and golden/guardrail queries → re-calibrate when
the corpus or query mix drifts (alert when metrics cross thresholds). Observability stacks
(Langfuse traces+scores, Arize Phoenix RAGAS evals, dashboards with drift alerts) operationalize
this; LLMs fail silently (HTTP 200 on wrong answers), so threshold + distribution monitoring catches
silent retrieval degradation. Layer reranking and citation checks rather than treating the threshold
as the sole gate.

## Sources

- Surprise: Result List Truncation via Extreme Value Theory (Bahri et al., SIGIR) — https://arxiv.org/abs/2010.09797
- Choppy: Cut Transformer for Ranked List Truncation (Bahri et al., SIGIR'20) — https://arxiv.org/pdf/2004.13012
- Learning to Truncate Ranked Lists for IR — https://arxiv.org/pdf/2102.12793
- Relevance prediction in similarity-search via EVT (ScienceDirect) — https://www.sciencedirect.com/science/article/abs/pii/S1047320319300720
- Understanding RAG Score Thresholds (Nick Berens) — https://nickberens.me/blog/understanding-rag-score-thresholds/
- RAG Engineering in Production: threshold calibration (Medium) — https://medium.com/@igorcnnbd/rag-engineering-in-production-hybrid-search-threshold-calibration-and-citation-verification-that-22f6adf7514d
- Better RAG Retrieval — Similarity with Threshold (Meisin Lee) — https://meisinlee.medium.com/better-rag-retrieval-similarity-with-threshold-a6dbb535ef9e
- Monitoring Vector Search Performance Metrics (APXML) — https://apxml.com/courses/advanced-vector-search-llms/chapter-4-scaling-vector-search-production/monitoring-vector-search-metrics
- Evaluate vector search retrieval quality (Databricks) — https://docs.databricks.com/gcp/en/vector-search/retrieval-quality-eval
- scikit-learn: Tuning the decision threshold — https://scikit-learn.org/stable/modules/classification_threshold.html
- ValidMind ClassifierThresholdOptimization — https://docs.validmind.com/tests/model_validation/sklearn/ClassifierThresholdOptimization.html
- Balanced Accuracy / Youden's J for LLM judges — https://arxiv.org/pdf/2512.08121
- Langfuse LLM Observability overview — https://langfuse.com/docs/observability/overview
- LLMOps Observability: LangSmith vs Arize vs Langfuse vs W&B — https://medium.com/@kanerika/llmops-observability-langsmith-vs-arize-vs-langfuse-vs-w-b-f1baeabd1bbf
