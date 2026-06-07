# Reusing retrieved context across turns: reuse vs re-retrieve, and staleness

**Scope.** Round-1 settled *single-query* mechanics on the librarian (verbatim query, k=20 / k=8 value point, quote-first generation, abstention contract → hallucination 12%→0%). This note covers the *cross-turn* layer: once chunks are already in the conversation, when should an agent reuse them, when must it re-retrieve, and how does carried context go stale? This is orthogonal to file 24 (how many calls *per query*); here the unit is the *conversation*.

## The reuse/re-retrieve decision is a learnable per-turn gate

Multi-turn RAG's defining problem is that follow-ups lack standalone meaning ("what about that one?") and that naively dumping the whole history into the retriever is *redundant and sub-optimal*. The settled architecture is a per-turn **retrieve-or-not gate** rather than retrieve-every-turn. SELF-multi-RAG (arXiv 2409.15515) trains the model to decide whether retrieval is needed given the conversation, and only then rewrite the turn into a standalone query and judge returned passages; it **calls retrieval fewer times than the single-turn baseline** while generating better responses when context contains both dialogue and previously-retrieved passages. RouteRAG (arXiv 2512.09487) formalises this as sequential decision-making: at each step the policy emits *continue-reasoning / retrieve / answer* under a step budget — i.e. reuse-in-context is the default and retrieval is an explicit action, not a reflex.

**Carrying old passages forward is an explicit accuracy↔efficiency dial.** SELF-multi-RAG reports both configurations (retrieve-with-prior-passages vs retrieve-without) are valid operating points: keep prior chunks when accuracy dominates, drop them when efficiency does. For the librarian this maps directly onto "should the agent re-issue a query it already answered from chunks still visible in context?" — usually no.

## Reuse fails in a specific way: the agent ignores context it already has

The dominant *empirical* failure is not over-retrieval but **redundant re-retrieval despite having the data**: agents re-issue identical tool calls even when prior tool inputs/outputs are in context, inflating latency and cost ("thrashing"). The fix found across the FaaS/MCP-agent work (arXiv 2601.14735) is purely prompt-level — instruct the agent to *check previous tool-message responses before making a new call and extract from prior outputs instead of re-calling with the same parameters*; after this the agents stopped repeating work. This is the cheapest lever available to us and needs no librarian change.

## Reuse has a hard ceiling: context rot and "retrieval laziness"

Reused chunks are not free — they consume the attention budget, and recall degrades as context fills. Anthropic's context-engineering guidance names this **context rot**: needle-in-haystack recall falls as token count grows, across all models; context is a finite resource with diminishing returns. Reported magnitudes: recall accuracy drops **~15–30% from ~8K to 128K tokens**. The sharpest task-conditioned number is **retrieval laziness** (*Fishing for Answers*, arXiv 2509.04820): the probability the model issues a follow-up retrieval collapses with context length — **95% at 3k tokens → 50% at 9k → 25% at 12k**. So *accumulating* carried chunks actively suppresses the very re-retrieval a later turn needs. Carry-forward and re-retrieve-readiness trade off against each other.

## Staleness: carried context can be valid yet wrong-for-now

Two distinct staleness mechanisms appear. (1) **Cache pollution / topic drift**: Dynamic Context Tuning (arXiv 2506.11092) keeps an attention-based KV cache of prior intent + tool results; it resolves **73% of co-referential turns via caching** but logs **8% of errors as stale entries surfacing after topic drift** in long sessions. (2) **Supersession**: retrieving a past statement does not tell you whether it has since been revised or retracted — RAG alone has no temporal/validity signal. For a *static* corpus like the librarian the corpus chunks themselves don't go stale, but the *carried selection* does: chunks retrieved for an earlier sub-topic become noise once the conversation pivots, and they then both pollute reasoning and (per laziness) deter the fresh retrieval the new sub-topic requires. Practical re-retrieval triggers named across sources: sudden topic shift, contradiction of an earlier established fact, ambiguous/vague intent, detectable goal change.

## Task-conditioning: one-shot vs iterative converge in accuracy, diverge in cost

*Fishing for Answers* (arXiv 2509.04820) is the sharpest task-conditioned datapoint: one-shot and iterative retrieval reach **near-identical accuracy (91% vs 90%, both +9–10 pts over basic Top-5 RAG)** — "different paths, same goal" — but **one-shot is cheaper in runtime**, and iterative's advantage concentrates on hard, multi-hop questions (L3 +14%, L4 +19%); average retrievals stay low (**1.59–1.97**, ~2× for hard questions). Crucially, *naive* agentic iteration **degrades simple questions** (L1 89.5%→86.5%) until a fallback path is added (+3.5% recovery), and **combining** one-shot + iterative *underperformed both* because the longer post-retrieval context broke the chunk-pruning step — direct evidence that more carried context is not monotonically good. This mirrors file 24's task-shaped stopping: lookup-style turns want reuse + at most one fresh shot; synthesis/multi-hop turns justify iterative re-retrieval.

## Actionable implications for librarian usage experiments

1. **Test an explicit reuse-or-re-retrieve gate, not retrieve-every-turn.** Before any new librarian call, the agent checks whether the answer is derivable from chunks already in context (SELF-multi-RAG / RouteRAG). Measure redundant-call rate and total chunks read per conversation; hypothesis: large cost cut, no recall loss.
2. **The cheapest win is a prompt instruction to consult prior chunks first.** Replicate the FaaS/MCP fix verbatim before any architectural change; it eliminated thrashing there at zero retrieval-side cost. This is the control arm.
3. **Decontextualise the follow-up before re-querying** (verbatim-but-standalone): resolve pronouns/breadcrumb references into a self-contained query — but keep Round-1's verbatim-beats-rewrite result by rewriting *only for standalone-ness*, not paraphrasing. Compare against raw-follow-up and against summarised-history-as-query.
4. **Treat carried chunks as a budget with a staleness clock, not a free cache.** On a detected topic pivot (new collection, contradiction, or breadcrumb-prefix change) *evict* prior chunks rather than accumulate. This directly counters retrieval-laziness (95%→25%) and DCT cache-pollution (8%). Measure laziness on the librarian: does follow-up re-query probability fall with carried-chunk volume?
5. **Condition carry-forward policy on task type.** Lookup/definition turns: reuse, no re-retrieve, drop prior chunks fast. Synthesis/multi-hop turns: allow iterative re-retrieval but verify sufficiency before each extra call, and add a fallback to avoid the simple-question regression. Do **not** blend strategies blindly — the combined approach hurt.
6. **Re-use the abstention/confidence label as the carry-forward gate too.** If a turn is answerable from high-confidence carried chunks, suppress retrieval; if carried chunks are low-confidence or off-topic for the new turn, force a fresh retrieve. Calibrate to a ~10–20% re-query budget (consistent with file 24).

## Sources
- SELF-multi-RAG / *Learning When to Retrieve, What to Rewrite* — arXiv 2409.15515
- RouteRAG — arXiv 2512.09487
- Dynamic Context Tuning (DCT) — arXiv 2506.11092
- *Fishing for Answers* (one-shot vs iterative; retrieval laziness) — arXiv 2509.04820
- *Optimizing FaaS Platforms for MCP-enabled Agentic Workflows* (redundant tool-call fix) — arXiv 2601.14735
- Anthropic, *Effective context engineering for AI agents* (context rot; tool-result clearing; memory tool)
- ChatQA — arXiv 2401.10225
