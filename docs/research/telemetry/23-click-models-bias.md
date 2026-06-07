# 23 - Click Models (position bias, cascade) for Extracting Relevance from Logs

## Findings
Clicks are cheap, abundant relevance signals but are *biased*: a top-ranked result is clicked more
regardless of true relevance (position bias). Practitioners model `P(click) = P(examine | rank) *
P(relevant)` (the examination hypothesis). The **Position-Based Model (PBM)** treats examination as
a per-rank propensity; the **Cascade Model** assumes users scan top-to-bottom and stop at the first
satisfying result (best fit for early-rank effects, but only handles single-click sessions). DBN and
UBM extend this. Production systems estimate propensities via result randomization (RandPair/RandTopN
— hurts UX), or randomization-free EM / Dual Learning (Google personal search), then debias with
**Inverse Propensity Weighting (IPS/IPW)** in the ranker's loss. Tripadvisor used historical bookings
as a relevance proxy per position. NDCG can *understate* real position bias, so offline wins must be
A/B-validated.

## What to log (per query event)
- Query text/hash, query embedding, user_id, session_id, timestamp.
- Ordered candidate list with retrieval scores AND displayed rank/position (the propensity key).
- Click events: which result, at which rank, dwell time (long vs. short click — relatively
  immune to presentation bias since it compares a doc to itself).
- Explicit feedback (thumbs up/down), follow-up queries, task-completion / abandonment.
- Impression/skip data (items shown but not clicked) — needed to estimate examination.

## Metrics
- Per-position CTR / click-ratio (raw propensity estimate); examination probabilities θ_k.
- IPS-weighted CTR; NDCG / NDCG@k and IPW-weighted NDCG.
- Cascade/PBM model fit (log-likelihood, perplexity of predicted clicks).
- Estimated per-document relevance (attractiveness) after debiasing.

## How it is used (feedback loop)
Two-step ULTR: (1) estimate position bias / extract debiased relevance from logs; (2) retrain ranker
with IPS-weighted loss (Propensity SVM-Rank, Unbiased LambdaMART, DLA). Validate offline NDCG gains
via A/B test or shadow traffic with confidence intervals before shipping; iterate.

## Sources
- Craswell et al., cascade model (WSDM 2008): https://dl.acm.org/doi/10.1145/1341531.1341545
- Chandar & Carterette, cascade clickthrough bias: https://pchandar.github.io/files/papers/Chandar2018.pdf
- Joachims et al. / Ai et al. Unbiased LTR + propensity estimation: https://arxiv.org/pdf/1804.05938
- Wang et al., position-bias estimation in personal search (EM, RandPair): https://research.google/pubs/position-bias-estimation-for-unbiased-learning-to-rank-in-personal-search/
- Hu et al., Unbiased LambdaMart (joint bias+ranker): https://arxiv.org/pdf/1809.05818
- Tripadvisor hotels ULTR (bookings as relevance proxy): https://arxiv.org/pdf/2002.12528
- Hofmann et al., position bias effects on click-based evaluation / NDCG: https://staff.fnwi.uva.nl/m.derijke/wp-content/papercite-data/pdf/hofmann-effects-2014.pdf
- Google patent, dwell time long-vs-short clicks: https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/11816114
- Langfuse data model (events for clicks, retriever observations, scores): https://langfuse.com/docs/observability/data-model
- Langfuse user feedback (implicit signals): https://langfuse.com/docs/observability/features/user-feedback
