# Topic 49: Evaluating long-form / multi-source answer quality from logs

## Findings
Production practice splits the pipeline (retriever vs. generator) and scores long-form,
multi-source answers **claim-by-claim** rather than as a blob. The dominant academic pattern
is **decompose-then-verify** (FActScore, SAFE, VeriScore): break the answer into atomic claims,
retrieve evidence per claim, label each as supported/unsupported. For attributed answers the
**ALCE** precision/recall framing dominates: a citation is "precise" if removing it breaks
entailment of the sentence; "recall" checks every claim is cited. Most of this runs as
**reference-free LLM-as-judge** so it can be applied to live traces with no gold labels. Tools
(RAGAS, TruLens "RAG triad", Langfuse/LangSmith) attach these scores to spans automatically.

## What to log
- Full trace: query, retrieved chunk IDs + ranks + scores, final answer text, **per-claim citation spans** (which source backs which sentence).
- Per-span generations: model/params, tokens, cost, latency.
- Explicit feedback: thumbs up/down, edits, regenerate, copy.
- Implicit signals: citation clicks, dwell time, follow-up questions, abandonment.
- Version tags (model, prompt, retriever config) for regression attribution.

## Metrics
- **Faithfulness / groundedness** = supported claims ÷ total claims.
- **Citation precision & recall (ALCE F1)**; citation coverage (claims cited at all).
- **Context precision / recall** — catches missing evidence that high faithfulness hides.
- **Answer relevance / completeness**; noise sensitivity.
- LLM-judge alignment vs. humans (Cohen's κ, ~95% in production studies; degrades on hard/ambiguous cases).
- Operational: latency, cost-per-correct-answer, freshness.

## How it is used
Production failures become test cases; sampled live queries refresh the gold set
(offline 0.92 faithful vs. live 0.78 gap is the recurring warning). Reference-free judges run
continuously to gate deployments on **composite** thresholds (faithfulness × context_precision),
avoiding single-metric gaming. Annotation queues route low-scoring traces to experts. Engagement
signals (clicks, follow-up satisfaction) re-rank retrieval (Perplexity). Pairwise/arena LLM-judge
runs do comparison-based regression testing between versions.

## Sources
- ALCE / citation precision-recall: https://arxiv.org/pdf/2510.06823
- RAGAS metrics: https://docs.ragas.io/en/v0.1.21/concepts/metrics/ ; production loop: https://www.invra.co/en/rag-evaluation-with-ragas-measuring-faithfulness-context-precision-and-recall-in-production/
- Reference-free production monitoring + offline/online gap: https://www.evidentlyai.com/llm-guide/rag-evaluation ; https://cobbai.com/blog/evaluate-rag-answers
- FActScore / SAFE / VeriScore decompose-then-verify: https://arxiv.org/abs/2406.19276 ; https://arxiv.org/html/2510.12839
- Langfuse/LangSmith trace logging, online eval, annotation queues, Ragas-on-traces: https://docs.ragas.io/en/v0.1.21/howtos/integrations/langfuse.html ; https://docs.langchain.com/langsmith/observability
- LLM-judge vs human alignment, arena pairwise regression testing: https://www.confident-ai.com/blog/llm-arena-as-a-judge-llm-evals-for-comparison-based-testing ; https://arxiv.org/pdf/2411.15594
- Perplexity engagement/citation telemetry into ranking: https://ziptie.dev/blog/how-perplexity-ai-answers-work/ ; https://authoritytech.io/blog/how-perplexity-selects-sources-algorithm-2026
