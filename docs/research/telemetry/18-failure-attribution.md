# Topic 18: Attributing failures to retrieval vs generation in RAG pipelines

## Findings
A wrong RAG answer is multi-causal but looks identical from outside, so a single
end-to-end "is the answer good" score collapses distinct failure modes. The
practitioner consensus is to **decompose** the pipeline into independently scored
retrieval and generation stages and attribute via the *gap between metrics*:
right chunk never reached the model (retrieval) vs reached but ignored/contradicted
(generation). The TruLens **RAG Triad** (context relevance → groundedness →
answer relevance) maps one metric to each pipeline edge; RAGAS, DeepEval, RAGChecker,
and RAG-X formalize the same split. Observability tools (Langfuse, LangSmith, Arize)
make this operational via span-level tracing so a low score drills into the exact
trace: which chunks were retrieved, what prompt was sent, where it broke. Watch the
"Accuracy Fallacy": high adherence can mask answers from parametric memory not
retrieval (RAG-X found a 14% accuracy vs context-hit-rate gap).

## What to log
- Per-request **trace** with typed spans: retrieval, rerank, generation.
- Retriever span attributes: `chunk_id`, `chunk_text`, `similarity_score`,
  `doc_version`, `source_url`, `retriever_strategy`, top-K.
- Reranker span: before/after ordering.
- Generation span: prompt sent, model, params (temperature), tokens, output.
- Embedding-model + index versions (drift / freshness detection).
- Attached scores from LLM-as-judge, heuristics, and user feedback.

## Metrics
- **Retrieval:** context relevance, contextual/claim recall@k, contextual precision
  (rank order), context hit rate.
- **Generation:** faithfulness/groundedness (claims supported by context), answer
  relevance, hallucination / noise-sensitivity, context utilization.
- **Attribution rule:** poor retriever + good generator = retrieval failure;
  good retriever + poor generator = generation failure; recall ok but faithfulness
  drop = ignored chunk; freshness regression = stale index.

## How it is used
Each metric points at a different fix: recall → embeddings/chunking; precision →
reranker; relevance → top-K/chunk size; faithfulness → prompt/temperature/model;
freshness → ingestion. Closed loop: trace every request → score (heuristics 100%,
LLM-judge 10-20% sample, periodic human annotation) → filter by low score to find
systematic patterns → curate failing traces into a dataset → offline regression-test
before shipping retrieval/prompt changes → repeat.

## Sources
- TruLens RAG Triad: https://www.trulens.org/getting_started/core_concepts/rag_triad/
- Snowflake, LLM-as-judge for RAG Triad: https://www.snowflake.com/en/engineering-blog/benchmarking-LLM-as-a-judge-RAG-triad-metrics/
- Confident AI (DeepEval) RAG metrics: https://www.confident-ai.com/blog/rag-evaluation-metrics-answer-relevancy-faithfulness-and-more
- RAGChecker (arXiv 2408.08067): https://arxiv.org/pdf/2408.08067
- RAG-X systematic diagnosis (arXiv 2603.03541): https://arxiv.org/html/2603.03541v1
- RAG observability span schema (Future AGI): https://futureagi.com/blog/what-is-rag-observability-2026
- Langfuse data model: https://langfuse.com/docs/observability/data-model
- Ten failure modes of RAG: https://dev.to/kuldeep_paul/ten-failure-modes-of-rag-nobody-talks-about-and-how-to-detect-them-systematically-7i4
