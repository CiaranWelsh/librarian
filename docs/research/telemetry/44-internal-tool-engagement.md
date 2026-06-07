# Topic 44: Engagement / Retention Metrics for Internal Developer Tools

## Findings
Practitioners measure adoption as a **funnel**, not a single number. Glean (enterprise search/RAG) exposes a canonical internal-tool funnel: **Coverage** = signups/employees, **Activity** = MAU/signups, **Habit/Stickiness** = WAU/MAU. "Active" is defined as taking *any one of a specific set of value-actions* (a query, chat, AI answer, agent run) — never just "opened it." For internal/productivity tools, **WAU is more telling than DAU** because usage is weekly, not daily, and engagement is best measured at the **team/service level**, not per-user (Slack: solo users barely retained; teams with 2,000+ messages held 93% weekly retention). SPACE/DevEx warn that activity counts alone mislead — pair them with satisfaction and outcome signals. IR research (Joachims, Radlinski) shows clicks/reformulations are biased as *absolute* relevance but reliable as *relative* preferences, and that query *chains* (reformulation sequences) yield far more learning signal than single queries.

## What to log
- Per-event: anonymized user/team id, timestamp, query text, latency, result count.
- Value-actions defining "active": query issued, result opened/cited, answer copied/used, agent run.
- Implicit signals: clickthrough (which `source_id` opened), dwell, copy/cite, query reformulation within a session, abandonment (zero-result / no-open).
- Explicit feedback: thumbs up/down, 1-5 rating, optional comment (Langfuse "scores").
- Session/chain id to reconstruct query chains.

## Metrics
- **Coverage, Activity (MAU/signups), Stickiness (WAU/MAU)** — adoption funnel.
- DAU/WAU/MAU on value-actions; per-team retention cohorts.
- Click-through rate, abandonment/zero-result rate, reformulation rate, mean queries-to-success.
- Feedback score rate + positive ratio; latency/cost (Langfuse "metrics").
- Time-to-value / time saved tied to outcomes.

## How it is used
Segment traces by low score or high abandonment to find failing queries; mine **query chains** to build relative-preference judgments and retrain ranking (learning-to-rank from implicit feedback); use feedback scores as eval ground truth (LLM-as-judge / Ragas faithfulness, context precision); track stickiness cohorts to spot drop-off; set internal baselines and beat them (don't chase social-app benchmarks).

## Sources
- Glean Insights overview (coverage/activity/stickiness, active defs): https://docs.glean.com/administration/insights/overview
- DAU/MAU stickiness, "team as unit of engagement": https://clevertap.com/blog/dau-vs-mau-app-stickiness-metrics/ , https://userpilot.com/blog/dau-wau-mau/
- SPACE framework (ACM Queue): https://queue.acm.org/detail.cfm?id=3454124
- DevEx framework (ACM Queue): https://queue.acm.org/detail.cfm?id=3595878
- Langfuse user feedback (explicit vs implicit scores): https://langfuse.com/docs/observability/features/user-feedback
- Joachims/Granka, accuracy of implicit feedback (clicks & reformulations): https://dl.acm.org/doi/10.1145/1229179.1229181
- Radlinski & Joachims, Query Chains: learning to rank from implicit feedback: https://www.cs.cornell.edu/~tj/publications/radlinski_joachims_05a.pdf
