# 20 - Query Reformulation & Abandonment as Dissatisfaction Signals

## Findings
Web-search IR established reformulation as an implicit dissatisfaction signal: if a user re-issues a similar query, the prior results likely failed (Hassan et al., CIKM 2013). But the signal is ambiguous. **Abandonment** (no click) can mean failure *or* "good abandonment" — the answer appeared on the result page (Williams/Khabsa, Microsoft). Likewise, reformulation can reflect natural task refinement or intent shift, not dissatisfaction (Chen et al., WWW 2021). So practitioners combine reformulation with clicks/dwell rather than using it alone, since some dissatisfied users abandon *without* reformulating. In RAG/LLM chat, the analogue is the **follow-up query**: Kim et al. (2024) flag Excluding-Condition, Substituting-Condition, and Criticizing-Response turns as low-satisfaction; BloomIntent defines "negative reformulation count." Retry/regenerate and thumbs-down are first-class implicit-negative signals, but research warns they are noisy as a training label (arXiv 2507.23158).

## What to log
- Query pairs in a session: text, edit distance / term overlap, terms added/removed, generalization vs specialization.
- Time-to-next-query; click/no-click before reformulation; dwell on any opened result.
- Session boundaries (reformulation vs new task vs abandonment).
- For chat/RAG: follow-up turns, retry/regenerate clicks, thumbs-down + reason category ("inaccurate/incomplete/not relevant"), copy-paste, retrieved-context + relevance scores.

## Metrics
- Query abandonment rate; distinguished into bad vs good abandonment.
- Reformulation rate; reformulations-per-session; negative-reformulation count.
- Reformulation-as-feature in a satisfaction classifier (query length & reformulation negatively correlate; time-to-next-query positively correlates with satisfaction).
- Session success / good-abandonment-adjusted success rate.

## How it is used
Feed reformulation/abandonment into satisfaction prediction models that complement offline relevance judgments at scale; mine "bad abandonment then reformulation" pairs as training data for ranking (negative = abandoned doc, positive = next clicked doc). RAG uses ranking feedback (RaFe) and retry rewards (ReZero) to learn query rewriting. Observability tools (Langfuse, LangSmith, Arize Phoenix) trace turns and surface regenerate/thumbs-down clusters to prioritize fixes by frequency × impact.

## Sources
- Hassan et al., "Beyond Clicks: Query Reformulation as a Predictor of Search Satisfaction," CIKM 2013 — https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/Hassan_CIKM13a.pdf
- Williams et al., "Detecting Good Abandonment in Mobile Search," WWW 2016 — https://www.microsoft.com/en-us/research/wp-content/uploads/2017/05/williams_www2016_good_abandonment.pdf
- Khabsa et al., "Learning to Account for Good Abandonment in Search Success Metrics" — https://www.microsoft.com/en-us/research/wp-content/uploads/2016/10/shp0291-khabsaA.pdf
- Chen et al., "Towards a Better Understanding of Query Reformulation Behavior in Web Search," WWW 2021 — https://dl.acm.org/doi/fullHtml/10.1145/3442381.3450127
- Song et al., "Context-Aware Web Search Abandonment Prediction," SIGIR — http://sonyis.me/paperpdf/sigir226-song.pdf
- Kim et al., "Using LLMs to Investigate Correlations of Conversational Follow-up Queries with User Satisfaction," 2024 — https://arxiv.org/abs/2407.13166
- "BloomIntent: Automating Search Evaluation with LLM-Generated Fine-Grained User Intents," UIST 2025 — https://dl.acm.org/doi/10.1145/3746059.3747677
- "User Feedback in Human-LLM Dialogues: ... Noisy as a Learning Signal," arXiv 2507.23158 — https://arxiv.org/html/2507.23158v2
- "RaFe: Ranking Feedback Improves Query Rewriting for RAG," arXiv 2405.14431 — https://arxiv.org/pdf/2405.14431
- "ReZero: Enhancing LLM search ability by trying one-more-time," arXiv 2504.11001 — https://arxiv.org/pdf/2504.11001
- "Teaching the model: Designing LLM feedback loops," VentureBeat — https://venturebeat.com/ai/teaching-the-model-designing-llm-feedback-loops-that-get-smarter-over-time
