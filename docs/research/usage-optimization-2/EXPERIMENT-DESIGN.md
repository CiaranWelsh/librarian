# EXPERIMENT DESIGN: Round-2 Task-Conditioned Librarian Usage

*This is a build-ready specification. It is consumed directly to write the harness
`experiments/usage_experiments_v2.py` on turbo. It inherits Round-1 mechanics as fixed
(verbatim query, k=20 retrieve / k=8 value point, quote-first generation, abstention contract
that drove hallucination 12%→0%) and tests the orthogonal Round-2 question: **does conditioning
the retrieval strategy on the task type beat a single fixed pipeline?***

Companion documents (read first if extending this): `SYNTHESIS-taxonomy.md` (what strategy per
task — T1–T12), `SYNTHESIS-metrics.md` (how to score — metric panel), and the Round-1 harness
`experiments/usage_experiments.py` (building blocks reused verbatim where possible).

---

## 0. Headline design decisions (and why)

1. **Five task types, not twelve.** The taxonomy enumerates T1–T12; we collapse to the five that
   (a) map to the user's real work — software engineering, particle physics, maths/theory,
   literature synthesis, learning — and (b) have **distinct predicted optimal strategies** so the
   one-variable comparison is informative. Picked: **LOOKUP** (T1/T6, lookup end of every dial),
   **MULTIHOP** (T4 science/factual QA, the depth-per-hop case), **SYNTHESIS** (T2 literature
   synthesis, the breadth end), **MATHS** (T3 derivation, the predicted retrieve-or-not / HyDE
   reversal), **LEARNING** (T8, the predicted quote-first *inversion*). T5/T7/T9/T10/T11/T12 are
   either sub-shapes of these on our corpus or need artifacts we cannot generate unattended; they
   are explicitly out of scope and listed in §7.

2. **One variable per comparison, and it is always the *strategy*, not the model.** Generator is
   pinned `gpt-4o-mini`, judge `gpt-4o`, temperature 0, exactly as Round 1. Within a task type the
   arms differ only in *how the librarian is driven* (single-shot vs planned multi-search vs
   iterative-refine vs deep-read). The k, query-form and quote-first generation settled in Round 1
   are held at their winning values inside every arm unless the arm *is* the variable under test
   (MATHS HyDE arm, LOOKUP confidence-gate arm).

3. **Closed-book and single-shot controls live in every task type.** Per the metrics doc §3.1 we
   need the net-help baseline (closed-book) and the Round-1 winner (single-shot k=20 quote-first)
   in each task type, or no arm's lift is interpretable. These two are arms `S0` and `S1` in every
   block.

4. **Multi-step strategies are scripted loops, never agents.** "Iterative refine" = a fixed
   Python `for` loop of ≤3 rounds that re-queries with a templated reformulation and a
   confidence/coverage stop check; "planned multi-search" = one decomposition LLM call producing N
   sub-queries, then N searches + RRF union. No tool-choosing agent, no open-ended ReAct. This
   keeps every arm deterministic, debuggable, and inside the call budget.

5. **`source_id` is NOT a reliable on-disk path.** Probing the daemon shows `software` chunks point
   at heterogeneous/stale paths (`/data/books-curated/ingest-ebook/…`, `/tmp/diag-ingest/…`,
   `.epub`). Only `particle-physics` is clean (`/data/corpus/markdown/particle-physics/`).
   Consequences baked into the design: (a) the deep-read / breadcrumb-expand arm uses the Round-1
   `expand_from_file` probe-match trick **and falls back to the chunk text when the file is
   missing** — and we *log the fallback rate* as a first-class number; (b) reference-anchored
   question generation reads files from the markdown trees we *can* enumerate, and stores the
   reference text inline in the question record (never re-reads at score time). The disk layout is
   enumerated at harness startup; if a markdown tree is unreadable that task type's corpus-generated
   questions fall back to golden/hand-seeded items and the run logs it rather than crashing.

6. **Bounded cost is enforced by construction, not hope.** §6 carries a line-item call budget that
   sums to **≈2,180 LLM calls** with headroom under the 2,500 cap. Question generation is cached to
   a JSON file (Round-1 pattern) so re-runs cost zero generation calls. Search calls are free
   (local daemon) and uncounted against the LLM budget but are logged for the efficiency axis.

---

## 1. Task types, corpus, and which collection each draws from

