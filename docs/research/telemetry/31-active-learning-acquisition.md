# Topic 31: Active learning — selecting which documents/data to acquire next

## Findings
Production search/RAG teams treat **failed and low-confidence queries as the primary acquisition signal**. The dominant production pattern is the *null/low-results report*: queries that return zero or sparse hits are clustered and triaged into (a) content/catalog gaps (acquire new docs), (b) vocabulary/synonym gaps (fix tokenization), or (c) retrieval failures (fix ranking) — diagnosed via an "overlap test" (does relevant content exist but isn't shown?) (Tunkelang; Algolia; Lucidworks). Glean computes per-session user-satisfaction signals in BigQuery and reviews search logs to find "indexing gaps." RAG-specific work adds retrieval-grading (context precision/relevance) and statistical query-knowledge-relevance tests to flag *out-of-knowledge* queries before generation (arXiv 2410.08320; Corrective-RAG). LangSmith/Langfuse continuously capture production traces, score retrieval vs. generation separately, and promote flagged cases into growing eval datasets (one case: 150→2,400 queries, 16x). IR literature provides the selection math: uncertainty/entropy sampling, query-by-committee (disagreement), expected-gradient-length, and diversity-aware batch acquisition (BADGE k-means++) to avoid redundant picks.

## What to log
- Query text + zero/low-result flag + result count; **query frequency** (to prioritize by demand).
- Top retrieval scores / margin, and a query-knowledge relevance/grade score per query.
- Abstention ("I don't know") events and their cause (no relevant context vs. unnecessary refusal).
- Session outcome: click/no-click, dwell, reformulation, thumbs feedback.
- Disagreement across multiple retrievers/judges (committee signal); collection/source of misses.

## Metrics
- Zero/null-result rate (target <2-3%; >10% = broken discovery).
- Low-confidence / abstention rate; retrieval grade pass rate (context precision, faithfulness).
- Per-cluster miss volume (frequency-weighted gap size).
- Acquisition value: predicted model/answer change if doc added (EGL); committee disagreement.
- Eval-set growth and post-acquisition recovery (regression delta on curated set).

## How it is used (feedback loop)
1. Capture failed/low-confidence queries automatically into a dataset (LangSmith/Langfuse online evals).
2. Cluster + frequency-rank; diagnose gap type via overlap test.
3. Score candidate acquisitions by informativeness (uncertainty/committee) and diversity (batch) to pick *what to ingest next* — not everything.
4. Acquire content (new docs, web-search fallback, expert tacit knowledge) or fix synonyms/ranking.
5. Re-run the curated eval set to confirm recovery and catch regressions; repeat monthly.

## Sources
- https://dtunkelang.medium.com/making-sense-of-null-and-low-results-a077f37bf8fc
- https://lucidworks.com/blog/learn-from-zero-results-searches-with-never-null
- https://www.algolia.com/ecommerce-merchandising-playbook/null-results-optimization
- https://cloud.google.com/blog/products/data-analytics/glean-uses-bigquery-and-google-ai-to-enhance-enterprise-search
- https://www.glean.com/perspectives/overcoming-challenges-in-maintaining-current-search-indexes
- https://arxiv.org/pdf/2410.08320
- https://medium.com/@inkollusrivarsha0287/corrective-rag-fixing-retrieval-failures-in-rag-systems-85dd2b079fbb
- https://www.aiacceleratorinstitute.com/why-rag-fails-in-production-and-how-to-fix-it/
- https://docs.langchain.com/langsmith/evaluation
- https://langfuse.com/docs/evaluation/experiments/datasets
- https://lilianweng.github.io/posts/2022-02-20-active-learning/
- https://www.sciencedirect.com/science/article/abs/pii/S0020025518303700
