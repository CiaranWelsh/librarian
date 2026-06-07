# RAG Quality Scoring — Findings & Recommendation

*Date: 2026-06-04. Synthesis of a four-agent web literature sweep (frameworks, LLM-as-judge,
Query Performance Prediction, production monitoring). Goal: a RAG quality score that (a) tells
us per-query whether we got decent hits, and (b) aggregates over time to flag regressions
(e.g. "added data → quality dropped"). Papers in this folder; sources cited inline.*

## TL;DR — and the one hard truth

There is **no single cheap number that precisely grades RAG quality**. The field splits the
problem by cost/accuracy, and so should we:

- **Cheap, always-on, no-LLM (per-query):** Query Performance Prediction (QPP) from the top-k
  score distribution. **But for *dense* retrieval its correlation with true quality is only
  r ≈ 0.2–0.4** (Faggioli et al., arXiv:2302.09947). So it's a **triage / abstain / confidence**
  signal — "did this whiff?" — **not a precise grade.**
- **Accurate, costs an LLM call (sampled or on-demand):** the **RAG Triad** — Context Relevance
  + Groundedness + Answer Relevance — all reference-free (TruLens/RAGAS). GPT-4-class judge ≈
  80% human agreement. **Sample 10–20% of queries** for monitoring; per-query only as a gate.
- **Regression detection ("added data broke retrieval"):** a **golden probe set run in CI / on
  a schedule** — *which we already built this session* — is the primary defense, plus cheap
  embedding-drift checks (static probe-doc re-embedding; cosine-baseline > 2σ drop).

**We are well-positioned:** the eval harness + golden sets from issue 027 *are* the monitoring
tier; we mainly need to add the cheap per-query QPP signal and a sampled LLM-judge.

## What the field does (the four angles)

### 1. Reference-free metric frameworks
The **RAG Triad** (TruLens; adopted by RAGAS, DeepEval, Phoenix, Galileo) is the de-facto
standard, all computable from `{query, context, answer}` with no gold label:
- **Context Relevance** — are the retrieved chunks pertinent to the query? (the pure retrieval signal)
- **Groundedness / Faithfulness** — is the answer supported by the context? (hallucination guard)
- **Answer Relevance** — does the answer address the question?

Most are LLM-judged, but **encoder-based variants avoid the LLM**: Vectara **HHEM** (DeBERTa
cross-encoder → 0–1 factual-consistency, ~0.6 s) and Galileo **Luna** (~200 ms, scores 100% of
traffic). RAGAS formulas: faithfulness = supported-claims/total-claims; context relevance =
needed-sentences/total. (arXiv:2309.15217.)

### 2. LLM-as-judge methodology
- Best-practice rubric = **per-chunk, pointwise, reason-before-score, coarse 0–2 scale** — this
  is essentially our `judge_eval.py`, so our offline arbiter was well-formed.
- Reliability: GPT-4 judge ≈ 80% human agreement (MT-Bench); RAGAS WikiEval 0.95 faithfulness /
  0.70 context-relevance. **Context relevance is the *least* reliable edge** — pair it with
  groundedness. Biases (position/verbosity/self-preference) → mitigate with CoT + pointwise.
- **Cost: sample 10–20%** for monitoring (consensus); per-query only as a guardrail/gate or with
  a cheap fine-tuned local judge (Prometheus-13B ≈ GPT-4 agreement). Prefix-cache the rubric; async.

### 3. Query Performance Prediction (QPP) — the cheap per-query signal
The 20-year IR field for "how good is this retrieval, without judgments." Post-retrieval,
score-based predictors usable directly on dense top-k cosines:
- **NQC** (normalized std-dev of top-k scores) — most reliable & **most dense-transferable**.
- **Top-1 absolute cosine** — doubles as **out-of-corpus / abstain** signal (kNN-distance OOD,
  Sun et al. arXiv:2204.06507): a low top-1 means the query isn't in our corpus.
- **Top1–top2 margin** / softmax entropy over top-k — distinguishability of the best hit.
- Combine via a small **logistic regression (Platt)** on a few hundred labels → calibrated
  `P(good) ∈ [0,1]`. Isotonic if > 10k labels.
