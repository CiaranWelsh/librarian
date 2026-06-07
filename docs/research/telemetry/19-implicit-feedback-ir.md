# Topic 19: Implicit Feedback in IR — Clicks, Dwell, Skips, Scroll

## Findings
Implicit feedback (clicks, dwell time, skips, scroll, query reformulation) is the dominant cheap relevance signal in production search. Joachims et al. established that raw clicks are informative but **biased**: relative pair-preferences ("clicked result B was skipped over A") are far more reliable than absolute clicks. Three biases dominate — **position bias** (top results clicked regardless of relevance), **selection bias** (unseen results get no signal), and **trust bias**. Untreated, these create a self-reinforcing feedback loop. The "long click" concept (Google NavBoost, Bing) treats dwell after a click as a satisfaction proxy; a quick bounce-back ("pogo-sticking") signals dissatisfaction. In RAG/LLM tools, the same idea reappears as implicit signals: copy-output, accept-suggestion, regenerate/retry, follow-up query, and session abandonment — distinct from explicit thumbs up/down.

## What to log
- Per-impression: query, result/source IDs, **position shown**, full result set displayed (for selection bias).
- Click events with timestamps; **dwell time** on the clicked result (click-to-return interval).
- Skips (results ranked above a click that were not clicked) and scroll depth.
- Query reformulations and follow-up queries within a session; absence/return time.
- Session ID to link the above; for RAG: copy, regenerate, citation click-through, abandonment.

## Metrics
- CTR and **debiased CTR** variants: IPW-CTR (inverse propensity), COEC (clicks over expected clicks), Empirical-CTR (position-weighted).
- Long-click / satisfied-click rate (dwell > threshold, often ~30s but query-intent dependent).
- Pogo-stick / short-click rate; skip-above-click pair counts.
- Estimated position-bias curve (propensities); reformulation rate; absence time.

## How it is used
Click models (cascade, PBM, DBN) apply the examination hypothesis to separate examination-by-position from true relevance, yielding debiased labels. Position propensities are estimated via result randomization, A/B tests, natural log variation, or regression-EM (no randomization, per Google personal search). Debiased signals then either generate Learning-to-Rank labels or serve as CTR features (consistently among strongest features). RAG observability tools (Langfuse, LangSmith, Arize/Phoenix) attach feedback as scores on a trace ID, combine with LLM-as-judge, and feed a continuous loop tightening retrieval/generation. Validate generalization under covariate shift (CMIP metric).

## Sources
- Joachims et al., Evaluating Accuracy of Implicit Feedback: https://www.cs.cornell.edu/people/tj/publications/joachims_etal_07a.pdf
- Eugene Yan, Measure & Mitigate Position Bias: https://eugeneyan.com/writing/position-bias/
- Google, Position Bias Estimation for ULTR in Personal Search: https://research.google/pubs/position-bias-estimation-for-unbiased-learning-to-rank-in-personal-search/
- Position bias in features (CTR features, IPW/COEC): https://arxiv.org/pdf/2402.02626
- Offline Metric for Debiasedness of Click Models (CMIP, covariate shift): https://arxiv.org/html/2304.09560v3
- Reweighting Clicks with Dwell Time in Recommendation: https://arxiv.org/pdf/2209.09000
- Biases in LTR and three approaches: https://datadojo.dev/2021/04/29/biases-in-learning-to-rank-models-and-three-approaches-to-deal-with-them/
- Langfuse User Feedback (explicit vs implicit): https://langfuse.com/docs/observability/features/user-feedback
- LangSmith feedback logging tutorial: https://docs.langchain.com/langsmith/observability-llm-tutorial
- Backlinko, Dwell Time / long click: https://backlinko.com/hub/seo/dwell-time
