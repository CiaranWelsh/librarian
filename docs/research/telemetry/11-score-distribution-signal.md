# Topic 11: Top-k similarity score distributions as a retrieval quality signal

## Findings
The shape of the top-k score vector — not just the #1 score — is a usable retrieval-quality
signal. Two practices dominate. (1) **Gap / drop-off detection**: relevant docs cluster high,
then scores fall sharply; the largest gap between consecutive ranks marks the relevant/irrelevant
boundary. MMed-RAG truncates k when the log-ratio `u_i = log(S_i/S_{i+1})` exceeds threshold γ
(borrowed from the clustering Gap statistic); "adaptive-k" cuts at the single largest consecutive
gap, tuning-free, beating fixed-k. (2) **Distribution statistics**: classic IR Query Performance
Prediction uses the **standard deviation of retrieval scores** as a post-retrieval predictor (high
spread = less query drift = better performance), and Clarity Score (KL divergence of query vs
collection model) correlates with average precision on TREC. A global μ/σ of all pairwise distances
yields thresholds μ−σ, μ, μ+σ. **Caveat repeatedly stressed**: mean similarity measures coherence,
not correctness, and rises with corpus size (false confidence); raw retriever scores aren't
fine-grained relevance judgments.

## What to log
- Full top-k score vector per query (all k scores, not just max), with ranks.
- Max score, mean, std/variance, and consecutive gaps (S_i − S_{i+1}) / log-ratios.
- Score at the chosen cutoff; #results above any threshold; "zero-above-threshold" flag.
- Distance vs similarity convention (cosine distance ≠ cosine similarity across LangChain/LlamaIndex).
- Query embedding + retrieval latency, for drift correlation.

## Metrics
- Std-dev of scores (QPP predictor); Clarity Score; max/mean similarity.
- Largest-gap position and magnitude; γ-threshold hit rate.
- Abstention / fallback-trigger rate (all docs below threshold → "no answer" or web-search).
- Precision@k / Recall@k / F1@k against labeled sets; correlation of score-stats with answer faithfulness.

## How it is used
- **Adaptive truncation**: pick k at the drop-off instead of fixed k (adaptive-k, MMed-RAG).
- **Abstention / corrective routing**: CRAG-style — if all scores below threshold → abstain or trigger external search.
- **Observability dashboards**: Arize/Phoenix, Langfuse, LangSmith log per-span retrieval scores; alert/trace-filter on "zero relevant docs" or score drops to catch KB-update or query-drift regressions.
- **Threshold tuning**: calibrate cutoff on a labeled set, trading abstention vs false positives; embedding-drift detection over time.

## Sources
- MMed-RAG (gap-statistic k optimization): https://arxiv.org/pdf/2410.13085
- Adaptive-k (tuning-free largest-gap cutoff): https://arxiv.org/pdf/2506.08479
- Better RAG Retrieval — Similarity with Threshold: https://meisinlee.medium.com/better-rag-retrieval-similarity-with-threshold-a6dbb535ef9e
- Confidence-Aware RAG (Microsoft, abstention on no-pass threshold): https://techcommunity.microsoft.com/blog/azuredevcommunityblog/confidence-aware-rag-teaching-your-ai-pipeline-to-acknowledge-uncertainty/4515061
- "RAG gets confidently wrong as memory grows" (mean-similarity ≠ correctness): https://towardsdatascience.com/your-rag-gets-confidently-wrong-as-memory-grows-i-built-the-memory-layer-that-stops-it/
- QPP: std-dev of scores / Clarity Score: http://ir.dcs.gla.ac.uk/smooth/Is-special-final.pdf
- Robust std-dev estimator for QPP (SIGIR ICTIR): https://dl.acm.org/doi/10.1145/3121050.3121087
- Langfuse RAG observability & per-span scores: https://langfuse.com/blog/2025-10-28-rag-observability-and-evals
- How to Evaluate Retrieval Quality (Precision/Recall/F1@k): https://towardsdatascience.com/how-to-evaluate-retrieval-quality-in-rag-pipelines-precisionk-recallk-and-f1k/