- **Hard caveat:** dense retrieval *compresses* score range, so QPP r drops to ≈ 0.13–0.28 on
  neural runs (NQC ~10% worse than on lexical). **Triage, not oracle.** Variance-based (NQC/SMV)
  survive the move to dense; LM-based (Clarity) don't.

### 4. Production monitoring & regression detection — the standard 4-layer architecture
1. **Offline gate (golden set in CI):** score retrieval & generation *separately*; fail below a
   floor. **This is what catches "added data broke retrieval."** (We have this.)
2. **Online per-query signal:** fast encoder scorer on 100% (HHEM/Luna) *or* sampled LLM-judge.
3. **Drift detection:** time-series of scores + embedding-distribution drift vs a baseline
   (cosine-sim > 2σ drop; **static probe-doc re-embedding** catches the partial-re-embed bug);
   alert only when drift *correlates* with a score drop.
4. **Flywheel:** low-scoring queries fold back into the golden set.
No universal single "health score" exists; teams roll a small composite and **score retrieval
and generation separately** so a drop is attributable.

## Recommendation for librarian — a 3-tier RAG quality capability

Mapped onto our daemon + the eval harness we already built, honestly scoped to a dense-only store:

- **Tier 0 — per-query retrieval confidence (always-on, no LLM, cheap).** From the top-k cosine
  scores the daemon already has: **NQC + top-1 + top1–top2 margin**, *plus our chunk-substance
  signal (fragment-rate)* — combined into a 0–1 `confidence`, returned with every search.
  Distinct from cosine (it folds in distribution shape *and* chunk quality). **Framed as triage**
  ("strong / weak / likely-no-answer"), not a precise grade, per the dense-QPP r≈0.3 reality.
  Also gives a free **abstain / out-of-corpus** flag from a low top-1.
- **Tier 1 — RAG-Triad LLM-judge (accurate, sampled / on-demand).** Context-relevance +
  groundedness via our validated `judge_eval` rubric. **Sample ~10–20%** of live queries (async,
  rubric prefix-cached) for an honest quality read; available on-demand for a single query.
- **Tier 2 — RAG health monitor (regression detection).** Productize the eval harness as
  `librarian health`: run the golden probe set, report hit-rate / MRR / Tier-1 judge score, track
  over time, **re-run after every ingest**; a drop is the "added data hurt us" alarm. Add a cheap
  drift check (static probe-doc re-embedding) for the partial-re-embed failure.

Tier 0 is new (small, well-grounded). Tiers 1–2 largely *productize what we built this session*.

## Honest caveats (truth_mode)
- The cheap per-query number is a **confidence/triage** signal (dense QPP r≈0.2–0.4), not a
  precise quality grade. Don't oversell it; calibrate it against the golden set.
- The **reliable** quality number comes from Tier 1 (LLM-judge, sampled) and Tier 2 (golden set).
- Context-relevance is the hardest thing for an LLM judge to score reliably — pair with groundedness.
- An aggregate-of-live-queries score conflates query-mix changes with real regressions; **use the
  fixed golden set to attribute a drop to the data**, not a rolling average over arbitrary queries.

## Sources (papers in this folder)
- QPP survey — Meng et al. 2023, arXiv:2305.10923 (formulas: Clarity/WIG/NQC/SMV/UEF).
- QPP for Neural IR — Faggioli et al., arXiv:2302.09947 (the dense-degradation r-values).
- Coherence predictors for dense QPP — Vlachou & Macdonald, arXiv:2310.11405.
- Calibrated similarity (Platt/isotonic on cosine) — arXiv:2601.16907.
- kNN OOD detection — Sun et al. ICML 2022, arXiv:2204.06507.
- RAGAS — Es et al., arXiv:2309.15217. G-Eval — arXiv:2303.16634. LLM-as-judge — Zheng et al., arXiv:2306.05685.
- Monitoring/regression patterns — TruLens RAG-Triad docs, Phoenix, Langfuse, Galileo, premai/decompressed.io
  guides (URLs in `WEB-NOTES_rag-quality-monitoring-regression.md`).
