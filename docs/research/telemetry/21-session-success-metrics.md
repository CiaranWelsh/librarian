# Topic 21: Session-level success/failure analysis

## Findings
A single query rarely captures intent, so IR and RAG practitioners evaluate the whole **session**. Microsoft Research showed search success can be modelled purely from behaviour, independent of document relevance, with query **reformulation** and **abandonment** as the strongest signals. Abandonment is split into *good* (need met on the result page, no click) vs *bad* (frustration) — conflating them mis-scores success, so models add session/context/(on mobile) gesture features to tell them apart. In RAG/LLM systems the **trace** (full lifecycle of a request) is the unit of analysis; sessions group multi-turn traces (Langfuse, LangSmith). Explicit feedback is sparse, so implicit behaviour (rephrase, copy, abandon, follow-up question) acts as a proxy. Conversational-AI practice distinguishes **deflection** (no human, easily gamed) from **containment** and **resolution** (issue actually solved), watching **re-contact rate** to catch silent failure. Glean treats a click on a low-ranked result as "rank higher" evidence but guards against overfitting one data point; enterprise signals are sparse so personalization compensates.

## What to log
- Per session: ordered query/turn sequence, reformulations (edit-distance vs prior query), time-to-next-query, end-of-session marker.
- Per result/answer: clicks, dwell time (>~30s = good click), copy/save, citation click-through.
- Abandonment with no follow-up action (flag good vs bad).
- Explicit feedback: thumbs up/down, free-text, annotation-queue ratings.
- Trace metadata grouping turns into a session id.

## Metrics
- Session success rate (behaviour-modelled), good vs bad abandonment rate, reformulation rate, time-to-next-query.
- Containment / resolution rate, re-contact rate, escalation rate, CSAT, cost-per-resolution.
- RAG Triad: context relevance, groundedness/faithfulness, answer relevance.

## How it is used
- Mine bad-abandonment→reformulation pairs as preference data to retrain ranking (MS found 265k such sessions).
- Re-rank: promote clicked low-ranked results, personalize from session/knowledge-graph context.
- Sample 1-5% of traces through LLM-as-judge to track drift; route low-success sessions to human annotation queues feeding eval datasets.
- Intent-level containment breakdowns reveal which query classes need work.

## Sources
- Hassan et al., reformulation as satisfaction predictor: https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/Hassan_CIKM13a.pdf
- Good abandonment in success metrics: https://www.microsoft.com/en-us/research/wp-content/uploads/2016/10/shp0291-khabsaA.pdf
- Good abandonment on mobile (gestures): https://www.microsoft.com/en-us/research/wp-content/uploads/2017/05/williams_www2016_good_abandonment.pdf
- Context-aware abandonment prediction (SIGIR): http://sonyis.me/paperpdf/sigir226-song.pdf
- Session effectiveness (C/W/L): https://www.sciencedirect.com/science/article/abs/pii/S0306457321000996
- Langfuse observability (traces/sessions): https://langfuse.com/docs/observability/overview
- Langfuse RAG observability & evals: https://langfuse.com/blog/2025-10-28-rag-observability-and-evals
- Braintrust LLM observability (implicit signals, sampling): https://www.braintrust.dev/articles/llm-observability-guide
- Containment vs deflection vs resolution: https://alhena.ai/blog/ai-chatbot-containment-vs-deflection-rate/
- Intent-level containment: https://www.typewise.app/blog/containment-rate-predicts-success
- Glean: learning from feedback, sparse signals: https://www.glean.com/blog/enterprise-search-is-hard-why-its-so-behind-and-what-itll-take-to-catch-up
