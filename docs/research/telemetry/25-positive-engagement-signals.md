# Topic 25: Positive Engagement Signals (follow-ups, copy/paste, citation clicks, save/share)

## Findings
Production search and RAG systems treat positive engagement as **implicit relevance feedback** — cheap, abundant, user-centered, but biased (Joachims, Agichtein). The strongest signal is not the raw click but the **"satisfied/long click"**: a click followed by high dwell time and no quick bounce-back. Perplexity feeds citation clicks plus likes/dislikes into ranking models and drops sources that are repeatedly skipped or downvoted within ~a week (vendor analysis, not official). Glean's self-tuning ranker learns from clicks, edits, comments, saves and re-uses, fine-tuning per-customer language models on "clicks on documents within queries," while warning that enterprise feedback is sparse and must not be overfit. LLM-observability tools (Langfuse, LangSmith, Arize) record explicit (thumbs/stars/comments) and implicit (copy output, accept suggestion, dwell, retry) signals as a unified **score object**.

## What to log
- Citation/source click (which retrieved item the user opened) + position (for bias correction)
- Dwell time on opened source; "satisfied click" flag (dwell over threshold, no bounce-back)
- Copy/paste of an answer or excerpt; save/bookmark; share
- Follow-up query in same session (reformulation vs. continuation)
- Explicit thumbs/star + optional comment, linked by `trace_id`/query_id
- Session id, query id, candidate ranks shown (impressions) for CTR denominators

## Metrics
- CTR (clicks / impressions); satisfied-CTR / long-click rate
- Click-skip ratio; average/mean click rank; skip rate
- Copy rate, save rate, follow-up/continuation rate, session abandonment rate
- Per-source engagement score driving rerank; MRR/nDCG offline checks

## How it is used
1. Aggregate clicks (more reliable than single events) into pairwise/relevance signals for learning-to-rank, correcting position bias (counterfactual / unbiased LTR).
2. Compare ranker variants with **interleaving** (each user is own control; Airbnb reached A/B conclusions on ~4% of traffic) before A/B confirming business impact.
3. Demote/drop low-engagement sources; boost high-engagement ones; build eval datasets and LLM-judge ground truth from logged feedback (Langfuse).

## Sources
- Joachims et al., Unbiased Learning-to-Rank with Biased Feedback: https://arxiv.org/abs/1608.04468
- Radlinski/Joachims, How Does Clickthrough Data Reflect Retrieval Quality (interleaving): https://www.cs.cornell.edu/people/tj/publications/radlinski_etal_08b.pdf
- Modeling Dwell Time to Predict Click-level Satisfaction (ACM): https://dl.acm.org/doi/pdf/10.1145/2556195.2556220
- Langfuse User Feedback docs (explicit/implicit scores schema): https://langfuse.com/docs/observability/features/user-feedback
- Langfuse Scores overview: https://langfuse.com/docs/evaluation/scores/overview
- Perplexity engagement/citation feedback (vendor analysis): https://ziptie.dev/blog/how-perplexity-ai-answers-work/ and https://discoveredlabs.com/blog/perplexity-optimization-how-to-get-cited-linked-2026
- Glean enterprise search ranking from clicks/activity: https://www.glean.com/blog/enterprise-search-is-hard-why-its-so-behind-and-what-itll-take-to-catch-up
- Airbnb interleaving vs A/B: https://airbnb.tech/data/beyond-a-b-test-speeding-up-airbnb-search-ranking-experimentation-through-interleaving/
- Search relevance click metrics (CTR, skip ratio, click rank): https://www.coveo.com/blog/measuring-search-relevance/
