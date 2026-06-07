# Topic 22: Explicit Feedback (thumbs/ratings) Collection Design & Response Bias

## Findings
Explicit feedback (thumbs up/down, 1-5 stars, comments) is cheap to ship but
suffers two structural problems. (1) **Very low, biased response rates**: a tiny
thumbs widget hidden in a corner yields ~0.1% responses; a prominent, contextual
prompt reaches ~0.4-0.5% (5x more data). A clinical RAG deployment got votes on
only 7.5% of conversations. (2) **Self-selection / positivity skew**: raters skew
to extreme opinions and momentary emotion; survey-style binary prompts invite
acquiescence. IR research shows letting users *pick* what to rate is harmful
(they over-rate the top result); present a fixed set instead. RLUF (Meta, 2025)
found Love/Thumbs-Up correlate with retention but encourage *positive* responses,
so naively training on them induces sycophancy. Products treat explicit feedback
as one low-volume signal to be fused with abundant implicit signals, not as sole
ground truth. Langfuse, Glean, and Perplexity all log it linked to a trace/turn.

## What to log
- Rating value: boolean/categorical (thumbs) or numeric (1-5); store typed, not free text, so it aggregates.
- Identifiers: trace_id / turn_id / session_id / message_id linking feedback to the exact answer + retrieved sources.
- Optional free-text comment + reason category (irrelevant, wrong, incomplete, hallucinated).
- Context: query, model/prompt version, retrieved doc IDs, latency, position of widget.
- Per-citation feedback (Perplexity-style) and a corrected-answer field when offered.
- Capture timestamp + idempotency key (allow rating to be updated).

## Metrics
- Feedback/response rate (votes per answer) and its lift by UI placement.
- Positive rate / CSAT, thumbs-down rate, NPS-style score.
- Score distribution and trend over time; per-model/prompt A/B comparison.
- Correlation of feedback with retention / task success (signal validation).

## How it is used
Filter low-rated traces to triage failures and build eval datasets; query+rejected-doc
pairs become negatives for retriever/embedding fine-tuning; downvoted sources get
de-ranked (Perplexity drops them within ~1 week); positive sets seed reward models
(RLUF/RLHF). Always debias: fuse with implicit signals, weight by propensity, and
guard against positivity/sycophancy via multi-objective checks.

## Sources
- Langfuse User Feedback / Scores: https://langfuse.com/docs/observability/features/user-feedback , https://langfuse.com/docs/evaluation/scores/overview
- 567-labs, Systematically Improving RAG (response-rate numbers): https://567-labs.github.io/systematically-improving-rag/workshops/chapter3-1/
- Glean, feedback to improve search quality: https://docs.glean.com/user-guide/basics/improve-search-quality-by-giving-feedback-on-results
- Perplexity source ranking / engagement signals: https://ziptie.dev/blog/how-perplexity-ai-answers-work/
- On bias problem in relevance feedback (CIKM): https://dl.acm.org/doi/10.1145/2063576.2063866
- Self-selection in rating (modify ranking via implicit feedback, USPTO): https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/8661029
- Reinforcement Learning from User Feedback (RLUF, arXiv): https://arxiv.org/html/2505.14946v1
- Clinical RAG retrospective (7.5% feedback): https://www.medrxiv.org/content/10.64898/2026.01.26.26344757.full.pdf
- Acquiescence bias: https://en.wikipedia.org/wiki/Acquiescence_bias
