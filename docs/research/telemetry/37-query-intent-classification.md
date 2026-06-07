# 37 — Query Intent Classification and Routing from Logs

## Findings
Search practice rests on Broder's tripartite intent taxonomy
(informational / navigational / transactional). Jansen et al. automated this
over a 1.5M-query log, finding ~80% informational, ~10% each navigational and
transactional, validated against 400 hand-coded queries at 74% accuracy. Click
signals carry intent: navigational queries show 1-2 clicks to a URL lexically
similar to the query; transactional queries a single decisive click. Bing's
ORCAS-I refined this into a hierarchy (informational → factual / instrumental)
using clicked-URL features. In RAG/LLM stacks the same idea becomes a *router*:
an orchestration layer classifies intent first and decides whether to retrieve,
which corpus/route, and which model. Semantic routers (embedding-vs-reference
similarity) are the cheap default for 3-10 separable routes; an LLM classifier
is the fallback for ambiguous cases. Routing is logged as a first-class trace
span so intent can later slice every downstream metric.

## What to log
- Predicted intent label + confidence/similarity score, attached as trace-level
  metadata/tags **early** so it propagates to all child spans (Langfuse).
- Chosen route / `retriever_strategy` and the cascade trace (heuristic → embedding → LLM → default route, with the deciding layer).
- Margin between top-2 route similarities, and whether a fallback fired.
- Clicked / opened result IDs and rank (implicit intent label).
- Query text, session_id, user_id, latency, cost per routed span.

## Metrics
- Routing precision / recall / F1 on a labelled held-out set (100-200 queries).
- Fallback rate (>50% after tuning signals bad routes/utterances, not strategy).
- Per-intent slices of latency, cost, and answer quality.
- Intent-class distribution over time (for drift / emerging-intent detection).
- Inter-rater agreement (Cohen's κ) when hand-labelling a golden set.

## How it is used
Treat the router as its own evaluable component, not bundled into end-to-end
evals. Version the reference utterances and confidence/margin thresholds with
code; tune thresholds from logged score distributions. Mine aggregate intent
labels to detect topic shifts and emerging intents, then add routes, reference
utterances, or corpus content to cover them. Audit misrouted queries (silent
wrong-shard retrieval) via the logged cascade trace. Use clicked-URL / weak
labels from logs to retrain the classifier (click-graph propagation, RouteLLM-
style preference training).

## Sources
- Jansen, Booth, Spink — Determining informational/navigational/transactional intent: https://faculty.ist.psu.edu/jjansen/academic/pubs/jansen_user_intent.pdf
- ORCAS-I dataset & classifier (Bing log subsample): https://arxiv.org/pdf/2504.21398
- Task-based information request intent taxonomy (Cohen's κ=0.84): https://arxiv.org/html/2601.12985v1
- Building a production-grade semantic router (thresholds, cascade, eval): https://atul4u.medium.com/building-a-production-grade-semantic-router-the-smart-way-to-route-ai-prompts-f303e6d2ae7e
- RouteLLM / LLM routing techniques: https://www.getmaxim.ai/articles/top-5-llm-routing-techniques/
- Langfuse metadata & attribute propagation: https://langfuse.com/docs/observability/features/metadata
- Building production-ready agentic RAG (intent classification in orchestration): https://labs.adaline.ai/p/building-production-ready-agentic
- What is RAG observability (retriever_strategy span attributes): https://futureagi.com/blog/what-is-rag-observability-2026
