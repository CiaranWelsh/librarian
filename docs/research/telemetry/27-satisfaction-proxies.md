# 27 - Proxy Metrics for User Satisfaction in Search/Conversational Systems

## Findings
Explicit feedback (thumbs, ratings) is the gold standard but sparse — most users never give it — so production systems lean on **implicit behavioral proxies** collected at scale. IR research (Joachims; Fox 2005; Hassan CIKM'13; Kim WSDM'14) establishes that **raw clicks are a weak proxy**: many clicked queries still fail. Refinements: **dwell time** distinguishes SAT vs DSAT clicks (a "satisfied click" is commonly ~30s+ dwell that ends the query); **query reformulation** signals unmet need and can outrank click signals. Combinations beat any single signal. LLM-observability tools (Langfuse, LangSmith, Arize) and RAG products (Perplexity, Glean) adopt the same playbook: log behavioral proxies, plus run **LLM-as-judge** on a 1-5% sample for scalable quality scoring. Caveat: relevance ≠ satisfaction; benchmark scores (RAGAS/TruLens) poorly predict real adoption — watch behavior instead.

## What to log
- Explicit: thumbs up/down, star rating, free-text comment.
- Clicks on cited sources / results; click rank/position.
- Dwell time on answer; time-to-first-token (latency).
- Query reformulation / rephrase; abandonment.
- Copy-to-clipboard, accepted suggestion, "more like this".
- Follow-up query + its sentiment ("thanks" vs "no, I meant…"); escalation to human.
- Session: duration, # result sets, exit type, task/ticket closure, next-day return.

## Metrics
- SAT/DSAT click rate; SAT precision, DSAT recall.
- Reformulation rate, abandonment rate, click-through rate.
- Dwell-time distributions (distance to SAT/DSAT clusters).
- Escalation rate (leading indicator of degradation).
- LLM-judge sentiment/satisfaction/relevance scores; next-day retention ("Retentive Relevance").

## How it is used (feedback loop)
- Store every signal as a **score linked to a trace** (Langfuse schema: `traceId`, `name`, `value`, `comment`) so feedback filters/joins to specific interactions.
- Correlate scores with traces to surface failing queries; build annotation queues; use feedback as ground-truth labels for evals.
- Sample 1-5% of traffic through LLM-as-judge to track drift (relevance down? escalations up?).
- Mine implicit signals into training labels; rank-from-feedback; trigger alerts on rising DSAT/escalation; A/B compare prompt/model versions in production.

## Sources
- Hassan et al., "Beyond Clicks: Query Reformulation as a Predictor of Search Satisfaction" (CIKM'13): https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/Hassan_CIKM13a.pdf
- Kim et al., "Modeling Dwell Time to Predict Click-level Satisfaction" (WSDM'14): https://dl.acm.org/doi/pdf/10.1145/2556195.2556220
- Radlinski/Joachims, "Query Chains: Learning to Rank from Implicit Feedback": https://arxiv.org/pdf/cs/0605035
- Langfuse, User Feedback docs: https://langfuse.com/docs/observability/features/user-feedback
- LLM observability comparison (LangSmith/Langfuse/Arize): https://research.aimultiple.com/llm-observability/
- RAG user-feedback / continuous-improvement loop: https://apxml.com/courses/optimizing-rag-for-production/chapter-6-advanced-rag-evaluation-monitoring/user-feedback-rag-improvement
- "RAG evaluation: why user feedback beats automated metrics": https://amitkoth.com/rag-evaluation-metrics/
- Feedback collection (Systematically Improving RAG): https://567-labs.github.io/systematically-improving-rag/workshops/chapter3-1/
