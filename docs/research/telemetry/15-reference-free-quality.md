# Topic 15: Reference-Free Retrieval / Answer Quality Estimation

## Findings
Reference-free estimation scores quality **without gold labels**, using only the query, retrieved
context, and generated answer. Two traditions converge in production:

1. **IR query performance prediction (QPP):** *pre-retrieval* predictors use query+corpus stats
   (AvgIDF, MaxIDF, SCQ, query-collection similarity); *post-retrieval* predictors analyse the
   result list (Clarity, WIG, NQC/UQC score-variance, retrieval coherency). Post-retrieval is
   consistently stronger; hybrids (e.g. GMM+NQC) win.
2. **RAG/LLM eval frameworks (RAGAS, TruLens, DeepEval) default to reference-free** LLM-as-judge.
   The "RAG triad" = context relevance + faithfulness/groundedness + answer relevance. Faithfulness
   decomposes the answer into claims and checks each against retrieved context. Answer relevance is
   estimated by back-generating questions the answer would satisfy and comparing to the real query.
3. **Uncertainty signals:** self-consistency (agreement across samples), semantic entropy (entropy
   over meaning-clusters), and cheap single-pass Semantic Entropy Probes off hidden states; used to
   trigger abstention or escalate to a larger model.

Key caveat: reference-free metrics measure *internal consistency, not external truth* — a stale
retrieved chunk can score 0.95 faithfulness and still be wrong. LLMs also over-trust their own
answers, so calibrate judges against human labels.

## What to log (per query trace)
- Query text + pre-retrieval stats (length, AvgIDF/MaxIDF, specificity/ambiguity).
- Retrieved set: doc IDs, similarity scores, score distribution/variance (for NQC/Clarity).
- Generated answer + per-claim grounding verdicts and cited chunk IDs.
- Reference-free judge scores: context-relevance, faithfulness, answer-relevance (0-1 + rationale).
- Uncertainty: self-consistency agreement, semantic entropy, verbalized confidence.
- Implicit feedback: which citations clicked/expanded, skips, dwell, follow-up/reformulation.
- Explicit feedback: thumbs up/down, abstentions; latency + cost per stage.

## Metrics
- RAG triad scores (faithfulness, context relevance, answer relevance) aggregated + trended.
- Post-retrieval QPP scores (NQC, Clarity) correlated to downstream answer quality.
- Hallucination AUROC from semantic entropy / NLI entailment (HHEM, Lynx, DeBERTa-MNLI).
- Calibration: ECE / Brier between confidence and observed correctness.
- Feedback rates: downvote %, abstention %, reformulation/skip rate.

## How it is used (feedback loop)
- **Online scoring on live traces** (Langfuse/TruLens): attach scores to every trace, dashboard +
  alert when faithfulness/relevance drop below threshold.
- **Routing/abstention:** low QPP or high entropy → abstain, ask to clarify, or defer to a bigger
  model (deferring ~15% can match full-model quality at lower cost).
- **Source pruning:** Perplexity de-indexes frequently skipped/downvoted sources within ~1 week;
  click-feedback retraining measurably improves retrieval.
- **Triage to golden sets:** route low-score / flagged traces to human review; use labels to
  calibrate the LLM judge and seed regression evals before shipping changes.

## Sources
- RAGAS reference-free claim decomposition / RAG triad: https://www.evidentlyai.com/llm-guide/rag-evaluation
- Braintrust RAG metrics (faithfulness, back-generated relevance): https://www.braintrust.dev/articles/rag-evaluation-metrics
- RAGAS/TruLens/DeepEval default reference-free: https://atlan.com/know/llm-evaluation-frameworks-compared/
- QPP survey (pre/post-retrieval predictors): https://www.emergentmind.com/topics/query-performance-prediction-qpp
- Pre-retrieval predictor survey (CIKM): https://dl.acm.org/doi/10.1145/1458082.1458311
- Langfuse online eval / scores on production traces: https://langfuse.com/docs/evaluation/overview
- Perplexity engagement signals + source de-indexing: https://ziptie.dev/blog/how-perplexity-ai-answers-work/
- Click-feedback retrieval (Princeton): https://arxiv.org/pdf/2305.00052
- Semantic entropy hallucination detection (Nature/Oxford): https://pmc.ncbi.nlm.nih.gov/articles/PMC11186750/
- Semantic Entropy Probes (single-pass): https://arxiv.org/abs/2406.15927
- Confidence Improves Self-Consistency (CISC): https://aclanthology.org/2025.findings-acl.1030.pdf