| Code | Task type | Taxonomy | Collection(s) | Predicted winning arm | The Round-2 question it answers |
|---|---|---|---|---|---|
| **LOOKUP** | Known-item / API-or-fact lookup | T1, T6 | software (+ pp facts) | S1 single-shot k=8, or **confidence-gated skip** | Does confidence-gating retrieval prevent the common-fact regression without losing the 0% halluc floor? |
| **MULTIHOP** | Multi-hop factual QA | T4 | particle-physics (cross-paper) | **decompose-then-retrieve** (planned) | Does plan-first decomposition beat single-shot on 2-hop physics questions, and does per-hop recall expose error propagation? |
| **SYNTHESIS** | Literature synthesis / compare-and-contrast | T2 | both (cross-domain incl.) | **planned fan-out per sub-question** | Does breadth-first multi-search raise nugget coverage + distinct-source count over a single high-k pull, at what token cost? |
| **MATHS** | Derivation / theory grounding | T3 | software (rust-theory) + pp | **closed-book or retrieve-the-consumed-fact only**; HyDE-target query | Does declining to retrieve correlate with correctness here, and does the math HyDE exception (draft-target query) beat verbatim — the one predicted Round-1 reversal? |
| **LEARNING** | Explain-to-learn / tutoring | T8 | software | **quote-then-elaborate** (inverts quote-first) | Does transform-the-source (elaborated explanation) beat verbatim quote-first on learner-rated helpfulness without raising hallucination? |

Corpus paths on turbo (enumerated at startup; the harness records what it actually found):
```
SW   = /data/corpus/markdown/software            # may be a SUBSET of indexed software
PP   = /data/corpus/markdown/particle-physics     # clean, authoritative
RUST = /data/books/software/rust-theory/texts     # rust-theory subset (for MATHS theory items)
```
Daemon: `http://100.127.138.48:6700`, `POST /v1/search {collection, query, limit}` →
`{hits:[{score, source_id, content_type, chunk_index, text}], confidence:{value,label,top_score,margin,score_spread,fragment_rate}}`.
`text` is breadcrumb-prefixed: `Book > Chapter > Section\n\n<body>`.

---

## 2. Test-task (question) generation — per task type

**Counts:** 8 questions per task type × 5 = **40 task questions**, plus a shared **5 absent-topic
controls** (reused from Round-1, certainly-not-in-corpus, correct behaviour = abstain) =
**45 records total**. All cached to `usage2_questions.json`; generation is the only place
generation-side LLM calls are spent on question construction (≈ 40 calls, one-time, cached).

Each question record schema (superset of Round-1):
```json
{"q": "...", "task": "LOOKUP|MULTIHOP|SYNTHESIS|MATHS|LEARNING|ABSENT",
 "ref": "verbatim source excerpt(s) that ground the answer (stored inline)",
 "src": "filename(s) the ref came from",
 "nuggets": ["atomic fact 1", "..."],          // SYNTHESIS/MULTIHOP only; LLM-extracted, cached
 "keywords": ["..."],                            // LOOKUP/MATHS fallback when no ref
 "expect_abstain": false}
```

Question generators are **reference-anchored** (write the question *from* a real excerpt so a
ground-truth reference always exists). Generation model = `gpt-4o-mini`, temp 0.

### 2.1 LOOKUP (8) — single-passage, localizable
Sample 8 software `.md` files spread across the directory (Round-1 stride trick:
`files[::len//8]`). From each, take the 600–2600 char excerpt and prompt:
> *"Write ONE specific, closed-form technical question whose complete answer appears in the
> passage below — a definition, a named principle, a single API/method, or a single threshold.
> The answer must be one or two sentences. Reply with only the question."*

Store `ref = excerpt[:1200]`, `keywords = []`. 2 of the 8 are deliberately seeded as
**well-known facts** (e.g. "what does the Single Responsibility Principle state?") to probe the
confidence-gate regression hypothesis (T6/DAG++), flagged `"wellknown": true`.

### 2.2 MULTIHOP (8) — two-paper particle-physics
Pick 8 *disjoint pairs* of particle-physics files. Per pair, prompt with the two excerpts:
> *"Two excerpts from different detector-physics papers follow. Write ONE question that can only
> be answered by combining a fact from BOTH excerpts (e.g. compare a value in A with a mechanism
> in B). Reply with only the question."*

