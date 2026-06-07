# 17 - Answerability Prediction: is a query answerable from the corpus?

## Findings
Two practitioner traditions converge. (1) **IR query-performance prediction (QPP)**: estimate retrieval quality *without* relevance labels, either *pre-retrieval* (query+collection stats: AvgIDF, SCQ, query length, ambiguity) or *post-retrieval* (score distribution of the top-k: NQC, WIG, Clarity, score variance/separability). Post-retrieval predictors generally beat pre-retrieval; hybrids win. (2) **RAG answerability/abstention**: a dedicated classifier or evaluator decides *before generation* whether the retrieved context supports an answer, to prevent "hallucination laundering." Examples: CRAG's lightweight T5 retrieval evaluator emits a confidence score bucketed via upper/lower thresholds into `{Correct, Ambiguous, Incorrect}`; IBM Granite ships a LoRA "answerability-prediction" intrinsic labeling the final query answerable/unanswerable against provided docs (reported lifting unanswerable F1 from 14 to 47). Microsoft's "confidence-aware RAG" layers retrieval-confidence scoring + citation validation + an LLM abstention judge.

## What to log
- Per query: top-k retrieval scores (max, mean, variance/std, gap top1-top2), result count above threshold.
- Pre-retrieval features: query length, AvgIDF/max-IDF, term rarity, ambiguity flag.
- Predicted answerability label/score + which threshold band (correct/ambiguous/incorrect).
- Action taken: answered / abstained ("I don't know") / fell back / escalated.
- Outcome: user reformulation, follow-up, thumbs, downstream correctness label.
- Citations emitted vs. context (citation-validation pass/fail).

## Metrics
- Answerable vs. unanswerable **precision/recall/F1**; unanswerable recall is the key safety metric.
- **Abstention/no-answer rate** and false-abstention rate (refused when answerable).
- QPP correlation to realized quality (Pearson/Kendall vs. nDCG/MAP).
- Context precision/recall, groundedness/faithfulness (RAGAS-style).
- Threshold sweep curves (precision vs. coverage) to pick operating point.

## How it is used
Feedback loop: log signals -> build a labeled eval set of answerable/unanswerable queries -> sweep upper/lower thresholds to hit target precision/coverage -> route: answer, abstain, expand retrieval, web-fallback, or escalate to human (CRAG/Microsoft pattern). Abstention clusters expose corpus coverage gaps that drive ingestion priorities. Observability tools (Langfuse/LangSmith) trace per-step scores and run regression evals on each pipeline change. Avoid latency-heavy sampling-based confidence in production.

## Sources
- IBM Granite answerability LoRA: https://huggingface.co/ibm-granite/granite-3.2-8b-lora-rag-answerability-prediction
- LLM Intrinsics for RAG (F1 14->47): https://arxiv.org/pdf/2504.11704
- CRAG (Corrective RAG evaluator, thresholds): https://arxiv.org/html/2401.15884v2
- Microsoft confidence-aware RAG (layered abstention): https://techcommunity.microsoft.com/blog/azuredevcommunityblog/confidence-aware-rag-teaching-your-ai-pipeline-to-acknowledge-uncertainty/4515061
- Query Performance Prediction overview: https://www.emergentmind.com/topics/query-performance-prediction-qpp
- Combining QPP predictors (reproducibility): https://arxiv.org/pdf/2503.24251
- Unanswerable/multi-hop RAG benchmark: https://arxiv.org/html/2510.11956
- Langfuse RAG observability & evals: https://langfuse.com/blog/2025-10-28-rag-observability-and-evals
- LangSmith RAG evaluation: https://docs.langchain.com/langsmith/evaluate-rag-tutorial
