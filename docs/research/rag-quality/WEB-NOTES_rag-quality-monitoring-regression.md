# Web research notes: RAG quality monitoring + regression detection (June 2026)

Source notes captured during a web literature search. Companion to the PDFs in this
directory (RAGAS, G-Eval, LLM-as-judge, QPP papers). Quotes are from the cited URLs.

## Tool-by-tool: dashboard signals + over-time tracking

### Arize Phoenix (open source, OTel/OpenInference)
- Three RAG evaluators: **DocumentRelevanceEvaluator**, **CorrectnessEvaluator**,
  **FaithfulnessEvaluator** (hallucination). Tutorial example: correctness ~0.91,
  hallucination 0.05.
- Retrieval metrics: **NDCG@k**, **Precision@k**, **Hit Rate**, aggregated via `.mean()`.
- Evals attach to spans via `log_span_annotations_dataframe` / `log_document_annotations_dataframe`.
- Distinctive: **embedding visualization** (UMAP/t-SNE projection of query + doc embeddings)
  to make retrieval drift visually detectable.
- Online eval cadence cited: "every few minutes for online evaluation," logged to Arize/Phoenix.
- Source: https://arize.com/docs/phoenix/cookbook/evaluation/evaluate-rag

### TruLens (RAG Triad)
- **RAG Triad** = Context Relevance (retrieval) + Groundedness (generation supported by context)
  + Answer Relevance (sanity check on output). All via LLM-as-judge, default output scale 0-3.
- Purpose: localize failures (e.g., good context relevance + poor groundedness => fix generation prompt).
- Production: feedback functions + OpenTelemetry tracing.
- Source: https://www.trulens.org/getting_started/core_concepts/rag_triad/

### LangSmith
- Offline (datasets/reference outputs): benchmarking, **regression testing**, unit testing, backtesting.
- Online (production traces, no reference outputs): safety, anomaly/quality-degradation detection.
- 30+ prebuilt evaluator templates; online evaluators can score every trace or a sample.
- CI: pytest/Vitest/GitHub Actions; fail PR when avg score < threshold (example ~0.85);
  trace-to-dataset "single click" to turn prod failures into regression cases.
- Dataset start size: docs say "10-20 high-quality examples"; articles say "a few dozen."
- Cost note: gpt-4o-mini judge ~$0.02 per 50-example CI run.
- Sources: https://docs.langchain.com/langsmith/evaluation-concepts ,
  https://www.langchain.com/articles/llm-evals

### Langfuse (open source)
- LLM-as-judge / heuristic / human-review evaluators on Observations, Traces, or Experiments.
- Online eval is async at ingest: filter -> queue -> score attached to observation.
- **Sampling** is the main cost lever; typical eval $0.01-0.10 per assessment.
- Custom + prebuilt dashboards (latency/cost/usage); filter by score thresholds; quality tracked
  over time across prompt/model versions.
- Gap: NO native automated drift detection/baselining; you build it on exported data.
- Sources: https://langfuse.com/docs/evaluation/overview ,
  https://langfuse.com/blog/2025-10-28-rag-observability-and-evals ,
  https://galileo.ai/blog/best-llm-output-drift-monitoring-platforms

### Galileo
- RAG metrics: **Context Adherence** (groundedness, ~precision), **Completeness** (recall),
  **Chunk Attribution** (binary: did chunk affect response), **Chunk Utilization** (0-1: how much
  of chunk used). Accuracy claims: attribution 86%, utilization 74%, adherence 74%, completeness 80%.
- "Luna" = free in-house small models (one inference for all metrics); "Plus" = external LLM (costlier, more accurate).
- **Luna-2 guardrails**: sub-200ms latency, ~$0.02/M tokens -> "practical to evaluate 100% of traffic
  rather than sampling." Runtime guardrails block before user sees output.
- "Signals" surface failure patterns linked to trace spans; threshold-based monitors + alerting.
- Sources: https://docs.galileo.ai/.../chunk-attribution , https://galileo.ai/blog/best-rag-observability-tools

### Ragas-in-production
- Component metrics: faithfulness, answer relevance, context precision, context recall (+ Precision@k,
  Recall@k, MRR, NDCG on retrieval).