Store `ref = exA[:700] + "\n---\n" + exB[:700]`, `src = "fileA|fileB"`. Then a **second** cached
call extracts `nuggets` (the 2–4 atomic facts the answer must contain) for per-hop recall scoring:
> *"List, one per line, the 2–4 atomic facts a correct answer to this question must state. Use the
> two excerpts as ground truth."*

### 2.3 SYNTHESIS (8) — compare/survey, breadth-demanding
Mix: 4 software (e.g. "compare how two of these books frame coupling vs cohesion"), 2
particle-physics, 2 **cross-domain** (software-for-physics, the §6-flagged collection-routing
cell — e.g. "what software-architecture principles apply to a real-time detector data pipeline?").
Seeded from 2–3 excerpts each. Prompt:
> *"Write ONE open-ended question that requires synthesising information from MULTIPLE sources to
> answer well (a comparison, a survey of approaches, or trade-offs). Reply with only the question."*

Cache `nuggets` = the union of vital atomic facts across the seed excerpts (LLM-extracted,
labelled `vital`/`okay` — AutoNuggetizer-lite); `ref` = concatenated seed excerpts. These power
**strict vital-nugget recall** and **distinct-source count**.

### 2.4 MATHS (8) — derivation / theory
4 from rust-theory texts (type theory, lambda calculus, ownership model), 4 from
particle-physics (a detector formula / derivation: ToT-to-energy, time-walk correction, cluster
centroid). Two sub-kinds, 4 each, flagged `"kind": "fact"|"derivation"`:
- **fact**: "state the formula / definition X" (retrieval *should* help — it is the consumed fact).
- **derivation**: "derive / show why Y follows" (retrieval predicted to *hurt*; closed-book should
  win — this is the retrieve-or-not correlation test).

Prompt for fact items quotes the formula excerpt; derivation items prompt:
> *"Write ONE question asking the reader to derive or explain *why* a result holds, using the
> passage as ground truth for the final result. Reply with only the question."*
Store `ref`, `keywords` (the canonical names/symbols), and `gold_final` (the final result string
for closed-form check where one exists).

### 2.5 LEARNING (8) — explain-to-learn
8 software concepts at two levels (4 `novice`, 4 `expert`, flagged). Seeded from a tutorial-style
excerpt. Prompt:
> *"Write ONE question a learner would ask to UNDERSTAND this concept (not just look it up) —
> 'help me understand X', 'why does Y work this way'. Reply with only the question."*
Store `ref` and a cached `nuggets` list of the must-convey teaching points (used by the
level-appropriateness + helpfulness judge).

### 2.6 ABSENT (5) — shared abstention controls
Reuse Round-1 `ABSENT` list verbatim (Kubernetes 1.31, Swift 6 concurrency, CrowdStrike 2024
postmortem, Bun S3 client, Datadog LLM pricing). `expect_abstain = true`. Run under every task
type's generation config? No — run **once** as a shared block under each *strategy family* so the
abstention floor is checked per strategy, not per task (see §5 run matrix).

---

## 3. Strategy arms — per task type (the one-variable comparisons)

Every arm shares the Round-1 generation contract unless noted. Notation: `S0..Sn` per task. All
arms reuse Round-1 building blocks (`ctx_flat`, `ctx_form`, `assemble`, `run_arm`,
`expand_from_file`, RRF `ctx_fanout`) — listed against each so the harness is a thin wiring layer.

### Shared controls (in EVERY task type)
- **S0 — closed-book.** `builder=None`, `GEN_SYS_CLOSED`. Net-help / net-harm baseline. (Round-1
  `A0`.) *This is the synergy/augmentation control the metrics doc §3.2 requires.*
- **S1 — single-shot k=8 quote-first verbatim.** Round-1 winner recipe (k=8 value point, verbatim
  query, `QUOTE_SYS`). The fixed-pipeline champion every task-conditioned arm must beat. (Round-1
  `A1_k8` + `A6` generation.)

### LOOKUP (4 arms: S0, S1, S2, S3) — *one variable: retrieval gating*
- **S2 — single-shot k=20 quote-first.** Tests whether more breadth helps or *hurts* a localizable
  fact (predicted: ties or slightly hurts → inverted-U). Variable vs S1 = k only.
- **S3 — confidence-gated skip (DAG++).** Run the verbatim search; if `confidence.label == "strong"`
  *and* the question is flagged `wellknown`, **answer closed-book** (skip the chunks); else behave
  as S1. Logs `skip_rate` and `context_dominance_flips` (cases where adding chunks changed a
  correct closed-book answer to wrong — measured by also running S0 on the same question).
  Variable vs S1 = the gate. *Answers the headline LOOKUP question.*

