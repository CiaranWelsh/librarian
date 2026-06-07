# 47. Retrieval Tool API Design for LLM Agents

How the *shape* of a retrieval tool — its result-count defaults, pagination, response verbosity, and (above all) its description text — steers an agent's usage strategy. This matters for the librarian because the CLI surface is the only lever we have for task-conditioning short of fine-tuning Claude: the tool contract is itself a prompt the model reads on every turn.

## Key findings

**Tool descriptions are a high-leverage control surface, not documentation.** Anthropic's own tooling guidance treats descriptions as context that is loaded on every turn and that "collectively steers agents toward effective tool-calling behaviors"; they advise writing them as you would brief a new hire, making implicit query formats and terminology explicit, and naming parameters unambiguously. Microsoft's Learn MCP team found small wording changes "swing tool activation rates materially" and built an automated eval loop to iterate descriptions against observed agent behaviour. Academic work on "MCP tool smells" (arXiv 2602.14878) frames poor descriptions as latent design defects that degrade tool selection, parameterization, and multi-step orchestration. Implication: the librarian's `--help`/description text is the primary place to encode task-conditioned strategy, and it should be A/B-tested, not hand-waved.

**Result-count defaults: small, with an inverted-U justification.** Anthropic recommends a "sensible 50-item default limit" for generic tools, but RAG-specific evidence pushes lower and makes the optimum task-dependent. RAGGED (arXiv 2403.09040) swept k=1..50 and found two model families: monotone-improvers vs. early-peak-then-decay, with "most insightful variation before k=30." Single-hop QA typically saturates at k≈3–5; multi-hop benefits from larger k because evidence is dispersed and lower-ranked (HotpotQA optimum k=5, MuSiQue k=3; raising k 2→8 lifted recall up to 13%/53% respectively but also admits distractors). A chemistry benchmark saw +1.29 only going k=5→15; an EvoWiki study saw the curve flatten and turn down past k=15. This directly corroborates the librarian's Round-1 result (k=20 best, k=8 the value point) and predicts the optimal k will *shift by task*: lower for known-item/definitional lookups, higher for synthesis/multi-hop.

**More context is not free — distractors, precision loss, and "lost in the middle."** Increasing passages raises recall but lowers precision, and system accuracy tracks *below* recall at every k — relevant text present in context is not guaranteed to be used (arXiv 2512.14313). Long-context RAG adds hard negatives and positional bias. Mitigations that let you safely raise k: rerankers (suppress top-k sensitivity, most stable), noise-filtering, and dynamic/adaptive-k classifiers that predict per-query context size. The librarian's confidence label is a lightweight analogue of this — a signal the agent can use to decide whether to widen k or stop.

**Iteration budgets and stopping rules dominate cost in agentic loops.** Agentic RAG is a control loop (plan→retrieve→evaluate→decide), and "stop-condition design is as important as the retrieval strategy." Production guidance converges on a hard cap of ~3 retrieval cycles, then best-effort answer with a confidence disclaimer. Multi-step loops cost 3–10x the tokens of single-pass RAG, motivating routing simple queries away from iteration entirely. The dominant failure modes (Towards Data Science, "Retrieval Thrash / Tool Storms / Context Bloat") trace to weak stopping criteria (verifier rejects without naming the missing piece) and poor reformulation (rewording vs. targeting a gap). Stop-RAG (arXiv 2510.14337) shows value-based learned stopping beats both fixed-iteration and prompt-based stopping; QPP-based work (arXiv 2507.10411) asks whether predicted query performance tells the agent it is "on the right track."

**Breadth vs. depth should be an explicit, task-set parameter.** DeepResearch^Eco exposes user-controllable depth and breadth knobs and recommends high-breadth/moderate-depth for exploratory questions, high-depth/low-breadth for specific ones. DeepEvidence splits this into separate breadth-first and depth-first subagents under a budget-tracking orchestrator. The factual-lookup vs. synthesis divide is stark in practice: "AI search" reads 1–5 sources in <30s; deep research reads 50–200+ pages over 5–30min — different machinery for different jobs. Caveat: even strong synthesis agents have severe recall bottlenecks (best agent o3 hit only ~21% recall of expert-cited papers per arXiv 2601.12369), so breadth is hard to achieve, not just costly.

