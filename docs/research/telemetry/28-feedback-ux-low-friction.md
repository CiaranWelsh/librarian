# Topic 28: Low-friction feedback UX that minimizes response bias

## Findings

Practitioners converge on a **tiered, mostly-implicit** design. Layer 1 is a zero-extra-click signal (thumbs up/down or "Was this helpful?") inline under each answer; layer 2 reveals structured reason chips only on a negative (Inaccurate / Not relevant / Incomplete / Wrong tone); layer 3 is an optional free-text box. Most value comes from implicit behaviour, not the rating itself.

Binary thumbs are deliberately the *first* layer only: they are familiar and cheap, but lossy and bias-prone. The strongest signals are **implicit and non-leading** — copy, regenerate, retry, follow-up query, dwell, abandonment, and user edits of the output (free ground-truth "this is what I wanted"). Code-assist analogues: Copilot acceptance rate (~30%) and retention (~88% of accepted code survives).

Response-bias mitigations from survey science apply directly to the widget: ask construct-focused questions ("How relevant?") not agree/disagree statements; avoid evaluative adjectives and leading framing (Calendly found "redesigned from your feedback" lifted positive ratings +15pts); present scales ascending; offer neutral/opt-out options; one idea per prompt.

IR literature warns that logged implicit feedback (clicks/opens) carries **position, trust, and selection bias**. The standard fix is the position-based click model + inverse-propensity weighting (Joachims 2017) to recover unbiased relevance, with caveats: high variance from tiny propensities, zero-exposure items.

## What to log

- Per-answer rating (boolean/categorical) + structured reason category on negatives, attached to the exact trace/result IDs (Langfuse `score`: trace_id, name, value, comment).
- Implicit events: copy, regenerate, retry, follow-up query, result open + dwell, session abandonment.
- Edits/corrections of the answer (high-value training pairs).
- Context for debiasing: result rank/position shown, which sources surfaced, query, latency.

## Metrics

- Helpful rate / categorical score distribution; reason-category breakdown.
- Implicit: acceptance/copy rate, regenerate & retry rate, abandonment rate, follow-up-query rate, dwell.
- Propensity-weighted (IPW) relevance to remove position bias.
- Bias diagnostics: acquiescence/skew checks; balanced-scale cancellation.

## How it is used

- Negative + reason routes triage; flagged query/rejected-doc pairs become contrastive negatives to retrain embeddings/rerankers.
- Edits/corrections feed fine-tuning and few-shot exemplars.
- Implicit signals detect silent failures and auto-score traces; LLM-as-judge scales evaluation (watch its own length/position bias).
- IPW on logged clicks drives unbiased learning-to-rank; metrics gate releases.

## Sources

- Winder AI, User Feedback in LLM Apps: https://winder.ai/user-feedback-llm-powered-applications/
- VentureBeat, Designing LLM feedback loops: https://venturebeat.com/ai/teaching-the-model-designing-llm-feedback-loops-that-get-smarter-over-time
- Nebuly, Explicit & implicit LLM feedback: https://www.nebuly.com/blog/explicit-implicit-llm-user-feedback-quick-guide
- Microsoft DS, Beyond thumbs up/down: https://medium.com/data-science-at-microsoft/beyond-thumbs-up-and-thumbs-down-a-human-centered-approach-to-evaluation-design-for-llm-products-d2df5c821da5
- Langfuse, User Feedback (scores schema): https://langfuse.com/docs/observability/features/user-feedback
- APXML, User feedback in RAG: https://apxml.com/courses/optimizing-rag-for-production/chapter-6-advanced-rag-evaluation-monitoring/user-feedback-rag-improvement
- Pistis-RAG (implicit signals: copy/regenerate/dislike): https://arxiv.org/pdf/2407.00072
- Joachims et al., Unbiased LTR / IPW (PBM, position bias): https://arxiv.org/pdf/1804.05938
- Zoominfo Copilot study (acceptance ~33%, retention ~88%): https://arxiv.org/html/2501.13282v1
- Mailchimp, Avoiding leading questions: https://mailchimp.com/resources/leading-questions/
- GESIS, Response biases in surveys (acquiescence): https://www.gesis.org/fileadmin/admin/Dateikatalog/pdf/guidelines/response_biases_standardized_surveys_bogner_landrock_2016.pdf
