# Topic 13: Groundedness / Faithfulness / Hallucination Detection Signals for RAG

## Findings
Production RAG defines a hallucination narrowly as a **claim not supported by retrieved context**, so the dominant signal is answer-vs-context consistency. RAGAS, TruLens, DeepEval, Arize Phoenix, LangSmith and Langfuse all converge on the same core construct (groundedness == faithfulness). The standard computation is **claim decomposition**: split the answer into atomic claims, then check each against retrieved chunks via an LLM-judge or an NLI/entailment model (DeBERTa-MNLI, SummaC entailment matrix, Vectara HHEM, Patronus Lynx-8B). Cheaper white-box signals (token logprobs, entropy, semantic entropy) and black-box self-consistency (SelfCheckGPT: N sampled responses compared by NLI/BERTScore) flag uncertainty when logprobs or reference contexts are unavailable. Citation/attribution is a distinct failure class needing external lookup verification.

## What to log
- Query, retrieved chunk IDs + scores + rank, prompt sent, generated answer.
- Per-answer atomic claims and per-claim supported/unsupported verdict + supporting chunk.
- Citation/attribution spans: which source each statement maps to; "unsupported" flag.
- Token logprobs / sequence-logprob / entropy when available; self-consistency sample agreement.
- Which retrieved-document rank positions were actually used to ground statements.
- Judge model/version, evaluator latency and cost.

## Metrics
- **Faithfulness/groundedness** = supported claims / total claims (0-1); pass/fail ~0.85.
- Context precision, context recall, answer relevancy (paired so grounded-but-off-topic is caught).
- Hallucination rate; SelfCheckGPT sentence-level AUC-PR (~0.93 reported on GPT-3 bios).
- Citation hallucination rate (47-77% for fabricated CS reference titles in studies).
- Document-rank utilisation distribution (e.g. ranks 7-10 = 5% of citations).

## How it is used
- **Regression gating**: track faithfulness over time; alert if weekly average drops >5%, then drill into the specific low-score trace to localise retrieval vs generation failure.
- **Root-cause on traces**: low groundedness -> strengthen grounding prompt or retrieval; low recall -> fix chunking/index.
- **Retrieval tuning**: rank-utilisation telemetry cut top-k from 10 to 6 (~40% cost saving).
- **Layered gating**: cheap logprob/entropy filter escalates to self-consistency sampling and citation-forcing (block/flag answers lacking a source).

## Sources
- RAGAS faithfulness: https://docs.ragas.io/en/stable/concepts/metrics/available_metrics/faithfulness/
- Langfuse RAG observability & evals: https://langfuse.com/blog/2025-10-28-rag-observability-and-evals
- LangSmith evaluate-RAG tutorial: https://docs.langchain.com/langsmith/evaluate-rag-tutorial
- deepset groundedness (Reference Predictor, rank utilisation): https://www.deepset.ai/blog/rag-llm-evaluation-groundedness
- Comet RAG evaluation guide: https://www.comet.com/site/blog/rag-evaluation/
- SelfCheckGPT (zero-resource black-box detection): https://aclanthology.org/2023.emnlp-main.557.pdf
- Token-probability hallucination detection: https://arxiv.org/html/2405.19648v1
- Reference/citation hallucination study: https://arxiv.org/pdf/2604.03173
- Scadea RAG hallucination metrics & thresholds: https://scadea.com/evaluating-rag-quality-hallucination-detection-and-answer-accuracy-metrics/
