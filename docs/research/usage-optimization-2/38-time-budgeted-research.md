# Anytime / time-budgeted research: quality vs budget, and how to split it across search/read/write

**Scope.** Round-1 settled single-query mechanics (verbatim query, k=20 / k=8 value point,
quote-first, abstention). The sibling notes 19 and 24 cover the *count* axis (how many searches,
stopping rules). This note is the orthogonal *budget-allocation* axis: treat a librarian-using
agent as an **anytime** system with a finite total budget (tokens, tool calls, wall-clock), and
ask (a) how answer quality scales with that budget, (b) where to *spend* the marginal unit —
on more searching, more reading of retrieved chunks, or more synthesis/writing — and (c) how
that split should change by task type. The theory is decades old; the LLM numbers are 2024-2026
and benchmark-specific (read as shape, not transferable thresholds).

## The theory: anytime algorithms and value-of-computation

An **anytime algorithm** can be interrupted at any point and return its best-so-far answer, with
quality rising (ideally monotonically) as more resources are spent (Dean & Boddy 1988; Zilberstein
& Russell 1995). The control problem — *which* computation to run next, and *when to stop* — is
**rational metareasoning**: Russell & Wefald's **Value of Computation** VOC(c) = expected gain in
decision quality from computation c, minus its cost; **stop when no available computation has
positive VOC** ([Rational Metareasoning for LLMs](https://arxiv.org/abs/2410.05563)). Allocating
deliberation time across sub-problems is provably hard even with perfect *performance profiles*
(quality-vs-time curves) ([Metareasoning complexity](https://arxiv.org/pdf/cs/0307017)), which is
why practical systems use a *myopic* one-step VOC approximation. De Sabbata et al. operationalised
VOC as a reward that penalises unnecessary reasoning and cut **20-37% of generated tokens across
three models at maintained accuracy** — the model *learned* to spend tokens only when they pay off.
This is the spine of the whole note: the librarian agent should run the next search/read only while
its estimated marginal quality gain exceeds its cost, and our per-chunk confidence label is a cheap
VOC proxy.

## Quality-vs-budget curves have a knee and often turn down

LLM reasoning behaves like an anytime system but with a *non-monotonic tail*. Budget-aware RL
(BRPO) explicitly trains for the interruptibility + monotonicity properties
([Anytime Reasoning](https://arxiv.org/pdf/2505.13438)). But on agentic *research* (not maths), more
budget frequently *hurts*: a static thinking-budget sweep rose 24.3% (256 tok) → 32.5% (8192 tok)
then **fell to ~26% at 16384 tok**, while inference time climbed 6.5s → 19s; an *Auto/dynamic*
budget matched the best static point at lower cost
([Learning When to Plan](https://arxiv.org/html/2509.03581v1)). FutureSearch's controlled "effort
paradox" is the sharpest version: on 150+ real web-research tasks, **GPT-5 declined monotonically
49.6% (low) → 48.6% (med) → 48.1% (high)** and Gemini 3 Flash dropped ~2 pts low→high, for strictly
more cost — because "the bottleneck in research is information retrieval and source evaluation, not
step-by-step deduction," so extra reasoning budget gets spent second-guessing good findings
([Effort paradox](https://futuresearch.ai/effort-paradox/)). Notable exception: Anthropic's 4.6
generation *does* benefit from higher effort on web research. This is the strongest task-conditioning
result available: **the maths/coding intuition that more thinking helps does not transfer to
retrieval-bound work.**

## Where to spend the marginal unit: search > read-quality > generate

The most directly relevant empirical study sweeps the three knobs of budget-constrained agentic RAG
and gives an explicit **allocation priority order**: "expand search depth first, improve evidence
quality through retrieval and re-ranking, then raise completion budgets when synthesis demands
justify the added cost" ([Budget-constrained agentic RAG](https://arxiv.org/html/2603.08877v1)).
Concretely: accuracy rises with searches up to **~3 then flattens**; hybrid retrieval + re-ranking
is the single largest lever (**+9.29 pts on HotpotQA** vs +6.36 for hybrid alone); completion-token
budget is **task-gated** — nearly flat on TriviaQA (lookup) but a sharp jump 4K→16K on HotpotQA
(synthesis), and *retrieval-bound* (not generation-bound) on 2WikiMultihop. A counterintuitive
finding worth replicating: **moderately restrictive token budgets (500-2K) + more searches beat
generous single-search configs**, because tighter budgets make the model terse and free up tool
calls. The search-vs-reasoning split is itself task-dependent: search tokens trade against reasoning
tokens, and "search-heavy tasks such as multi-hop RAG or literature surveys prioritise broader
content access, while reasoning-heavy tasks like causal analysis or maths require deeper internal
processing" ([Agentic Deep Research](https://arxiv.org/pdf/2506.18959)).

## Wall-clock and depth ceilings for synthesis/writing

For literature-synthesis/writing work, depth gains **plateau ~9 deepening steps** (Comprehensiveness
and Insight +~6 pts shallow→deep, then flat) ([AgentCPM-Report](https://arxiv.org/pdf/2602.06540)),
and there is a hard **collapse beyond ~4 sequential inference steps / ~35 min** human-equivalent time
across all evaluated deep-research systems. Breadth and accuracy are in *fundamental* conflict:
Gemini's 111-citation breadth gives 81% accuracy; Perplexity's 31-citation coverage gives 90%
([ResearchRubrics](https://arxiv.org/pdf/2511.07685)). For breadth-then-depth scheduling, a
**Descending** policy (broad exploration early, focused exploitation late) beat static and ascending
schedules ([W&D](https://arxiv.org/pdf/2602.07359)).

## Actionable implications for librarian usage experiments

1. **Define a per-task budget and a performance profile, then sweep it.** Fix a total budget (tool
   calls × k × generation tokens) and measure quality-vs-budget *curves per task type*
   (lookup / multi-hop synthesis / maths / writing / coding). Locate each task's knee; expect lookup
   to peak almost immediately and writing/synthesis to plateau ~9 deepening steps.
2. **Allocate in priority order search → read-quality → generate, and verify the order holds here.**
   Default: spend the first budget units on coverage (more chunks / a re-rank pass), raise the
   generation budget only for synthesis tasks. Replicate the "tight gen-budget + more searches >
   generous single search" result on the librarian.
3. **Make confidence the VOC stop-signal.** Continue searching/reading only while the marginal
   expected gain (proxied by returned chunk confidence) exceeds cost; stop when no next action has
   positive estimated VOC. Calibrate to a ~10-20% re-query budget and log token savings against an
   always-search baseline (target the De Sabbata 20-37% range).
4. **Test the effort paradox directly on retrieval-bound tasks.** Run librarian queries at low/med/high
   reasoning budgets and confirm whether, like GPT-5/Gemini, *more* thinking degrades retrieval-bound
   answers while only maths-style sub-questions reward it. If so, default the agent to low effort for
   pure lookup/synthesis and reserve high effort for the maths building-block case.
5. **Treat breadth and accuracy as a Pareto choice, not a target.** For synthesis/writing, expose
   breadth (citations/chunks) as a tunable operating point and report the accuracy it costs, rather
   than maximising coverage; prefer a Descending breadth→depth schedule.
6. **Cap wall-clock / sequential depth and flag the ceiling.** Given the universal collapse beyond
   ~4 sequential steps / ~35 min, set a hard depth cap, surface remaining budget to the agent, and
   treat hitting the cap as a review signal rather than rewarding longer runs.