### MULTIHOP (4 arms: S0, S1, S2, S3) — *one variable: planning*
- **S2 — planned decompose-then-retrieve.** One decomposition call → 2 sub-questions (one per hop)
  → search each at k=8 → union (dedup by `(source_id, chunk_index)`) → answer once. Variable vs S1
  = the decomposition step. (Reuses `ctx_fanout` machinery with a 2-way, hop-targeted decomposition
  prompt instead of the generic 3-way.)
- **S3 — iterative refine (≤2 rounds).** Search hop-1 verbatim; LLM extracts "what fact is still
  missing for hop 2?"; second search on that gap; answer. Stop early if `confidence.label` is
  strong after round 1. Variable vs S2 = sequential-conditioned-on-result vs parallel-decompose.
  *Tests decompose-then-iterate vs naive (taxonomy T4 open question).*

### SYNTHESIS (4 arms: S0, S1, S2, S3) — *one variable: breadth mechanism*
- **S2 — single-shot k=20 quote-first.** The "one wide pull" baseline (breadth via k, not via
  multiple queries).
- **S3 — planned fan-out per sub-question.** One outline call → 3 perspective sub-queries (STORM
  multi-perspective) → search each at k=8 → RRF union to ~12 → quote-first answer with a
  claim-source instruction. Variable vs S2 = fan-out vs single high-k (breadth source). *Answers
  the headline SYNTHESIS question; this is also the "poor-man's global" test from taxonomy §2.*

### MATHS (4 arms: S0, S1, S2, S3) — *one variable: query form / retrieve-or-not*
- **S1 — single-shot k=8 verbatim quote-first** (as everywhere).
- **S2 — HyDE-target query.** Draft a one-sentence *target result statement* (Round-1 `reform_hyde`)
  and retrieve against it, k=8, quote-first. Variable vs S1 = query form. *The one predicted
  Round-1 reversal (taxonomy T3 / experiment #8).*
- **S3 — tiny-k=3 verbatim.** Hard-negative-minimisation for derivations. Variable vs S1 = k only.
- *Retrieve-or-not* is tested across S0 vs {S1,S2,S3} stratified by `kind`: prediction is S0 wins
  on `derivation`, retrieval wins on `fact`. The **decline-to-retrieve↔correctness correlation** is
  computed from this stratification, not a separate arm.

### LEARNING (4 arms: S0, S1, S2, S3) — *one variable: generation transform*
- **S1 — single-shot k=8 quote-first** (verbatim grounding, the contract LEARNING is predicted to
  *invert*).
- **S2 — quote-then-elaborate.** Same retrieval (k=8); generation system prompt: *"First quote the
  relevant passage [n]; THEN explain it in your own words at a level appropriate to the question,
  with one concrete example. Cite [n]. If the passages don't cover it, say so."* Variable vs S1 =
  generation transform only. *Answers the headline LEARNING question (Levonian inversion).*
- **S3 — Socratic-withhold (novice items only; expert items skip to S2).** Retrieve, then generate a
  hint + one guiding question *without stating the answer*; measure **answer-leakage rate** (does it
  reveal the answer anyway — 57% baseline to beat). Variable vs S2 = withhold vs reveal.

**Arm count:** 5 task types × 4 arms = 20 arms, **minus 5** shared-control de-duplication (S0 and S1
are defined once and reused; absent controls run once per strategy family) — see §6 for the actual
call accounting.

---

## 4. Judge rubrics (pinned prompts, model = gpt-4o unless noted)

Metrics follow `SYNTHESIS-metrics.md`: **never one number** — correctness, attribution, coverage,
efficiency, abstention scored separately. Hallucination Ratio = 0 is a hard pass/fail gate reported
separately, never folded into the quality score.

Reused verbatim from Round 1 (do not re-derive): `P_CTX` (context precision, cheap judge `gpt-4o-mini`),
`P_FAITH` (faithfulness 0/1/2), `P_QUAL_REF` (quality vs reference 0/1/2), `P_QUAL` (quality vs
keywords). Round-1 abstention/hallucination bookkeeping in `run_arm` is reused unchanged. New
rubrics below.

### 4.1 Attribution gate — verbatim-quote check (programmatic, NO LLM)
Per the metrics doc: **ban LLM-as-judge for attribution.** For every `[n]` citation in the answer,
check the quoted span is a **verbatim subsequence** of chunk `n`'s `text` (normalised whitespace,
case-insensitive, ≥8-word span match). Report `quote_verbatim_rate`. A cited chunk that does not
contain the claimed quote is an **attribution failure** logged separately from `halluc`. (This is
the MARIN/quote-first existence check folded in — cheap, deterministic, comparable across runs.)

