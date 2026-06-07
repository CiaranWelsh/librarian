# Task-Conditioned RAG Usage: Note-Taking & Strategy Synthesis

Scope: how an AI assistant should use the librarian (top-k chunk retriever over SWE/physics
corpora) *differently per task type*, and what note-taking / structured-memo discipline
turns multi-chunk retrieval into reliable synthesis. Round-1 single-query mechanics
(verbatim > rewrite, k=20 best / k=8 value, quote-first, abstention contract) are taken as given.

## 1. Note-taking / structured memos are the synthesis layer, not a nicety

**Chain-of-Note (CoN)** is the most directly relevant prior. The model writes a *reading note per
retrieved chunk* before answering, classifying each as (A) directly answers, (B) contextually useful
+ parametric knowledge, or (C) irrelevant -> abstain. CoN raised open-domain QA performance on noisy
retrieval and improved handling of out-of-scope questions, and its case studies show the gain comes
specifically from *integrating across multiple chunks* rather than seizing the first surface match
(Tencent AI Lab, arXiv:2311.09210). Our abstention contract is essentially CoN's Type-C made into a
hard rule; the untapped piece is the *per-chunk A/B/C labelling step* as a forcing function before
synthesis. Note: original CoN fine-tunes LLaMA-2-7B for note generation, but the structure works as a
pure prompt scaffold on a strong model.

**Claim-source matrices** are the literature-synthesis specialisation. FactReview uses a
schema-constrained extractor that emits, per claim, a typed JSON record: claim text, type, scope,
source span, linked evidence, and — critically — *decomposes a claim into narrower subclaims when it
spans multiple datasets/metrics*, so a supported local result is never laundered into an overbroad
global statement (arXiv:2604.04074). Practitioner multi-LLM literature-matrix workflows add an
explicit **conflict-detection pass** ("list every disagreement; flag claims in one extraction but not
the other") and a draft rule that every sentence must be inline-sourced or flagged. Multi-agent survey
systems that enforce this structure report large quality deltas (Agentic AutoSurvey 8.18 vs 4.77/10;
PRISMA copilot 84% agreement with expert scores) — evidence that the *structured intermediate
artifact*, not just more retrieval, drives synthesis quality.

**Working-memory discipline** generalises this to all long tasks. Agents that succeed at long-horizon
work externalise reasoning into scratchpads/notes; "active context compression" found *passive*
prompting yielded only ~6% savings with accuracy loss, while *aggressive* instructions to compress
every 10-15 steps gave 22.7% savings and prevented stale-context pollution. A-MEM (Zettelkasten atomic
notes + linking) and HiAgent (subgoal memory chunks: 2x success, 42 vs 21) show structure beats flat
dumping. Memex(RL) keeps a compact in-context index but dereferences full-fidelity chunks on demand —
the right pattern for a citation-bearing corpus where the verbatim span is load-bearing.

## 2. Strategy varies sharply by task type

- **Literature synthesis** — breadth-first. Over-decompose into parallel sub-queries; coverage, not
  hop-count accuracy, is the goal (decomposition gave +4.4 Hits@4, reached 87.2% Hits@10; yields 9-15
  distinct fragments vs 3-5 paraphrases of one source). Stop on *coverage saturation* (new queries
  return already-seen chunks). Output is a claim-source matrix.
- **Maths / pure reasoning** — retrieve sparingly. Standard retrieval over generic corpora gives weak,
  sometimes negative gains for maths; benefit accrues to weaker models or when the *corpus* holds
  worked solutions (RAG+ retrieves application examples/reasoning chains, not just facts). For us:
  retrieve a definition/theorem once, then reason; don't re-query mid-derivation.
- **Science / factual** — retrieval's strongest case; depth-via-sequential hops for multi-hop facts.
  Confidence/recall threshold to beat the base model varies widely (0.2-1.0 across datasets), and RAG
  adds little for well-known facts unless retrieval quality is high — so route obvious facts past
  retrieval (Adaptive-RAG / FAIR-RAG OBVIOUS/SMALL/LARGE/REASONING routing).
- **Coding** — task-aware. Prompting/reasoning scaffolds help code *substantially* but QA *little*,
  because QA queries are self-explanatory while code needs requirement->API translation. Structural
  retrieval (grep/AST) often beats semantic similarity for code navigation; reserve semantic RAG for
  docs/commits/issues. Even with correct APIs in context, models sometimes ignore them — retrieval
  alone is insufficient for code (arXiv:2511.05302, 2510.20609, 2411.19463).
- **Writing** — retrieve for grounding facts/quotes only; over-retrieval injects voice-distorting
  boilerplate. Quote-first + inline-source-or-flag rule applies.
- **Learning** — retrieve canonical definitions to *verify the learner*, not to lead; minimal depth,
  authoritative single chunk preferred over breadth.

## 3. Stopping rules and search budgets

When-to-stop must be self-evaluated, and naive "ask the LLM if done" underperforms. Stop-RAG casts
iterative RAG as a finite-horizon MDP with a learned value controller and beats both fixed-iteration
and prompt-based stopping (arXiv:2510.14337). Production consensus: hard tripwires (max ~3 retrieval
iterations, 10-15 tool calls, token + wall-clock ceilings) because reference agentic-RAG loops
genuinely run away; route simple queries away from the loop entirely; surface remaining budget to the
model so it reasons about spend (BCAS). Adaptive-RAG's three-way complexity classifier (no / single /
multi retrieval) is the cheap version of task-conditioning.

## 4. Actionable implications for librarian experiments

1. **Test the per-chunk A/B/C note step as an independent variable.** Compare quote-first alone vs
   quote-first + CoN-style per-chunk relevance labelling on synthesis accuracy and abstention quality.
2. **Add a claim-source matrix mode for literature synthesis** (claim | source_id | quote | scope |
   confidence) with a mandatory conflict-detection pass; measure unsupported-sentence rate vs free-form.
3. **Build a task-type router** (synthesis / maths / science / coding / writing / learning) selecting
   k, breadth-vs-depth, and a stopping rule per type; measure quality and chunk-budget vs a fixed policy.
4. **Define per-task stopping rules and log them:** coverage-saturation for synthesis; one-shot for
   maths/learning; hop-chain-with-cap for multi-hop science; complexity-router + hard tripwire (~3
   iters) overall. Test against fixed-k and unbounded baselines.
5. **Route obvious / parametric-known queries past retrieval** and confirm the abstention contract
   degrades gracefully when the corpus genuinely lacks the answer (the BCAS "low-signal" failure mode).
6. **Treat the scratchpad as a measured artifact:** require an indexed-note summary (Memex-style) that
   dereferences source_ids on demand, and check whether aggressive note compression preserves the
   load-bearing verbatim spans cited in the final answer.

### Sources
arXiv:2311.09210 (Chain-of-Note); 2604.04074 (FactReview); 2509.18661 (Agentic AutoSurvey);
2509.17240 (PRISMA SLR multi-agent); 2510.14337 (Stop-RAG); 2603.08877 (BCAS budget-constrained);
2403.14403 (Adaptive-RAG); 2510.22344 (FAIR-RAG); 2406.19215 (SeaKR); 2507.00355 (Question
Decomposition); 2510.02827 (StepChain GraphRAG); 2506.11555 (RAG+); 2511.05302 (When More Retrieval
Hurts: code review); 2510.20609 (Practical Code RAG at Scale); 2411.19463 (RAG design decisions);
2510.00615 (ACON); 2601.07190 (Active Context Compression); 2603.04257 (Memex(RL));
2502.12110 (A-MEM); 2408.09559 (HiAgent).