- Reference-free judges; integrates with LangSmith/Langfuse for tracing low-scoring requests.
- CI pattern: golden set 50-100 Qs version-controlled, run on every merge, fail if faithfulness < 0.85 floor.
- Prod complement: sample ~1 in 1000 live queries, build faithfulness time-series, alert on
  3-day rolling drop of 5+ points.
- Caveat: track context precision/recall on the *retrieve span* independently — generation metrics can
  look healthy while context recall silently drops 30%.
- Suggested floors (calibrate!): faithfulness 0.75, answer relevancy 0.8, context precision 0.7, context recall 0.8.
- Source: https://www.invra.co/... , https://blog.premai.io/rag-evaluation-metrics-frameworks-testing-2026/

### Vectara HHEM / Factual Consistency Score
- **HHEM** (Hughes Hallucination Evaluation Model) outputs calibrated **Factual Consistency Score (FCS)** 0-1.
- Specialized small model: ~0.6s on RTX 3090 for 4096-token context vs RAGAS+GPT-4 up to 35s.
- HHEM-2.1-Open open source (#1 on HF, 100k+ downloads); commercial HHEM-2.3 better on long contexts
  (open drops to 62.58% balanced accuracy on long premises).
- Hallucination detection is asymmetric/non-commutative.
- Pairs with VHC (Vectara Hallucination Corrector) for remediation.
- Source: https://www.vectara.com/blog/hhem-v2-a-new-and-improved-factual-consistency-scoring-model

## Drift detection specifics (apxml course + practitioner blogs)
- Embedding distribution monitoring vs a reference window:
  - **KS test** per embedding dimension; Mahalanobis distance / drift detectors on PCA-reduced embeddings.
  - **PSI** bands: <0.1 no shift, 0.1-0.25 minor, >0.25 major (apxml). (Credit-risk heuristic; calibrate.)
  - Query drift: Jensen-Shannon divergence, Wasserstein distance; OOD via autoencoder/one-class SVM.
  - Static "probe" docs: re-embed, compare cosine/L2 to stored vector to catch embedding-model degradation.
- Cosine-similarity baseline: rolling avg + std of query<->retrieved-doc similarity; alert on sustained
  **>2 standard deviation** drop.
- Most common real cause: re-embed 20% of corpus, 80% stays on old generation -> silent gradual decay.
- Monitoring frequency: batch (hourly/daily); golden-set re-eval with nDCG/MRR/Recall@K.
- Sources: https://apxml.com/courses/optimizing-rag-for-production/chapter-6-advanced-rag-evaluation-monitoring/monitoring-retrieval-drift-rag ,
  https://decompressed.io/learn/embedding-drift

## Golden / probe set sizing (multiple sources)
- Start: 30-50 (LangSmith docs: 10-20) curated queries covering core + edge cases.
- Common: 50-100 queries for a CI regression suite.
- Pre-prod reliability: 200+ questions; "industrial/SOTA": 1000+. Quality > quantity.
- Three-layer pattern (practitioner): offline golden 100-500 tuples gates every change;
  online eval samples 5-20% of prod traffic for faithfulness/groundedness/answer-relevance;
  periodic probe set 50-100 query-context pairs daily/weekly.
- Human calibration: 10 random responses/week -> estimate quality within +/-10pp at 95% confidence.
- Include: factual, multi-doc synthesis, ambiguous, and "no answer/insufficient info" cases.
- Version the dataset like code; refresh from sampled production failures (trace-to-dataset).
- Sources: https://medium.com/data-science-at-microsoft/the-path-to-a-golden-dataset... ,
  https://www.statsig.com/perspectives/golden-datasets-evaluation-standards

## Cost/latency tradeoffs of scoring
- Online every-query LLM-judge: high cost, multi-second latency -> usually impractical at scale,
  EXCEPT with small specialized scorers (Galileo Luna-2 <200ms; Vectara HHEM ~0.6s).
- Sampled (5-20%): standard middle ground (Langfuse, Arize, Ragas-in-prod).
- Offline probe/golden run: cheapest per-query coverage; daily/weekly batch.
- Levers: sampling %, target observation vs full trace, cheaper judge model, encoder-based scorers.