### 4.2 Nugget coverage (SYNTHESIS, MULTIHOP) — strict vital recall
For each cached `nugget`, one judge call (gpt-4o):
> *"NUGGET (an atomic fact): {nugget}\nANSWER:\n{answer}\nDoes the answer state or clearly imply
> this nugget? Reply 1 for yes, 0 for no. Only the digit."*
`vital_recall = supported_vital / total_vital`. Also report `distinct_sources` = count of unique
breadcrumb-root books cited in the answer (dedup `book_of(source_id)`), and `citation_density` =
`[\d+]` markers per 100 words (both programmatic). For MULTIHOP these nuggets *are* the per-hop
facts → `hop_recall = supported / total`.

### 4.3 Maths step-correctness (MATHS) — earliest-error, gpt-4o
Final-answer matching is banned (ProcessBench: >50% right answers via wrong reasoning). Rubric:
> *"QUESTION:\n{q}\nGROUND-TRUTH RESULT:\n{ref}\nANSWER:\n{answer}\nScore the reasoning: 2 = result
> correct AND each step valid; 1 = result correct but a step is unjustified or hand-waved; 0 = a
> step is wrong or the result is wrong. Only the digit."*
The `1` bucket is the metrics-doc **attributed-but-wrong / right-but-unjustified** cell; report its
rate separately. Where `gold_final` exists also log programmatic final-answer match as a secondary.

### 4.4 Learning helpfulness + level-appropriateness (LEARNING) — gpt-4o
Two digits, two calls (kept separate so they don't trade off):
> *(helpfulness)* *"A {novice|expert} learner asked: {q}\nRESPONSE:\n{answer}\nRate how well this
> helps them UNDERSTAND (not just look up) the concept: 2 = clear, builds intuition, well-pitched;
> 1 = correct but a bare definition / mis-pitched level; 0 = unhelpful or wrong. Only the digit."*
> *(level-appropriateness)* *"…is the explanation pitched correctly for a {level} learner (novice =
> needs worked example/intuition; expert = wants precise/concise)? 2 yes, 1 partly, 0 no."*
For S3 Socratic items, the **answer-leakage** check (programmatic + 1 judge call): *"Does the
response state the final answer outright rather than guiding the learner to it? 1 leak, 0 no."*

### 4.5 Judge-bias guards (apply to all content-quality judges)
- Generator `gpt-4o-mini` ≠ judge `gpt-4o` (cross-family-ish; self-preference mitigation, Round-1).
- Temperature 0, `max_tokens` minimal (2 for digit judges).
- For any **pairwise** preference (none required by the arms above, but if a follow-up adds one):
  randomize answer order, run both orders, average (>30-pt position-bias swings documented).
- Treat ρ≈0.5 as the judge ceiling; **do not over-interpret sub-0.2 quality-point gaps** — report
  as ties, exactly as Round 1 ("overlapping CIs = tie").

---

## 5. Run matrix and statistics

### 5.1 Matrix
For each task type, run its 4 arms over that task's 8 questions **plus** the 5 absent controls
(absent controls scored only on abstention/hallucination, not quality). The absent block is run
once per arm *family* label so each strategy's abstention floor is verified:

| Task | Questions scored on quality | + absent controls | Arms |
|---|---|---|---|
| LOOKUP | 8 | 5 | S0, S1, S2, S3 |
| MULTIHOP | 8 | 5 | S0, S1, S2, S3 |
| SYNTHESIS | 8 | 5 | S0, S1, S2, S3 |
| MATHS | 8 | 5 | S0, S1, S2, S3 |
| LEARNING | 8 | 5 | S0, S1, S2, S3 |

S0 (closed-book) and S1 (single-shot k=8 quote-first) are *defined once* and their results reused
across task blocks for the absent controls (the absent questions are identical), so absent controls
are scored once per distinct strategy, not 5×.

