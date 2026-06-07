# Topic 32: Building golden/evaluation sets from real production queries

## Findings
- Real production queries beat synthetic/public benchmarks: high public-benchmark scores don't transfer to domain traffic, and synthetic sets miss the long-tail complexity and linguistic diversity of real queries (Statsig, Microsoft DS).
- Standard operating rule across LLM-observability tools: **every production failure becomes a golden-set entry** — the set is a growing regression suite, not a one-shot artifact (Langfuse, LangSmith).
- Tooling supports this directly: Langfuse/LangSmith let you select traces with poor user feedback or low judge scores and add them to a dataset (manually, by CSV, or via auto **run rules**); SMEs can edit inputs/reference outputs in annotation queues before promotion. LangSmith Engine auto-clusters failing traces into named issues and pulls them into ground-truth datasets.
- Classic IR analogue: TREC **pooling** (Sparck Jones & van Rijsbergen) builds reusable qrels by merging the top-k from many systems for human judging; web engines instead infer relevance at scale from click-through-rate per impression.
- Query traffic is exponential (head/torso/tail). Pure frequency-weighted sampling misses the tail; use **stratified sampling** by frequency (Yahoo!: 1000 queries, equal probability across 3 strata), and oversample torso/tail for diagnostic coverage.

## What to log
- Raw query, retrieved doc/passage IDs + ranks, generated answer, timestamp/session.
- User feedback as scores (thumbs up/down, `user_satisfaction`) keyed to a trace ID.
- Provenance/governance metadata: reviewer identity, rubric version, consent/risk tags, dataset version.
- Stratification keys: query frequency, length, intent/cluster, difficulty (# relevant items).
- Abstention / "I don't know" cases and ambiguous queries flagged explicitly.

## Metrics
- Retrieval: precision, recall, nDCG/MAP/MRR (note: set-based metrics may fit RAG better than rank-discounted ones).
- Answer: correctness, groundedness/faithfulness.
- Judge calibration: judge-vs-human agreement (target 75-90%), Cohen's kappa; report accuracy AND kappa, not one metric.
- Inter-annotator agreement on labels themselves (kappa) before a set is trusted; <80% agreement => fix the rubric.
- Per-stratum metrics (aggregate hides tail regressions); correlation of golden-set score with business KPIs.

## How it is used
- Calibrate an LLM-judge against the human golden set, freeze a decision threshold, then run the judge on 5-20% of live traffic.
- Judge-flagged or user-flagged traces -> human review queue -> verified failures appended to golden set (closed loop).
- Golden set acts as a regression gate on every prompt/model/retrieval change (e.g., block if factuality < baseline).
- Scheduled recalibration + drift monitor (monthly kappa check, quarterly refresh swapping stale entries for fresh traces).
- Silver-to-gold promotion: cheap auto/simulation labels grown then human-reviewed into gold to control cost.

## Sources
- https://www.statsig.com/perspectives/golden-datasets-evaluation-standards
- https://medium.com/data-science-at-microsoft/the-path-to-a-golden-dataset-or-how-to-evaluate-your-rag-045e23d1f13f
- https://langfuse.com/docs/evaluation/experiments/datasets
- https://docs.langchain.com/langsmith/manage-datasets-in-application
- https://www.langchain.com/blog/introducing-langsmith-engine
- https://www.langchain.com/articles/llm-as-a-judge
- https://tsapps.nist.gov/publication/get_pdf.cfm?pub_id=51229
- https://bonsai.io/blog/elasticsearch-opensearch-query-sampling-relevancy/
- https://futureagi.com/blog/llm-as-judge-best-practices-2026
- https://booking.ai/llm-evaluation-practical-tips-at-booking-com-1b038a0d6662
