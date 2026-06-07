# Topic 36: Relevance Feedback Loops That Improve Ranking Over Time

## Findings
The core loop is: log interactions -> infer relevance -> retrain/re-rank -> repeat. The dominant lesson from IR practice (Joachims, unbiased LTR) is that raw clicks are a *biased* signal: top results get clicked regardless of relevance (position bias), plus trust/selection bias. A naive loop just reinforces its own ordering, so practitioners debias with Inverse Propensity Weighting (IPW)/counterfactual LTR, or use probabilistic click models (Position-Based Model). Glean stresses enterprise feedback is *sparse and noisy* — "if you click a low result it probably should rank higher, but don't overfit one data point." RAG products (Langfuse, Label Studio, 567-labs) treat feedback as "scores" tied to traces, and design UIs (citation marking, doc-filtering) that auto-generate hard negatives for reranker/embedding training.

## What to log
- Explicit feedback: thumbs up/down, 1-5 stars, "was this helpful", per-citation "irrelevant" marks — attached to a trace/query ID.
- Implicit signals: clicks + rank position, dwell time, copy/accept of output, query reformulation/retry, abandonment, skips above a click.
- Context for debiasing: the ranked list shown (positions/exposure), query, user role/team, timestamp.
- Component attribution: which retrieved chunks were used vs. ignored (RAG); flag retrieval-vs-generation failures.

## Metrics
- Click-derived: CTR by position, MRR, NDCG, click-skip / "last click" rates.
- Debiased relevance via IPW-weighted ranking loss; propensity estimates per position.
- RAG/quality scores: context relevance, faithfulness/groundedness, recall@k; human-feedback satisfaction rate / NPS; task-completion rate.
- Drift: query-pattern drift, score trend over time, alert thresholds (e.g. ~15% faithfulness drop).

## How it is used
- Mine clicks into (query -> positive/hard-negative) pairs; retrain LTR/reranker periodically, debiased with IPW or click models so the loop converges to true relevance, not self-reinforcement.
- Route low-scoring traces to annotation queues; convert into eval/regression datasets ("real failures strengthen coverage") and LLM-as-judge baselines.
- Correlate user feedback with offline metrics to localize fault (retriever vs. reranker vs. generation vs. stale corpus); update index/corpus accordingly.
- Self-tuning ranking + alerts/auto-revert on metric anomalies.

## Sources
- Joachims et al., Unbiased Learning-to-Rank with Biased Feedback: https://arxiv.org/pdf/1608.04468
- Counterfactual LTR / position-bias survey context: https://arxiv.org/html/2404.03707 and https://arxiv.org/html/2506.20854
- AI-Powered Search, Ch.11 Automating LTR with click models: https://livebook.manning.com/book/ai-powered-search/chapter-11
- Langfuse User Feedback / Scores: https://langfuse.com/docs/observability/features/user-feedback and https://langfuse.com/docs/scores/overview
- 567-labs, Systematically Improving RAG (feedback collection, hard negatives): https://567-labs.github.io/systematically-improving-rag/
- Label Studio, Human-in-the-loop for RAG: https://labelstud.io/blog/why-human-review-is-essential-for-better-rag-systems/
- Feedback Loop RAG (Salton/Buckley lineage, index fine-tuning): https://www.machinelearningplus.com/gen-ai/feedback-loop-rag-improving-retrieval-with-user-interactions/
- Glean, Enterprise search ranking/personalization: https://www.glean.com/blog/enterprise-search-is-hard-why-its-so-behind-and-what-itll-take-to-catch-up