### 5.2 Oracle-router post-hoc analysis (the central Round-2 deliverable)
No separate arm. After all arms run, compute the **oracle-routed** quality/cost point: for each
task type pick the arm with the best (quality, then faith, then −tokens) and report the
task-conditioned ceiling vs the single-fixed-pipeline (S1-everywhere) point. Then run a **cheap
prompt-classifier** (one gpt-4o-mini call per question: *"Classify this request as LOOKUP /
MULTIHOP / SYNTHESIS / MATHS / LEARNING. One word."*) over all 40 questions and report
**router accuracy** and the **oracle gap** (Adaptive-RAG's lesson: the router, not the arms, is the
bottleneck). This is ≈40 extra calls and is the headline result.

### 5.3 Statistics
- **Bootstrap 95% CIs** on the quality mean, 800 resamples over questions (Round-1 method, reused
  `run_arm` bootstrap). Overlapping CIs ⇒ reported as a tie. With n=8 per cell CIs are wide *by
  design* — this is exploratory; we report effect direction + CI, never a significance claim.
- **One variable per comparison** is guaranteed by arm construction (§3): each Sn differs from its
  named comparator in exactly one knob.
- **Stratification reported** where the hypothesis is conditional: MATHS by `kind`
  (fact/derivation), LOOKUP by `wellknown`, LEARNING by `level`, SYNTHESIS by domain
  (software/pp/cross). These are descriptive splits, not new arms.
- **Phrasing robustness (optional, budget-permitting):** re-run S1 on 2 paraphrases of 5 questions
  to confirm verbatim-query robustness extends to routing (metrics §3.4). Costed as optional in §6.

---

## 6. Cost accounting (hard cap ≤ 2,500 LLM calls)

Search calls hit the local daemon and are free (logged for the efficiency axis, not counted here).
LLM = OpenAI calls (gen + judge). Per scored non-absent question, judging cost is: 1 quality + 1
faith + ≤2 nugget/level + up-to-8 cheap ctx-precision (gpt-4o-mini). We budget conservatively.

| Block | Calls | Notes |
|---|---|---|
| Question generation (cached, one-time) | ~55 | 40 questions + ~15 nugget/level extraction calls |
| Per-arm GENERATION calls | 5 tasks × 4 arms × 8 q = 160 | + multi-step arms add 1 decomp/refine call each: ~4 arms × 8 = +32 → **~192** |
| Absent-control generation | 4 distinct strategies × 5 = 20 | abstention check only |
| Quality judge (gpt-4o) | 160 | 1 per scored answer |
| Faith judge (gpt-4o) | ~128 | 1 per grounded (non-closed-book) answer |
| Nugget recall (SYNTHESIS+MULTIHOP) | 16 q × ~3 nuggets × 4 arms ≈ 192 | the largest single line; cap nuggets at 3/q to bound it |
| Maths step + Learning level/leak judges | 8×4 + 8×4 + leak ≈ 70 | |
| Context-precision (cheap, gpt-4o-mini) | 160 answers × up to 8 = capped at **400** | cap to top-5 chunks → ~ 5×160 = 800; **reduce cap to top-3 → ~480**; budget 400 |
| Oracle router classifier | 40 | one per question |
| **Subtotal** | **~1,535** | |
| Optional phrasing-robustness (5 q × 2 × gen+judge) | ~30 | |
| **Headroom / retries / re-runs** | **~600** | |
| **TOTAL (planned)** | **≈ 2,180** | under the 2,500 cap |

Budget guards in the harness: a global `CALL_COUNT` counter incremented in `chat()`; if it crosses
2,400 the harness **stops launching new arms and writes a partial report** rather than overrunning.
Nugget count is capped at 3/question and ctx-precision at the top-3 chunks to hold the two largest
lines down. Cheap judge (gpt-4o-mini) calls are tracked separately and not charged against the
gpt-4o sub-budget (they dominate count but are ~20× cheaper).

---

## 7. Out of scope (and why) — keeps the series to one page of arms

- **T7 Debugging, T12 implementing-papers** — need a real error/codebase or executable check; cannot
  be scored unattended on a prose corpus without fabricating the artifact.
- **T9 fact-checking, T11 requirements, T10 ADR** — T9 is a relabelled MULTIHOP+attribution on our
  corpus; T11/T10 need Volere/ADR gold artifacts not generable at quality from chunks. The
  *attribution gate* (§4.1) and *conflict* idea carry their core metric into SYNTHESIS/MULTIHOP.
- **Human-in-the-loop synergy arms** — the metrics doc flags these as the real headline, but they
  need a human; out of scope for an unattended run. The closed-book (S0) arm preserves the
  AI-alone-from-memory leg so synergy is computable *later* if a human-alone arm is added.
- **Reranking / breadcrumb-snowball / reuse-across-turns** — Round-1 showed mini-listwise rerank
  *hurt*; breadcrumb expansion is partially blocked by the stale-`source_id` problem (§0.5); these
  are deferred to a Round-3 once the task-router result is in. The deep-read mechanism survives only
  inside no arm here (dropped) because its file-path dependency is unreliable on the software
  collection — noted as a known limitation, not silently included.

---

## 8. Report format (`usage2_report.txt`)

Mirror the Round-1 report so the two are directly comparable, but **grouped by task type** with the
per-task winner and the oracle-router summary on top.

```
ROUND-2 TASK-CONDITIONED USAGE REPORT  <date>
(40 task questions + 5 absent controls; gen=gpt-4o-mini judge=gpt-4o;
 CIs = 800x bootstrap over questions; overlapping CIs = tie; n=8/cell is exploratory)

== ORACLE ROUTER SUMMARY ==
single-fixed-pipeline (S1 everywhere): qual=__  faith=__  tokens=__
oracle task-routed (best arm per task): qual=__  faith=__  tokens=__   ΔQUAL=__
cheap-classifier router accuracy: __%   estimated routed qual=__   ORACLE GAP=__

== PER-TASK RESULTS ==
[LOOKUP]    arm  qual  95%CI  faith  halluc  quoteOK  absOK  prec  tokens  searches
  S0_closedbook ...
  S1_single_k8  ...
  S2_single_k20 ...
  S3_confgate   ...   skip_rate=__  ctx_dominance_flips=__
  WINNER: __    (S1 to beat: __)
[MULTIHOP]  ... hop_recall column
[SYNTHESIS] ... vital_recall, distinct_sources, cite_density columns
[MATHS]     ... split by kind: fact vs derivation; retrieve-vs-skip correlation=__
[LEARNING]  ... help, level, leak columns; split novice vs expert

== CROSS-TASK READING ==
- Hallucination floor (must be ~0 in every retrieval arm): __
- Where task-conditioning beat fixed S1 (by >1 non-overlapping CI): __
- Predicted reversals confirmed/refuted: MATHS-HyDE __, LEARNING-elaborate __, LOOKUP-confgate __
- Magnitudes do not transfer; directions do (taxonomy §6). Next: encode the per-task recipe + the
  router in the cw:librarian skill; re-validate top arms with Claude as generator.
```

Per-arm JSON dumped to `usage2_results/<task>_<arm>.json` (Round-1 structure: `{agg, rows}`) so any
cell is re-analysable without re-running. The efficiency axis (`searches`, `tokens`, derived TPC =
tokens-per-correct) is logged from the trajectory in every row, never rewarded directly (long
trajectories are a symptom, not a lever — metrics §E).

---

## 9. Build notes for the harness author

1. Start from `experiments/usage_experiments.py`; keep `post/search/chat/digit/book_of/hits_of/
   ctx_flat/ctx_form/assemble/expand_from_file/ctx_fanout` and the `run_arm` skeleton. Add a `task`
   field to each row and a `CALL_COUNT` global in `chat()`.
2. Generalise `run_arm(name, builder, gen_sys, judges)` to take a **list of judge functions** so a
   task can attach its extra rubric (nugget / step / level) without forking the function.
3. `build_questions()` becomes `build_questions_v2()` writing `usage2_questions.json`; it must
   **enumerate the corpus dirs at startup** and degrade gracefully (fall back to golden/seeded
   questions, log it) if `SW/PP/RUST` are missing or empty — never crash a 2-hour run on a missing
   tree.
4. The deep-read mechanism is **dropped** (stale `source_id`); breadcrumb roots for
   `distinct_sources` come from `book_of(source_id)` on the breadcrumb prefix in `text`, which is
   reliable even when the file path is not.
5. Determinism: `random.seed(31)`, temperature 0 everywhere, cached questions ⇒ a re-run reproduces
   the same arms. Search calls are idempotent against a static index.
6. Run via the existing `overnight_master_v*.sh` pattern (nohup, unattended) on turbo; write
   incremental per-arm JSON so a mid-run failure loses at most one arm.
