# Topic 47: Citation/Attribution Logging and Measuring Citation Quality

## Findings
Production RAG/search systems treat citations as the "trust contract made visible." Two independent telemetry streams exist: (1) **offline/automatic attribution scoring** (NLI-based, from IR/ML literature — AIS/AutoAIS, ALCE), and (2) **online user-behaviour signals** (clicks, dwell, thumbs) borrowed from web-search click-modelling. Observability tools (Langfuse, LangSmith) log retrieval as typed spans carrying the source IDs needed to later score whether each cited chunk actually supports the claim. A recurring failure mode is citations that look right but don't entail the claim (one study: 57% of a RAG model's citations were "unfaithful"), so logging must capture both *what was cited* and *what was retrieved-but-uncited*.

## What to log
- Per answer: `query`, retrieved `doc_id`/`chunk_id` list, similarity scores, retrieval latency, which chunks were actually cited, span offsets of the quoted text, model + tokens/cost.
- Per citation: cited source_id, the claim/sentence it backs, NLI entailment label (support/partial/none), `cross_validated` (cited by >1 source), `retrieval_timestamp`.
- Online signals: citation click-through, position/slot of the source, dwell time after click, thumbs up/down, "answer used/copied".

## Metrics
- **Citation recall** — fraction of statements fully entailed by their cited passages (AIS/ALCE, NLI-based).
- **Citation precision** — fraction of cited passages that are actually needed (remove one → still entailed? then irrelevant).
- **Citation coverage** — share of important claims that carry any citation.
- **Faithfulness/groundedness** — per-claim support against context.
- **Hallucinated-citation rate** — citations pointing to non-existent/unverifiable sources.
- **ChunkUtilization / ChunkAttribution** — did the generator use the cited span's content.
- **Online: citation CTR, dwell, thumbs ratio.**

## How it is used
Automatic citation scores gate releases (block regressions) and localise faults: citation precision dropping while faithfulness is flat points to a prompt/parser bug (wrong chunk_id mapping), not the model. Per-claim metadata (unsupported claims) drives debugging. Online click/dwell signals — de-biased via click models for position bias — become implicit relevance labels that retrain the retriever/reranker, and thumbs feedback seeds eval datasets. Better retrieval raises citation quality (retrieval recall is an upper bound), closing the loop.

## Sources
- ALCE benchmark (citation recall/precision, NLI): https://arxiv.org/abs/2305.14627 ; https://github.com/princeton-nlp/ALCE
- Rashkin et al., AIS / AutoAIS: https://arxiv.org/html/2508.15396v1
- CiteEval: https://arxiv.org/pdf/2506.01829
- Why Citation-Based RAG Still Hallucinates (57% unfaithful; correctness vs faithfulness): https://yaihq.com/research/citation-based-rag-still-hallucinates
- RAG benchmark hallucination-rate definition: https://arxiv.org/html/2601.14949v2
- Span-level citation-validity, ChunkUtilization/Attribution: https://futureagi.com/blog/evaluating-rag-chunking-strategies-2026/
- RAG metric guide (citation precision/coverage, fault diagnosis): https://www.digitalapplied.com/blog/rag-system-metrics-recall-precision-faithfulness-2026
- Langfuse RAG spans + RAGAS eval: https://langfuse.com/guides/cookbook/evaluation_of_rag_with_ragas
- LangSmith/Langfuse observability (retrieval spans, source IDs): https://www.langflow.org/blog/llm-observability-explained-feat-langfuse-langsmith-and-langwatch
- Enterprise citation/provenance schema (similarity_score, embedding_id, cross_validated): https://www.the-main-thread.com/p/enterprise-rag-java-citations-provenance-quarkus
- Click models / implicit relevance feedback (position bias de-biasing): https://arxiv.org/pdf/2006.07581
- Perplexity citation CTR (~41%) and selectivity: https://www.averi.ai/blog/ai-citation-tracking-chatgpt-perplexity-claude
