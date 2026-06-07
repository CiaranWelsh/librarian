# Interleaving Search with Reasoning and Writing: When to Retrieve in a Long Task

**Scope.** Round-1 settled single-query mechanics (verbatim query, k=20/k=8, quote-first, abstention). This note asks the orthogonal, task-conditioned timing question: across a long reasoning or writing trace, *when* should the assistant call the librarian and how should that interleave with generation. Two archetypes recur in the literature: **search-then-write** (retrieve before you commit a claim) and **write-then-verify** (draft from parametric knowledge, then retrieve to check/repair). The evidence says the right pattern is task-dependent, and that *timing* — not just query quality — is a first-class lever.

## Key finding 1: Interleaving beats one-shot retrieval for multi-step work

IRCoT (Trivedi et al., ACL 2023) is the anchor result. Single up-front retrieve-then-read is insufficient when "what to retrieve depends on what has already been derived," so IRCoT alternates: generate the next CoT sentence, use that sentence as the next query, repeat until the answer or a step cap. On HotpotQA / 2WikiMultihopQA / MuSiQue / IIRC it lifted retrieval by **up to 21 points** and downstream QA by **up to 15 points** over one-shot RAG, with gains holding OOD and on small models — and it **reduced hallucination**. The explicit caveat: interleaving's overhead only pays off for *genuinely multi-hop* tasks; on simple questions it can hurt. This maps our literature-synthesis vs. lookup split onto a retrieval-pattern split.

## Key finding 2: Trigger retrieval on uncertainty and on *future* intent, not on the past

FLARE (Jiang et al., EMNLP 2023) operationalises *when*: generate a candidate next sentence, and only if a token falls below a probability threshold treat that low-confidence span as a retrieval signal; the forward-looking lookahead (the draft sentence itself becomes the query) beats querying from prior context. Two transferable numbers: best results came from triggering on **40–80% of sentences** (not every step, not once), and using **>32 past tokens as the query hurt** — past context is a poor proxy for what you are about to write. This is a write-a-little / check / continue rhythm: write-then-verify at sentence granularity.

## Key finding 3: Make "retrieve?" an explicit, learnable decision

Self-RAG (Asai et al., ICLR 2024) shows that indiscriminate fixed-k retrieval "diminishes versatility" and can degrade output; the model instead emits a *retrieve* reflection token on demand (zero, one, or many times per generation) plus *critique* tokens scoring support. Retrieval frequency is tunable at inference via the retrieve-token probability. Self-RAG-7B/13B beat ChatGPT and retrieval-augmented Llama2-chat on QA, reasoning, fact-verification, and long-form citation accuracy. The lesson for us: the decision to query is itself a control point, and over-retrieval has a real downside, not just a latency cost.

## Key finding 4: Agents systematically mis-time retrieval — over-search and under-search

The agentic-search literature names the failure modes precisely. **Over-search** = a retrieval step whose answer was already derivable from internal knowledge + prior context; **under-search** = a no-retrieval step that produces a wrong answer. The root cause is a *misaligned decision boundary* — the agent cannot judge when accumulated context suffices. "Search Wisely" (Wu et al., 2505.17281) reports one model could have skipped **27.7% of its search steps**, finds response accuracy correlates with the model's *confidence in its search decision*, and its β-GRPO (reward high-certainty search decisions) gives a 3B model **+4% average exact-match** across seven QA benchmarks while cutting redundant searches. Real-trace analysis (2601.17617): >90% of sessions are ≤10 steps, fact-seeking sessions loop/repeat most, reasoning sessions sustain broader exploration.

## Key finding 5: Write-then-verify is a legitimate, distinct pattern

RARR (Gao et al., ACL 2023) inverts ordering entirely: generate first, then for each claim produce verification questions, retrieve, run an agreement gate, and edit only contradicted spans — preserving **>90% of original content**. This is the right shape when the draft is mostly parametric-recall correct and retrieval's job is attribution/repair, not construction. Contrast with search-then-write (STORM, below), which is right when grounding must precede commitment.

## Key finding 6: For long-form writing, plan first, then retrieve per section

STORM (Shao et al., 2024) separates pre-writing from drafting: build a draft outline from parametric knowledge, refine it with retrieved evidence and perspective-guided question-asking, then draft **section by section**, retrieving per heading. Ablations show outline-driven RAG (oRAG) **> plain RAG > direct generation**; 70% of editors found it useful for pre-writing. Coding agents show the analogous discipline: most tokens go to a *discovery phase* before any code, but the AGENTS.md study (2602.11988) found developer context files gave only **+4%** while LLM-generated ones gave **-3%** and raised cost **>20%** — favouring minimal non-inferable up-front context plus on-demand retrieval. Augment's *selective retrieval* (skip what the base model already knows from training, fetch only project-specific context) doubled effective context to 200k tokens.

## Actionable implications for librarian usage experiments

1. **Pick the pattern by task, and test both.** Run search-then-write (retrieve before each claim) against write-then-verify (draft then audit) on (a) detector-physics synthesis and (b) parametric-recall writing. Predicted: synthesis wants search-then-write/per-section (STORM); recall-heavy prose wants RARR-style post-hoc verification with smaller total retrievals.
2. **Trigger queries on the assistant's own uncertainty, forward-looking.** Implement a FLARE-style hook: draft the next sentence/claim, query the librarian only when the assistant flags low confidence, and form the query from the *drafted claim*, not the prior paragraph. Measure against query-on-every-claim and query-once.
3. **Instrument over-search / under-search directly.** Log every step as retrieved/not; post-hoc judge whether each retrieval was necessary (answerable from context) and whether each non-retrieval step erred. Target the 27.7%-redundant benchmark and report a search-efficiency curve, not just accuracy.
4. **Cap and stop on a sufficiency signal.** Adopt an IRCoT-style step cap plus an explicit "enough evidence" stopping rule; correlate early-stop with correctness to calibrate the boundary (the under-search risk lives here).
5. **For multi-section deliverables, retrieve per heading, not per document.** Outline first from parametric knowledge, then scope each librarian query to a section; compare oRAG-style per-section retrieval against one bulk top-k over the whole brief.
6. **Extend the abstention contract to abstaining-from-search.** Make "decline to query" a first-class, logged outcome (Self-RAG retrieve-token analogue); expect declining-on-recall-tasks to correlate with success and to cut latency without hurting accuracy.

## Sources
- Trivedi et al., IRCoT — https://arxiv.org/abs/2212.10509 (ACL 2023)
- Jiang et al., FLARE / Active Retrieval Augmented Generation — https://arxiv.org/abs/2305.06983 (EMNLP 2023)
- Asai et al., Self-RAG — https://arxiv.org/abs/2310.11511 (ICLR 2024)
- Gao et al., RARR — https://arxiv.org/abs/2210.08726 (ACL 2023)
- Shao et al., STORM (Assisting in Writing Wikipedia-like Articles From Scratch) — https://arxiv.org/pdf/2402.14207
- Wu et al., Search Wisely (β-GRPO) — https://arxiv.org/pdf/2505.17281
- Agentic Search in the Wild (trajectory dynamics) — https://arxiv.org/html/2601.17617
- HiPRAG: Hierarchical Process Rewards for Agentic RAG — https://arxiv.org/pdf/2510.07794
- Evaluating AGENTS.md: Repository-Level Context Files for Coding Agents — https://arxiv.org/html/2602.11988v1
- Retrieval-Augmented Code Generation: A Survey (repo-level) — https://arxiv.org/abs/2510.04905
