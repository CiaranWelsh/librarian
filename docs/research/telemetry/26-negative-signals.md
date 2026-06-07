# 26 — Negative Signals: Re-query, Rephrase, Quick-Back, Early-Exit

## Findings
Practitioners treat post-result *behaviour* as implicit dissatisfaction feedback, since
clicks alone don't measure success and LLM answers "fail silently" (HTTP 200 on a
hallucination). Three converging traditions:
- **IR/search**: query reformulation is a standard online quality metric — "if a user is
  satisfied they are less likely to reformulate." Reformulation is detected when two queries
  share goal/mission and Jaccard similarity > ~0.3 (Microsoft/USPTO). *Bad abandonment*
  (no click, then reformulate) signals failure; *good abandonment* (answer on the SERP) does not.
- **RAG products**: Perplexity tracks downvotes/skips and de-indexes weak sources within ~1 week.
  Glean watches "repeated rephrase patterns, low citation click-through, high edit distance
  between draft and final, tool error codes, timeouts, spike in human takeovers."
- **LLM observability** (Langfuse/LangSmith): a *regenerate/retry* is logged as an implicit
  negative score on the trace; failing traces are filtered and drilled into (retriever span)
  to separate retrieval failure from generation failure.

## What to log
- Re-query within session: query pair + Jaccard/edit-distance + goal-same flag.
- Rephrase / regenerate / retry events (count per session).
- Quick-back / pogo-stick: result opened then abandoned with short dwell (<~10s, intent-dependent).
- Early-exit / abandonment: result shown, no open, session ends — distinguish good vs bad.
- Citation click-through, copy/accept events, human-takeover/escalation, tool errors/timeouts.

## Metrics
- Reformulation rate; bad-abandonment rate; pogo-stick / short-click rate.
- "Last longest click" (NavBoost-style good-vs-bad click split).
- Session success rate; citation CTR; regenerations-per-answer (saturates ~3 loops).

## How it is used
- Train rankers to minimise reformulation likelihood (query-chain learning, Radlinski & Joachims).
- Surface failing traces → diagnose retriever vs generator → fix chunking/embeddings/prompt.
- De-index/down-weight low-engagement sources; auto-expand+retry query on flagged answers.
- Each fix = one experiment, one metric delta, one eval-set update (Glean); risk-tiered release.
- Aggregate across sessions (never single-visit); good-abandonment guards against false negatives.

## Sources
- Glean — AI feedback loops: https://www.glean.com/perspectives/how-to-incorporate-ai-feedback-loops-for-continuous-learning
- Hassan et al., "Beyond Clicks: Query Reformulation as a Predictor of Search Satisfaction" (CIKM'13): https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/Hassan_CIKM13a.pdf
- Chen et al., "Query Reformulation Behavior in Web Search" (WWW'21): https://xuanyuan14.github.io/files/WWW21chen.pdf
- Huang & Efthimiadis, "Reformulation Strategies in Web Search Logs" (CIKM'09): https://jeffhuang.com/papers/Reformulation_CIKM09.pdf
- USPTO patent (reformulation/abandonment rate via Jaccard, goal/mission): https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/10769547
- Langfuse — User Feedback: https://langfuse.com/docs/observability/features/user-feedback
- Perplexity answer pipeline (engagement signals, ~1-week de-index): https://ziptie.dev/blog/how-perplexity-ai-answers-work/
- NavBoost good/bad clicks, last-longest-click: https://www.hobo-web.co.uk/navboost-how-google-uses-large-scale-user-interaction-data-to-rank-websites/
- Pogo-sticking & dwell time thresholds: https://trafficsoda.com/understanding-bounce-rate-long-clicks-and-pogo-sticking
