# Search-budget curves: how many searches, and when to stop

**Scope.** Round-1 settled single-query mechanics (verbatim query, k=20/k=8, quote-first,
abstention). This note is the *quantity* axis: how outcome quality scales with the *number*
of searches in an agent trajectory, where diminishing returns set in, and how the optimal
budget is conditioned on task type. Findings are drawn from agentic-search test-time-scaling
work and adaptive-RAG literature; numbers are benchmark-specific and should be read as shape
guidance, not transferable thresholds for the librarian.

## The curve is real, has a knee, and then turns down

Multiple ablations agree on a three-phase shape for sequential search budgets. A CMU
test-time-scaling analysis measured turns at {1,3,5,7,10,15,20+} and found peak performance at
**3-7 turns**: turns 1-5 add roughly +5% each (refinement/error-correction), turns 6-10 add
roughly +1% each (plateau), and turns 11+ actively *degrade* below the turn-5 baseline as
accumulated failed-attempt context pollutes reasoning ("context pollution, not insufficient
reasoning, drives the cliff")
([Effloow/CMU](https://effloow.com/articles/agent-test-time-compute-scaling-context-ceiling-2026)).
Synthesis-planning work shows the same knee on retrieved-example count: 3 RAG examples reach an
86% solve rate at 1,478 tokens; 20 examples add only +2% while inflating tokens by 177%
([AOT*](https://arxiv.org/pdf/2509.20988)). A membership-inference ablation independently found
AUC rising with query count but saturating at a clear point
([Riddle Me This](https://arxiv.org/pdf/2502.00306)).

## "More budget" alone does nothing; budget *awareness* is what scales

The most important negative result: simply raising the tool-call cap does not help, because
agents lack budget awareness and hit a ceiling (and often terminate *prematurely* well inside
the cap). "Budget forcing" pushed GLM-4.5 from 19%->27% and Qwen3 from 8%->18% on BrowseComp,
but excessive forcing then degrades
([Asymmetric Verification](https://arxiv.org/html/2510.06135)). Making the budget explicit to
the agent fixes this: a Budget Tracker matched ReAct accuracy with **40.4% fewer search calls
and 31.3% lower cost**, and BATS (adaptive dig-deeper-vs-pivot on remaining budget) avoided
ReAct's plateau and beat parallel majority-vote on cost (>37% accuracy at ~$0.23 vs >$0.50);
its ablation showed verification is load-bearing (18.7%->15.4% on BrowseComp when removed)
([Budget-Aware Tool-Use](https://arxiv.org/abs/2511.17006)).

## The optimal budget is task-conditioned

Routing by query complexity beats a fixed budget. **Adaptive-RAG** trains a small classifier to
route each question to *no-retrieval / single-step / multi-step*, labelled by which strategy
actually succeeded, and shows that one policy is wrong for mixed workloads: multi-step is
wasteful on simple queries, single-step fails complex multi-hop ones
([Adaptive-RAG](https://arxiv.org/pdf/2403.14403)). The task-type effect is sharp:

- **Knowledge-intensive / fact-lookup:** retrieval helps most; depth in *documents* shows flat
  returns (small k often suffices, more adds noise), so spend the budget on coverage not depth
  ([AutoBnB-RAG](https://arxiv.org/pdf/2508.13118)).
- **Maths / deep reasoning:** retrieval helps *little* once the bottleneck is reasoning depth -
  RAG "struggles to assist deeper reasoning" ([How Much Can RAG Help](https://arxiv.org/html/2410.02338v1));
  retrieval can even *underperform* retrieval-free reasoning when noisy/conflicting context
  overrides correct parametric knowledge ("context dominance"). It helps only when it supplies a
  missing factual building block (a formula, a definition).
- **Multi-hop synthesis:** genuinely needs multiple searches, but *how* matters more than count.
  Iterative refinement suffers early-hop error propagation; up-front decomposition risks entity
  drift; hybrids (decompose then iterate) recover the most complete evidence set
  ([PRISM](https://arxiv.org/html/2510.14278v1)). Cost can be decoupled from hop depth -
  CompactRAG fixes LLM calls at two (decompose + synthesise) regardless of hops
  ([CompactRAG](https://arxiv.org/html/2602.05728)).
- **Simple/well-formed queries:** extra agentic querying *degrades* via "query drift" - the
  model reformulates away from an already-good query
  ([Fishing for Answers](https://arxiv.org/pdf/2509.04820)).

## Stopping rules

Three families: prompting (model says stop: IRCoT, Search-o1), confidence (FLARE token-prob
threshold, DRAGIN entropy+attention), and learned (Self-RAG reflection tokens; Stop-RAG learns
a value function over the trace) ([Stop-RAG](https://arxiv.org/html/2510.14337v1)). A 2026 result
worth heeding: adaptive retrieval "helps reasoning - but mostly if it's not used"; the *decision
to abstain from retrieving* correlates with good performance, and retrieval frequency scales
with problem difficulty as a metacognitive signal
([arXiv 2602.07213](https://arxiv.org/abs/2602.07213)). Parallel scaling (Best-of-K) raises the
ceiling far above sequential (GLM-4.5 67% Pass@32) but only with a real verifier - self-selection
and majority vote are weak ([Asymmetric Verification](https://arxiv.org/html/2510.06135)).

## Actionable implications for librarian usage experiments

1. **Make the budget a first-class, task-conditioned variable, not a constant.** Sweep search
   count {1,2,3,5,8} crossed with task type (lookup / multi-hop synthesis / maths / writing /
   coding). Expect the lookup curve to peak at 1-2, synthesis at 3-7, maths near 0-1.
2. **Test a complexity router before testing more searches.** Reproduce Adaptive-RAG's
   no/single/multi routing on librarian queries; the win is usually in *not* searching cheap
   queries, not in adding searches to hard ones.
3. **Instrument budget awareness.** Tell the agent its remaining search budget (the cheapest
   intervention with the largest published effect: ~40% fewer calls at equal accuracy) and
   measure premature-stop vs over-search rates.
4. **Add an abstention/stop signal as a measured outcome.** Round-1 already has a confidence
   label per chunk - test whether "stop when top-chunk confidence clears threshold" matches
   Stop-RAG-style value control, and whether *not retrieving* on maths beats retrieving.
5. **For multi-hop, compare decompose-then-iterate vs naive iteration** and log per-hop recall
   to catch error propagation and entity drift; consider a fixed two-call (decompose+synthesise)
   budget as the efficiency baseline.
6. **Watch for the degradation cliff and query drift.** Log accuracy beyond the knee (8+ searches)
   to confirm the down-turn on the librarian, and check that re-querying on already-answerable
   prompts does not drift away from a good verbatim query.