**Coding tasks invert the "more is better" intuition.** Cursor found semantic + grep hybrid beats either alone (+12.5% avg QA accuracy on large codebases), but the wider lesson is precision over volume: returning large code blocks "inflates the conversation" and the agent re-reads redundant material; Semble reports 566 vs. 45,692 tokens vs. ripgrep+read for the same answer. Coding wants the *right few* chunks, exact-match-first, with verbosity control — not big k.

## Actionable implications for librarian usage experiments

1. **Make the experiment manipulate the tool description, not just behaviour.** Run task-conditioned trials with description variants that encode per-task strategy ("for definitional/known-item lookups, one query at k≈8 and stop; for synthesis, decompose and issue 2–3 queries up to k≈20"). Measure activation/strategy shift, mirroring Microsoft's data-driven description loop. This is the cheapest high-leverage lever and should be its own arm.

2. **Sweep k per task family, not globally.** Replicate the RAGGED-style curve separately for known-item, definitional, multi-hop-synthesis, maths, and coding queries. Hypothesis: optimum k rises monotonically with hop-count/synthesis-load (lookup k≈5–8, synthesis k≈20, multi-hop possibly higher), confirming whether Round-1's single k=20 should be replaced by a task-conditioned default.

3. **Add an iteration-budget arm with a ~3-cycle cap and an explicit stopping contract.** Test fixed-cap vs. confidence-label-gated stopping (extend the existing abstention contract: "stop when the confidence label is high OR after 3 queries, then answer or abstain"). Log retrieval-thrash signatures (repeated near-identical queries, oscillation) as a failure metric, and measure token cost vs. answer quality to locate the synthesis value point.

4. **Add a `--concise/--detailed` (verbosity) parameter and a per-task default.** Anthropic's `response_format` pattern lets coding/known-item tasks pull breadcrumbs-only or top-3 chunks while synthesis pulls full chunks. Measure token-to-quality ratio per task — expect coding/lookup to win big from concise, synthesis to need detailed.

5. **Expose breadth vs. depth explicitly and route by task complexity.** Treat "number of distinct queries" (breadth) and "k per query" (depth) as orthogonal knobs the assistant sets from task type, per DeepResearch^Eco. Include a query-complexity router as a baseline: trivial/known-item → single shallow query; synthesis → decomposed multi-query. Report cost separately so the 3–10x iteration tax is visible.

6. **Instrument refinement *quality*, not just count.** Distinguish gap-targeted reformulation from mere rewording (the documented thrash cause). A QPP- or confidence-trajectory metric (is each successive query's confidence label improving?) gives an objective "on the right track" signal and a candidate learned stopping rule à la Stop-RAG.

## Sources

- Anthropic — Writing effective tools for AI agents; Effective context engineering for AI agents (50-item default, response_format verbosity, descriptions-as-context).
- Microsoft Engineering — How we built the Microsoft Learn MCP Server (data-driven description iteration, activation-rate sensitivity).
- arXiv 2602.14878 — MCP tool descriptions "smells" (description quality → agent efficiency).
- RubyMine/JetBrains blog; The New Stack "15 Best Practices for MCP Servers"; Docker MCP best practices (pagination offset vs cursor, educational errors, build-for-the-agent).
- arXiv 2403.09040 — RAGGED (k=1..50 sweep; monotone vs early-peak model families; variation before k=30).
- arXiv 2512.14313 — Dynamic Context Selection (recall↑/precision↓, accuracy below recall, distractors, lost-in-the-middle, dynamic-k, rerankers).
- Chemistry RAG (arXiv 2505.07671); EvoWiki (arXiv 2412.13582); MuSiQue/HotpotQA k-sweeps — inverted-U, multi-hop k-dependence.
- arXiv 2510.14337 — Stop-RAG (value-based adaptive stopping); arXiv 2507.10411 — QPP and agentic search behaviour.
- Towards Data Science — Agentic RAG failure modes (thrash/tool-storms/context-bloat); Agentic RAG vs Classic RAG (control loop).
- DeepResearch^Eco (bioRxiv 2025.07.14.664755); DeepEvidence (arXiv 2601.11560) — breadth/depth knobs and subagents.
- arXiv 2601.12369 — synthesis gap, ~21% expert-citation recall ceiling.
- Cursor (cursor.com/blog/semsearch); Semble; LightOn ColGrep; "Is Grep All You Need?" — code retrieval: hybrid, precision over volume, token cost.
