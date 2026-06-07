# Topic 40: Closing the Loop -- Telemetry to Corpus/Model Updates

## Findings
Real systems treat telemetry as a routing problem: each feedback *category* maps to a
specific fix (corpus refresh, embedding fine-tune, reranker, or prompt change). The
canonical IR pattern (Joachims; Radlinski & Joachims "Query Chains", KDD'05; deployed in
the Osmot/Nutch search engine) turns clickthrough + query-reformulation logs into
relevance *preferences* to retrain a ranking SVM, with random result-shuffling to remove
position bias (Microsoft recency-ranking study). Modern RAG products repeat this: Glean
self-tunes ranking from clicks + activity-graph signals (file opens, authorship, team),
reporting ~20% search-quality gain over 6 months (vendor-reported). RAG gap-analysis
research (arXiv 2509.13626) mines real conversations to find *content gaps*; directed
augmentation hit ~95% of reference quality while cutting authoring work ~58-82% vs random
corpus expansion. LLM-observability tools (LangSmith, Langfuse, Arize Phoenix) operationalize
the loop: trace -> sample failures -> annotation queue -> versioned "golden" eval dataset ->
regression tests before shipping.

## What to log
- Query, response, retrieved doc IDs + scores, reranker order, model/prompt version, A/B variant.
- Explicit: thumbs up/down, star/NPS, categorized issue ("outdated", "irrelevant", "hallucination", "missing sources"), free-text, span highlights.
- Implicit: clicks on citations, copy-paste, dwell time, query reformulation/abandonment, follow-ups, downstream task completion.
- IDs/context: user, session, timestamp (for chaining + slicing).

## Metrics
- Retrieval: Recall@k, MRR/nDCG, click-through, abandonment & reformulation rate, oracle retrieval gap (doc/page/chunk).
- Answer: LLM-as-judge faithfulness/relevance, thumbs-up rate, correlation of eval score with user feedback.
- Drift/health: embedding/query-distribution drift, corpus staleness, per-slice quality vs baseline.

## How it is used
1. Diagnose: split *content gap* (answer absent -> augment corpus) from *retriever gap* (present but missed -> fine-tune/rerank).
2. Build training data: query/positive/negative triplets from feedback; BM25-mined hard negatives; contrastive fine-tune (last resort -- watch catastrophic forgetting; prefer reranker for near-misses).
3. Curate golden eval sets from flagged failures via annotation queues; run online + offline evals; regression-gate deploys; re-measure.

## Sources
- Radlinski & Joachims, Query Chains (KDD'05): https://www.cs.cornell.edu/~tj/publications/radlinski_joachims_05a.pdf
- Microsoft, Online Learning for Recency Search Ranking: https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/published-4.pdf
- Glean (ZenML LLMOps DB) -- IR + LLM enterprise search: https://www.zenml.io/llmops-database/building-robust-enterprise-search-with-llms-and-traditional-ir
- Glean, why enterprise search is hard: https://www.glean.com/blog/enterprise-search-is-hard-why-its-so-behind-and-what-itll-take-to-catch-up
- Mind the Gap (arXiv 2509.13626) -- KB gap analysis from conversations: https://arxiv.org/html/2509.13626
- APXML, User Feedback in RAG (feedback-to-fix mapping): https://apxml.com/courses/optimizing-rag-for-production/chapter-6-advanced-rag-evaluation-monitoring/user-feedback-rag-improvement
- LangSmith evaluation concepts / annotation queues: https://docs.langchain.com/langsmith/evaluation-concepts
- Langfuse evaluation overview (online+offline, golden datasets): https://langfuse.com/docs/evaluation/overview
- RAGOps (arXiv 2506.03401) -- operating RAG pipelines: https://arxiv.org/html/2506.03401v1
