# Topic 34: Online / Continuous Evaluation Pipelines for RAG in Production

## Findings
Production teams run evaluation as a *continuous loop*, not a one-off: sample live
traffic, score it (cheap automated judges + targeted human review), gate releases,
and feed failures back into golden datasets and judge prompts. The LLM-observability
stack (Langfuse, LangSmith, Arize, TruLens, Opik) instruments every RAG step as a
trace/span, then attaches scores to those traces. Scoring runs either per-trace
(expensive, full coverage) or on a periodic random/triggered sample (cheap, may miss
cases). Reference-free judges (RAGAS, feedback functions) are favored because
production lacks ground-truth answers. Classic IR practice complements this: search
teams (Airbnb, Amazon, web search) use **interleaving** (team-draft) for 1-2 orders
of magnitude more sensitivity than A/B tests, and **counterfactual / off-policy (IPS)**
estimators to replay logged clicks against new rankers without deploying them.

## What to log
- Full trace/span per query: retrieved chunk IDs + per-chunk relevance scores, assembled
  context, final answer, latency, token cost.
- Session ID to reconstruct multi-turn conversations.
- Explicit user feedback (thumbs, ratings) and implicit signals: clicks/click position,
  dwell, fallback/refusal hits, frustration, query reformulation, competitor mentions.
- Logging policy / propensity (which ranker placed each item) for unbiased off-policy replay.
- Human annotation-queue verdicts on sampled/low-scoring traces.

## Metrics
- Generation: faithfulness/groundedness, answer relevancy, completeness-to-question
  and -to-context, tone, refusal rate.
- Retrieval: hit rate, share of relevant chunks, context precision/recall, avg relevance.
- Session: binary "solved/not-solved", helpfulness, sentiment.
- Online comparison: interleaving preference Delta_AB / preference margin (t-test),
  CTR, P50/P99 latency, cost.

## How it is used
Daily/streamed scores power dashboards and threshold alerts (webhook/PagerDuty).
Low-scoring or signal-triggered traces route to human review; verified verdicts expand
golden datasets and recalibrate (and de-bias) LLM judges. Production failures become
regression test cases that gate deploys. Candidate changes flow offline-eval ->
interleaving -> A/B before rollout; counterfactual estimators pre-screen many candidates
from logged data. Validate the judge against human labels before trusting it (verbosity/
self-preference/position bias).

## Sources
- Langfuse, RAG Observability and Evals: https://langfuse.com/blog/2025-10-28-rag-observability-and-evals
- Langfuse + RAGAS cookbook: https://langfuse.com/guides/cookbook/evaluation_of_rag_with_ragas
- LangSmith observability/online evals: https://www.langchain.com/langsmith/observability
- EvidentlyAI, RAG evaluation guide: https://www.evidentlyai.com/llm-guide/rag-evaluation
- Confident AI, RAG evaluation metrics: https://www.confident-ai.com/blog/rag-evaluation-metrics-answer-relevancy-faithfulness-and-more
- Airbnb, Beyond A/B Test (interleaving): https://medium.com/airbnb-engineering/beyond-a-b-test-speeding-up-airbnb-search-ranking-experimentation-through-interleaving-7087afa09c8e
- Airbnb, Interleaving + Counterfactual eval: https://arxiv.org/html/2508.00751v1
- Chapelle et al., Large-Scale Validation of Interleaved Search Evaluation: https://www.cs.cornell.edu/people/tj/publications/chapelle_etal_12a.pdf
- Oosterhuis & de Rijke, Taking the Counterfactual Online: https://arxiv.org/pdf/2007.12719
- Case-Aware LLM-as-a-Judge for Enterprise RAG: https://arxiv.org/pdf/2602.20379
