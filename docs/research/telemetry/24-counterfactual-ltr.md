# Topic 24: Counterfactual / Unbiased Learning-to-Rank from Implicit Feedback

## Findings
Clicks are a biased relevance signal: under the examination hypothesis a click depends on both relevance and the probability the result was *observed*. Position bias dominates (higher ranks get more clicks regardless of relevance); selection bias (top-k only) and trust bias also appear. Counterfactual LTR (Joachims et al. 2017; Wang et al.) corrects this by treating examination probability as a *propensity* and applying Inverse Propensity Scoring (IPS) to get an unbiased risk estimate, enabling training on a new ranker from logs of an old one. Production deployments exist: eBay (intervention-free propensity estimation exploiting natural rank changes of the same query-doc pair over time), Google personal search (regression-EM for sparse clicks), Airbnb (counterfactual eval + interleaving), Adyen (off-policy eval for payments).

## What to log
- Per-impression: query (+ context: navigational/informational), the full displayed ranking, and the **rank/position** each result was shown at.
- Clicks (and non-clicks) per result, plus dwell/booking/purchase as stronger conversions.
- The **logging policy's** scores/propensities (probability the displayed ranking was chosen) — required for unbiased IPS.
- Same query-doc pair seen at *different* ranks over time (enables intervention-free propensity estimation).
- Result-level side features (snippet bolding, price) for contextual position-based models.

## Metrics
- Estimated propensities / examination curve per rank (PBM).
- IPS, SNIPS (self-normalized), and Doubly-Robust off-policy estimates of ranking utility.
- Unbiased DCG (correlates well with online reward; **nDCG correlation degrades** — KDD'24).
- Variance diagnostics: Effective Sample Size ESS=(Σw)²/Σw²; clipping threshold sensitivity (clipped IPS is downward-biased).
- Offline↔online correlation (Pearson ~0.65 Airbnb, >0.8 Adyen IPS/SNIPS).

## How it is used
Estimate propensities (randomization, intervention harvesting, regression-EM, or dual/joint learning like DLA / Unbiased LambdaMART), then train the ranker by IPS-weighting clicks so debiased relevance — not position — drives the loss. Use IPS/SNIPS/DR off-policy estimates as an offline A/B gate to pre-screen candidate rankers cheaply (~15-100x traffic savings reported) before committing to a live A/B test, closing the loop.

## Sources
- Joachims et al., Unbiased LTR with Biased Feedback (WSDM'17): https://www.cs.cornell.edu/people/tj/publications/joachims_etal_17a.pdf
- Wang et al., Position Bias Estimation in Personal Search (regression-EM): https://research.google/pubs/position-bias-estimation-for-unbiased-learning-to-rank-in-personal-search/
- Agarwal et al., Position Bias without Intrusive Interventions: https://arxiv.org/pdf/1806.03555
- Aslanyan & Porwal (eBay), Position Bias in eCommerce Search: https://arxiv.org/abs/1812.09338
- Oosterhuis, Doubly-Robust Estimation for Position Bias: https://arxiv.org/pdf/2203.17118
- DCG as Off-Policy Eval Metric (KDD'24): https://arxiv.org/html/2307.15053v3
- Double Clipping (Amazon): https://arxiv.org/pdf/2309.01120
- Off-policy Evaluation for Payments at Adyen: https://arxiv.org/html/2501.10470
- Interleaving + Counterfactual Eval for Airbnb Search Ranking: https://arxiv.org/pdf/2508.00751
- Gupta et al., Policy-Aware Unbiased LTR for Top-k: https://arxiv.org/pdf/2005.09035
