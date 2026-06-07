# Tool-call frequency and patterns in successful agent trajectories

**Scope.** Round-1 settled single-query mechanics on the librarian (verbatim query, k=20 / k=8 value point, quote-first, abstention contract). This note surveys the published evidence on *how many* tool calls a successful agent makes, *how* it refines, and *when* it stops — the task-conditioned layer Round-2 must design experiments for.

## Over- vs under-searching are the two named failure modes

Recent work formalises agent search behaviour around a *decision boundary*: the threshold where parametric + retrieved knowledge becomes sufficient to answer. **Over-search** (querying when the answer is already available) and **under-search** (stopping early / trusting parametric knowledge wrongly) are the two failure modes on either side of it (DAS, *To Search or Not to Search*, arXiv 2602.03304). Crucially the error pattern is **task-shaped**: single-hop NQ shows an abrupt flip — high over-search at step 1, then critically high under-search immediately after — whereas multi-hop datasets show *gradual* deterioration, with under-search rate climbing steadily at every reasoning step. So the optimal stopping rule for a one-shot fact lookup differs structurally from a compositional question.

**Over-searching is large and measurable.** *Over-Searching in Search-Augmented LLMs* (arXiv 2601.05503) finds models run ~**70.5% more searches than necessary** (0.620 vs optimal 0.364 per query), and that search *improves answerable-query accuracy by +24.0% but degrades abstention by 12.8%* on unanswerable queries. Cost is non-linear: Tokens-Per-Correctness rises from ~300–400 (base) to ~730–812 (search-augmented) to **38.9k for a Deep Research system (221×)**. A noisy corpus (low retrieval quality) amplified searching **3.6×** — directly relevant to us, since our chunks carry a confidence label. The worst behaviour is repeated searching on fundamentally unanswerable queries — which our abstention contract should suppress, but only if it fires *before* the agent loops.

## More tool calls correlate with *failure*, not success — but it's confounded

On research/browsing benchmarks (BrowseComp, GAIA), tasks resolved in **fewer tool-call turns achieve higher accuracy**, read as better planning (*Scaling Agents via Continual Pre-training*, arXiv 2509.13310, Fig. 13). On **SWE-bench**, the original SWE-agent analysis found action *distributions* were nearly identical between solved and failed tasks — model capability, not action count, gated resolution (arXiv 2405.15793). Successful SWE-agent runs are short: median **11 steps, ~2 distinct files viewed** across 256 successful trajectories. Per-task tool-call counts vary 3× by model family on SWE-bench Verified — GPT-5 family **13.45–14.67 calls**, Qwen **34.63–50.56** (TRAJEVAL, arXiv 2603.24631) — so "calls per task" is a model property, not a fixed target. Trajectory length tracks *failure severity* more than success. Takeaway: long trajectories are a *symptom*, not a lever; don't reward call count directly.

## Depth/iteration and breadth/k both scale with task complexity

For *iterations* (rounds of retrieval): single-hop PopQA converges in **2–3 steps**; multi-hop 2WikiMultiHopQA / HotpotQA need **3–5 steps** (Adaptive-RAG line). For *breadth* (top-k): k=1 starves reasoning, k=3 gives large gains, k=5 plateaus on simpler sets while harder multi-hop sets keep benefiting; **answer-correctness peaks at 3–5 chunks**, faithfulness *declines* with more chunks (noise dilution), and very long reasoning chains show an *intermediate* optimal k (more noise-sensitive, not less). Adaptive-RAG's trained classifier routes each query to **no-retrieval / single / multi-step** — the canonical task-conditioning result.

## When to stop / when to search: confidence gating

FLARE searches when next-token probability drops below threshold; Self-RAG learns reflection/retrieve tokens; CRAG gates on retrieved-passage quality; TARG makes a single training-free up-front decision from prefix-logit margin, calibrated to a retrieval budget (~5–20% of queries). The shared lesson: a **confidence/uncertainty gate** beats both always-search and never-search. Our librarian already returns a confidence label — we can gate the *next* call on it rather than on a fixed budget.

**Test-time scaling saturates early.** DeepSearchQA: n=2 sampling → 74.51%, n=4 → 81.72% (diminishing fast); but below a compute threshold agents suffer *trajectory divergence* — a step-function drop, not a smooth one. Parallel-sampling gains are gated by having a *verifier* to select the right output; without one, more samples barely help and majority-vote can amplify bias.

## Actionable implications for librarian usage experiments

1. **Route by task type, not a global k/iteration budget.** Test an Adaptive-RAG-style up-front classifier (lookup / synthesis / multi-hop) that sets retrieval depth: 1 call for definition/API lookups, 3–5 iterative calls for synthesis. Measure resolution vs total chunks read.
2. **Make the abstention/confidence label the stopping gate.** Mirror FLARE/TARG: continue searching only while the returned confidence is below threshold; calibrate the threshold to a ~10–20% re-query budget. Hypothesis: kills the 70% over-search overhead without hurting recall.
3. **Instrument over- vs under-search directly.** For each task, log whether an answer was reachable without the marginal query (over-search) and whether early stopping caused an error (under-search) — the DAS metrics. Expect under-search to dominate on synthesis/multi-hop, over-search on single lookups.
4. **Cap depth and treat long trajectories as a failure signal, never a reward.** Given SWE-agent's ~11-step successes and the browsing "fewer-turns-win" result, set a hard iteration cap and flag runs that hit it for review.
5. **Verify before re-querying on hard tasks.** Where parallel/repeated retrieval is used (literature synthesis), add a cheap sufficiency check before spending another call — without a verifier, extra calls add cost, not accuracy.
6. **Stress-test with deliberately noisy / low-confidence chunks.** The 3.6× over-search amplification under noisy corpora predicts our weakest collections (sparse particle-physics topics) will trigger the most wasteful looping; measure call count as a function of returned confidence.

## Sources
- DAS, *To Search or Not to Search* — arXiv 2602.03304
- *Over-Searching in Search-Augmented LLMs* — arXiv 2601.05503
- *Scaling Agents via Continual Pre-training* — arXiv 2509.13310
- *SWE-agent / Agent-Computer Interfaces* — arXiv 2405.15793
- TRAJEVAL — arXiv 2603.24631
- *Understanding Code Agent Behaviour* — arXiv 2511.00197
- DeepSearchQA — arXiv 2601.20975
- Adaptive-RAG / HopRAG — arXiv 2502.12442
- TARG (training-free adaptive gating) — arXiv 2511.09803
- Self-RAG; FLARE (active retrieval); CRAG
