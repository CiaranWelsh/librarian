# Topic 10: Confidence Calibration for Retrieval/Classification

## Findings
Calibration aligns reported confidence with empirical correctness. Two layers matter for a
reference-search tool: (1) **IR score calibration** — raw BM25/dense scores are unbounded and
not comparable across queries, so practitioners fit score-distribution models (normal-exponential)
or Bayesian/logistic transforms to convert scores into relevance probabilities in [0,1], enabling
consistent thresholds and fusion. (2) **LLM/RAG answer calibration** — RAG is notoriously
overconfident; verbal-confidence ECE often exceeds 0.4, and added retrieved context can *suppress*
justified abstention. Post-hoc fixes: temperature scaling (needs logits), or Platt/isotonic on
verbalized scores for black-box APIs. Query Performance Prediction (NQC, WIG, clarity) predicts
per-query difficulty as a confidence proxy without labels.

## What to log
- `(confidence, is_correct)` pairs per answer (gold label from feedback/eval) — min ~500.
- Raw retrieval scores + score distribution of top-k (variance feeds NQC/WIG QPP).
- Verbalized confidence and, if available, token logprobs.
- Sufficient-context flag, citation-validation result, abstain/answer decision + reason.
- Threshold used and resulting coverage at decision time.

## Metrics
- **ECE** (10 bins, sample-weighted |acc−conf|); reliability diagram for direction of miscalibration.
- **Brier score**, **AUROC** (confidence vs. correctness ranking).
- **Risk-coverage curve / AURC**, **Coverage@Acc** (e.g. P=0.95 at ~70% coverage).
- QPP correlation (predicted vs. actual nDCG/MAP).

## How it is used
Build a held-out calibration set; fit temperature/Platt/isotonic mapping; pick abstention threshold τ
from the risk-coverage curve to meet a domain risk tolerance. Route low-confidence queries to abstain
(with reason/citations) or escalate. Monitor ECE drift: investigate if ECE rises >0.03; recalibrate
monthly/quarterly. Guideline thresholds: ECE >0.10 = confidence unreliable; >0.15 = routing worse
than a fixed threshold.

## Sources
- LLM calibration in production (ECE thresholds, monthly recalibration): https://tianpan.co/blog/2026-04-20-llm-calibration-production-overconfidence
- Model calibration / ECE / reliability diagrams (ICLR 2025 blogpost): https://iclr-blogposts.github.io/2025/blog/calibration/
- Noise-aware verbal confidence calibration in RAG (ECE>0.4, AUROC): https://arxiv.org/html/2601.11004
- Adaptive temperature scaling for LLMs: https://arxiv.org/pdf/2409.19817
- Know Your Limits: survey of abstention in LLMs (coverage/risk, AURC): https://direct.mit.edu/tacl/article/doi/10.1162/tacl_a_00754/131566/
- Google "sufficient context" for selective generation in RAG: https://research.google/blog/deeper-insights-into-retrieval-augmented-generation-the-role-of-sufficient-context/
- Confidence-aware RAG (retrieval scoring + citation validation + abstention): https://techcommunity.microsoft.com/blog/azuredevcommunityblog/confidence-aware-rag-teaching-your-ai-pipeline-to-acknowledge-uncertainty/4515061
- Modeling score distributions in IR (normal-exponential, relevance probability): https://link.springer.com/article/10.1007/s10791-010-9145-5
- Bayesian BM25 score-to-probability calibration: https://github.com/cognica-io/bayesian-bm25
- Query Performance Prediction (NQC/WIG, pre/post-retrieval): https://www.emergentmind.com/topics/query-performance-prediction-qpp
- Uncertainty & calibration for deep retrieval models: https://arxiv.org/abs/2105.04651
