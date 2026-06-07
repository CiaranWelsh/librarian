# Topic 33: A/B Testing and Interleaving for Retrieval/Ranking Changes

## Findings
Two online methods dominate. **A/B tests** split traffic into arms (control vs. new ranker) and compare behavioral metrics; they are the gold standard but data-hungry — each arm sees only a fraction of traffic, and small ranking changes need orders of magnitude more traffic to reach significance (Pinterest, Amazon). **Interleaving** merges both rankers' results into one list shown to every user, then attributes clicks back to the source ranker. **Team Draft Interleaving (TDI)** alternates picks via a coin toss, tagging each result with its origin; balanced and probabilistic variants exist. Chapelle et al. (Yahoo) and Amazon Search show interleaving is far more sensitive — detecting the winner with ~10-100x less traffic — and its outcomes correlate linearly with A/B deltas. Workflow: offline screen (NDCG/MRR/Recall@k) then online A/B or interleave on behavioral signals.

## What to log
- Per-query impression with the **emitted ranking and source ranker (A/B) tag per result** (TDI requires per-result origin).
- **Views and clicks at each rank** (position-resolved), tied to query + session.
- Experiment/variant assignment id, prompt/index version, release tag.
- Guardrail signals: latency, error rate, cost, tokens.
- Offline labels / LLM-judge scores (faithfulness, context relevance) for pre-screening.

## Metrics
- Interleaving **per-impression preference**: wins/ties/losses, credit by ranker; CTR lift = `(CTR_B - CTR_A)/CTR_A`; expected per-query Δ_AB.
- A/B behavioral: CTR, save/engagement propensity, abandonment.
- Sensitivity / statistical power; significance via t-test, chi-squared, or sequential testing.
- Offline correlates: NDCG, MRR, Precision@k, Recall@k.

## How it is used
Offline metrics gate candidates before online exposure. Interleaving runs first as a cheap, high-power filter; survivors graduate to A/B tests that quantify real user-outcome impact. Significant positive deltas (with neutral guardrails) ship; failures are rolled back. Production traces feed online evaluators (Langfuse/LangSmith label prompt/index versions, score live traffic), and surprising cases become new offline test data — closing the loop.

## Sources
- Chapelle et al., Large-Scale Validation of Interleaved Search Evaluation (Cornell/Yahoo): https://www.cs.cornell.edu/people/tj/publications/chapelle_etal_12a.pdf
- Radlinski & Craswell, Optimized Interleaving (Microsoft, WSDM 2013): https://www.microsoft.com/en-us/research/wp-content/uploads/2013/02/Radlinski_Optimized_WSDM2013.pdf.pdf
- Debiased Balanced Interleaving at Amazon Search: https://assets.amazon.science/a9/c8/c9016a1c47caac6a634768e7491d/debiased-balanced-interleaving-at-amazon-search.pdf
- OpenSource Connections, A/B Testing with Team Draft Interleaving: https://opensourceconnections.com/blog/2025/08/06/a-b-testing-with-team-draft-interleaving/
- Sease, Online Testing for Learning To Rank: Interleaving: https://sease.io/2020/05/online-testing-for-learning-to-rank-interleaving.html
- Pinterest, Related Pins recommender (arXiv): https://arxiv.org/pdf/1702.07969
- Langfuse, A/B Testing (prompt versions): https://langfuse.com/docs/prompt-management/features/a-b-testing
- LangSmith Evaluation (offline/online): https://docs.langchain.com/langsmith/evaluation
- APXML, A/B Testing & Experimentation Frameworks for RAG: https://apxml.com/courses/large-scale-distributed-rag/chapter-5-orchestration-operationalization-large-scale-rag/ab-testing-experimentation-rag
