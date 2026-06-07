# SYNTHESIS: A Task Taxonomy for Librarian Usage

*Round-2 synthesis over the 50 numbered research notes in this folder. Round 1 settled the
**single-query mechanics** that every note below takes as given: verbatim query beats
rewrite/HyDE; k=20 retrieves best, k=8 is the value point; quote-first generation; an
abstention contract that drove hallucination 12%→0%. This document answers the orthogonal
Round-2 question — **how should an assistant drive the librarian differently per task type** —
by extracting the distinct task classes, the evidence-backed optimal strategy for each, the
strength of that evidence, and the open questions only an experiment can settle.*

---

## 0. The cross-cutting model (true for every task)

Before the per-task table, eight findings recur in nearly every note and form the spine of the
whole taxonomy:

1. **Retrieve-or-not is itself task-conditioned and is the first decision.** Retrieval helps
   knowledge-intensive/factual work and *hurts* pure reasoning. On maths (GSM8K/MATH-500) static
   retrieval underperforms plain CoT; the *decision to abstain from retrieving* correlates with
   correctness (08, 09, 19, 49). The librarian's abstention contract should extend to abstaining
   from *querying*, not just from answering.
2. **Optimal strategy = routing by complexity, not a fixed pipeline.** Adaptive-RAG's three lanes
   — no-retrieval / single-step / iterative-multi-step — is the single most-cited primitive
   (cited in 02, 03, 08, 09, 14, 16, 19, 24, 29, 30, 34, 35, 37, 39, 46). Its key lesson: **the
   router, not the strategy arms, is the bottleneck** (~54% three-way classifier accuracy; the
   oracle router is both cheaper *and* more accurate).
3. **The search-count curve has a knee, then turns *down*.** Peak at 3–7 sequential searches for
   genuinely multi-step work; 1–2 for lookups; gains flatten by ~5 and *degrade* past ~8 from
   context pollution (19, 24, 38). PaperQA2's measured **1.26 searches/question** on hard
   literature QA is the empirical anchor for "shallow is usually right" (10).
4. **More context is not free (the inverted-U / lost-in-the-middle).** Answer accuracy saturates
   well before retrieval recall; the model is misled by co-retrieved "hard negatives," and
   *stronger* retrievers surface more plausible-but-irrelevant distractors (08, 22, 28, 30, 47,
   49). Decouple **retrieve-k (broad, ≤20) from answer-k (narrow, ~3–8)** — PaperQA2 retrieves 30,
   answers on ~5.
5. **Stopping is a distinct, learnable control problem.** Confidence/sufficiency/value signals
   (FLARE, Self-RAG, Stop-RAG) beat both fixed-iteration and "ask the model if it's done."
   Stop-RAG frames it as a forward-looking MDP: *will another query help?* not *is current
   evidence sufficient?* (15, 20, 24, 26, 27, 29, 49).
6. **Generation pattern matters as much as retrieval.** Quote-first grounding, plan-then-retrieve
   (decompose-first closes the "compositionality gap"), and draft→verify→revise loops each win on
   the right task — but verify-revise only works with an *external* signal (the retrieved chunk);
   intrinsic self-correction is net-harmful (27, 37, 48).
7. **Multi-turn: reuse before re-retrieve.** The cheapest documented win is a prompt instruction
   to consult chunks already in context before re-querying; carried context goes stale on topic
   pivots and induces "retrieval laziness" (95%→25% follow-up probability as context fills) (25).
8. **Breadcrumbs are our only graph.** The corpus has section/chapter breadcrumbs but **no
   citation edges**. Breadcrumb traversal gives cheap *intra-document* depth (good for lookup,
   debugging, derivations); it does **not** give the *inter-cluster* breadth real snowballing buys
   for synthesis — that gap must be manufactured with embedding-space-diverse re-queries (28, 32, 36).

---

## 1. The task taxonomy (compact)

| # | Task type | Retrieve? | Searches | Breadth vs depth | Granularity | Stopping rule | Generation pattern | Evidence |
|---|---|---|---|---|---|---|---|---|
| T1 | **Known-item / fact lookup** (definition, threshold, single API) | Gate on confidence; skip if head-knowledge | 1 (hard-stop) | Depth, narrow | Tight chunk, no expansion | First above-confidence hit | Quote-first, verbatim query | **Strong** |
| T2 | **Literature synthesis / survey** | Always, aggressively | 3–7 iterative / fan-out | Breadth-first, multi-source | Chunk→expand to section per node | Coverage saturation + source-diversity floor | Plan/outline-first, per-section retrieve, claim-source matrix | **Strong** |
| T3 | **Maths / derivation / proof** | Mostly *no*; retrieve only the consumed fact | 0–1 per missing fact | Depth, tiny k (3–8) | Tight chunk; exact-name lookup | Stop once each needed fact is grounded; skip proof math | Retrieve fact → reason unaided; pair with code tool for arithmetic | **Strong** |
| T4 | **Science / factual QA** (incl. multi-hop) | Yes for tail facts | 1 single-hop; 2–4 multi-hop | Depth per hop | Chunk; parent on multi-hop | Hop-count matched to claim structure; sufficiency judge | Decompose-then-retrieve; FEVER-score (answer + right chunk) | **Strong** |
| T5 | **Grounded writing** (methods, related-work, docs) | Yes, per-section | 1 upfront + per-paragraph on low-confidence | Per-section breadth | Chunk for facts; section for flow | 2–3 per section, diminishing | FLARE-style forward trigger; or draft→RARR-verify; cite retrieved chunks only | **Strong** |
| T6 | **Coding / API lookup** | Conditional/confidence-gated (DAG++) | 1, structured-before-semantic | Depth, shallow, small k | Function/symbol-intact; version-pinned | First high-relevance signature pin; aggressive | Resolve-then-fetch; verify symbol appears verbatim | **Strong** |
| T7 | **Debugging / known-issue** | Yes (the InferFix/RAGFix slot) | Several, interactive | Narrow-and-deep, follow one thread | Chunk + breadcrumb traversal | Cap 2–3 hypothesis-test cycles; fresh-start re-retrieve on flat result | Error-as-query (verbatim for clean errors, rewrite for vague); ReAct | **Medium-strong** |
| T8 | **Learning / tutoring** | Yes, but *transform* output | 1–2; assemble a fade sequence | Depth on canonical source | Worked-example (novice) vs definition+prereq (expert) | Don't reveal answer (Socratic); recall-success ≥75% gate | **Inverts quote-first** — elaborate/Socratic, not verbatim; free-recall is the deliverable | **Medium** (pedagogy strong, RAG-mapping inferred) |
| T9 | **Fact-checking / verification** | Yes, per atomic claim | ≤5/claim (flat after 5); 1 vs curated source | Depth per claim; hops match complexity | Sentence/proposition unit | Set-level sufficiency (support/refute/insufficient) | Decompose→retrieve→NLI verdict; verify independently of draft | **Strong** |
| T10 | **Design decisions / ADR** | By sub-shape (see note) | 1 (pattern) / N-per-QA (trade-off) / few-exemplars (authoring) | Trade-off=breadth; authoring=narrow recency window | Chunk; few exemplars beat dump-all | Source-diversity for trade-off; single high-conf insufficient | Decompose by quality-attribute; verification pass > more searches | **Medium** |
| T11 | **Requirements grounding** (Volere/MRP) | Exception-driven; often none | 1 per artifact field; escalate on doubt | Mostly shallow; minority deep | Section-level (template-shaped) | Confirm one definition then stop; second query only on conflict | Citation-grounding rate is the metric; verbatim per field | **Medium-strong** |
| T12 | **Implementing papers (theory→code)** | Yes, repeated, state-dependent | N small targeted (one per equation) | Narrow/deep, many passes over one method | Equation/component chunk; notation queries | Every equation in data-flow has a grounded mapping | Iterative retrieve→synthesize→execute→repair; reproduce intermediate values | **Medium** |

**Global routing baseline (T0):** before anything, classify the request → set breadth (#queries)
up front from task type/complexity; set depth/stopping in-loop from the confidence label. Run all
arms on a labelled set to get the **oracle ceiling first**, then measure how close a cheap
prompt-classifier gets (the Adaptive-RAG protocol).

---

## 2. Per-task detail, evidence strength, and the experiment-only open questions

### T1 — Known-item / fact lookup *(Strong)*
- **Strategy:** One verbatim query, narrow k (k=8 value point), stop on first above-confidence hit.
  No breadcrumb expansion (Dense-X: fine units win on localizable facts). Gate retrieval on
  parametric confidence/rarity (Mallen popularity; DAG++) — skip for head knowledge, retrieve for
  the tail. Marchionini "lookup": precision-first single-query, early stop.
- **Evidence:** Strong and convergent (12, 19, 20, 24, 28, 33, 34, 35). GraphRAG/CS-textbook
  benchmark shows extra context *lowers* accuracy on facts the model knew (35).
- **Open questions (experiment-only):** Where is the parametric-vs-corpus crossover *on our
  corpus* (RustEvo²-style 3-arm: no-context / RAG / gold-chunk)? Does forced retrieval measurably
  hurt well-known software/physics facts? Does the confidence label correctly predict "skip search"
  without losing the 0% hallucination floor?

### T2 — Literature synthesis / survey *(Strong)*
- **Strategy:** Plan/outline first, then **retrieve per node/subsection** — holds citation
  grounding flat as output lengthens (AutoSurvey: 82% recall across 8k–64k vs naive RAG decaying
  to 69%). Decompose into 3–5 perspective/sub-concept queries (STORM ablation: breadth comes from
  multi-perspective questioning, not the writer; removing it collapses references 99.8→39.6).
  Fan-out + union; dedup by breadcrumb root before counting sources (target 2–3 *independent*
  sources/claim). MMR (λ≈0.5–0.7) for distinct-angle coverage. Stop on **coverage saturation** /
  source-rediscovery (seed re-surfaces) + source-diversity floor.
- **Evidence:** Strong (01, 02, 04, 05, 10, 11, 26, 30, 32, 35, 36, 39, 41). The recurring caveat
  is our missing citation graph (32, 36) and that LLM-judge synthesis scores cap at ρ≈0.5 vs humans.
- **Open questions:** Per-section vs upfront retrieval on *our* long outputs — does grounding
  decay at 16k+ tokens as predicted? Where does the coverage knee sit on our small/imbalanced
  particle-physics collection? Does a breadcrumb-only "scout" pass improve final coverage vs
  jumping to k=20? Does "poor-man's global" (decomposed sub-queries + map-reduce) recover the
  comprehensiveness GraphRAG gets without a graph index?

### T3 — Maths / derivation / proof *(Strong)*
- **Strategy:** Default to **no retrieval**; retrieve only the *static fact the procedure consumes*
  (a definition, constant, theorem statement, detector formula) — never the derivation steps
  (Ruis: reasoning influence is spread thin across procedures, not looked up). Tiny k (3–8) because
  hard negatives hurt derivations most. Exploit canonical names; concept-before-claim (define
  quantity → find formula). Math is the predicted **exception** to verbatim-beats-HyDE: retrieving
  against a *drafted target statement* (ProofNet ŷ trick) may beat literal phrasing. Pair with a
  code/computer tool for arithmetic.
- **Evidence:** Strong (07, 08, 09, 19, 26, 43, 46). KG-RAR: structured > unstructured, refine >
  raw chunks; benefit is capability-gated.
- **Open questions:** Does the HyDE-for-math exception actually hold on our corpus (A/B/C: verbatim
  / target-statement rewrite / draft-against-answer)? Does declining to retrieve on derivations
  correlate with correctness here as in the adaptive-retrieval literature? Score the
  *attributed-but-wrong* cell (faithful citation, misapplied step) — predicted common, invisible to
  answer-matching.

### T4 — Science / factual QA *(Strong)*
- **Strategy:** Single-step for single-hop; decompose-then-retrieve for multi-hop with hop-count
  matched to claim structure (FEVER optimal ~2 hops; HoVer 2/3/4 by subset). Watch early-hop error
  propagation (decompose-then-iterate hybrids recover the most complete evidence). Empirical/numeric
  claims are the sweet spot; theoretical/contested claims are weak (Elicit-vs-Scite split). Score
  evidence-conditioned (answer AND right chunk), not label-only.
- **Evidence:** Strong (06, 09, 15, 19, 22, 37).
- **Open questions:** Does decompose-then-iterate beat naive iteration on our multi-hop physics
  questions (log per-hop recall for propagation)? Where does the librarian's curated-source depth
  let it use *fewer* searches than open-web fact-checkers (the 1-query-Wikipedia-beats-5-query-web
  result)?

### T5 — Grounded writing *(Strong)*
- **Strategy:** Plan outline from parametric knowledge, then retrieve per heading/claim. Trigger
  re-retrieval forward-looking on low-confidence spans (FLARE: best at 40–80% of sentences, not
  every step). Cite **retrieved chunks only** — this is what converts the 18–95% fabrication range
  to ~0%. Choose pattern by source: search-then-write when grounding must precede commitment;
  draft→RARR-verify (write-then-verify) when the draft is mostly parametric-correct and retrieval's
  job is attribution. ~50% of long-form statements are typically unsupported even with strong
  models — claim-level citation recall/precision is the headline metric, not fluency.
- **Evidence:** Strong (11, 23, 39, 42). The quote-first compression trade-off (snippets cost ~8pts
  citation recall but let the model see more passages) is a tunable, not a free win.
- **Open questions:** Per-paragraph vs per-claim vs upfront cadence on our writing tasks (predicted
  to converge by 2–3/section)? Does our breadcrumb metadata recover the recall ALCE lost to
  snippetting? Does a capped RARR/OpenScholar feedback loop lift citation-F1 (0.1→39.5 elsewhere)
  on synthesis but not lookup/coding?

### T6 — Coding / API lookup *(Strong)*
- **Strategy:** The opposite of synthesis — shallow, selective, structured-before-semantic.
  Resolve-then-fetch with a tight topic filter; version/edition-pinned. **Confidence-gate
  retrieval (DAG++)**: forced grounding *regressed* well-known APIs −39% in CloudAPIBench, so
  always-on RAG is a net negative on common APIs. Add a deterministic exact-match/grep path fused
  with dense search (Cursor hybrid +12.5%, largest on big corpora). Verify the cited symbol appears
  verbatim in a retrieved chunk (MARIN-style existence check). Separate retrieve-k (20) from
  context-k (rerank to 3–5).
- **Evidence:** Strong (12, 24, 28, 44, 47). Hallucination tracks API *frequency* in training data,
  not difficulty — our detector/Rust niche is low-frequency by construction (parametric collapse
  to ~38%).
- **Open questions:** Does confidence-gating prevent the common-API regression on our stack? Build a
  frequency-stratified eval set to locate the grounding-helps-vs-hurts crossover. Does
  rerank-to-3–5 beat pasting all chunks on exact-signature accuracy?

### T7 — Debugging / known-issue lookup *(Medium-strong)*
- **Strategy:** This is the one task where **interactive multi-step retrieval reliably beats
  one-shot** (+21–28pts over BM25 RAG on localization). The librarian fills the InferFix/RAGFix
  slot: retrieve a grounded explanation / canonical fix-pattern for an *observed error*, not code
  localization. Error-as-query — verbatim for clean compiler/stack errors, *reformulated* for vague
  symptom reports (+20–35%; refines, not contradicts, verbatim-beats-HyDE). Narrow-and-deep: follow
  one promising breadcrumb thread. **Cap hypothesis-test cycles at 2–3** (debugging capability
  decays 60–80% within 2–3 attempts); a flat/worsening result = re-retrieve broadly, don't patch
  again. Cheap-first tiered policy (Agentless is 3–4× cheaper than open-ended agentic search).
- **Evidence:** Medium-strong; well-quantified but mostly on code-localization corpora, transferred
  by analogy to the librarian's "known-pitfall" role (13, 23, 25).
- **Open questions:** Verbatim-error vs reformulated-error on our corpus by error class
  (conceptual/algorithmic vs surface/API)? Does fresh-start re-retrieval beat round-4+ patching?
  Cost-per-resolved-question, not recall, as the metric.

### T8 — Learning / tutoring *(Medium — pedagogy strong, RAG-mapping inferred)*
- **Strategy:** This task **inverts the quote-first contract** — Levonian: humans prefer RAG
  answers *but not when too grounded in textbook text* (verbatim is pitched at the wrong level).
  Retrieve broad, then *transform*: elaborated worked example (novice) or Socratic hint (expert),
  conditioned on an expertise signal (worked-example effect → expertise-reversal → guidance
  fading). "Retrieve to withhold" for Socratic mode (don't reveal the answer; 57% leakage baseline
  to beat). Free-recall is the deliverable (testing effect g=0.81 for generative format).
- **Evidence:** Medium — pedagogy literature is strong (CLT, expertise-reversal, testing effect);
  the mapping onto librarian usage is inferred, not yet measured (14, 40).
- **Open questions:** Does quote-then-elaborate beat quote-first on learner-rated helpfulness (the
  Levonian inversion)? Can breadcrumbs act as a prerequisite graph (traverse *upward* to the
  presupposed concept)? Does withholding-then-eliciting-recall beat chunk-first reading on retention?

### T9 — Fact-checking / verification *(Strong)*
- **Strategy:** Decompose → retrieve → NLI/entailment verdict. ≤5 search queries per atomic fact
  (SAFE: correlation with humans peaks at 5, flat after); against a *curated* source, 1 query can
  match 5-query-web. Decompose only for multi-part claims (MiniCheck: near-zero decomposition gain
  for strong models on simple claims; "molecular facts" = minimal self-contained unit, not
  smallest). Set-level sufficiency stop (support/refute/insufficient over the *accumulated* set),
  not per-passage relevance. Verify *independently of the draft* (CoVe: factored beats joint —
  conditioning on the draft re-copies its errors). Extend abstention to a fourth verdict —
  *conflicting* — and measure recall (the documented weak spot: models miss real contradictions).
- **Evidence:** Strong (15, 27, 31, 41, 42, 43).
- **Open questions:** Baseline conflict rate per task on our corpus (how often do top-k chunks
  actually disagree)? Does forcing conflict-type reasoning before answering give the +24/+9pt
  DRAGged lift here? Cheap citation correction (CiteFix keyword+semantic, +15%) vs LLM re-judging
  (worst, ~100× slower) on our chunks.

### T10 — Design decisions / ADR *(Medium)*
- **Strategy:** Branch on *sub-shape*: (a) pattern/tactic lookup → 1 search, near-QA; (b) trade-off
  analysis → decompose by quality attribute (one query per ATAM utility-tree branch), source-diversity
  stop; (c) ADR authoring → few exemplars beat dump-all (Last_K(3) ≈ All_N), recency window, reduces
  verbosity; (d) audit/violation-detection → single retrieve + multi-model cross-check. A single
  high-confidence chunk is *insufficient* for trade-off/authoring (single-source over-reliance is the
  failure mode). Verification pass > more searches for divergent work.
- **Evidence:** Medium (17); corpus-grounded in SAIP/Cervantes-Kazman but ADR-generation evidence is
  semantic-similarity-judged.
- **Open questions:** Does "3 exemplars" match "dump all" on our ADRs at lower tokens? Does per-QA
  decomposition recover more distinct trade-off points than one fused query?

### T11 — Requirements grounding (Volere/MRP) *(Medium-strong)*
- **Strategy:** Exception-driven, not ritual — practitioners consult standards *rarely and by
  personal choice* (~47% don't even know the core RE standard). One well-aimed verbatim query per
  atomic requirement-artifact (fit criterion, snow-card field, business-event); escalate to a second
  refined query *only* on low confidence or conflict. The Volere template is structurally a retrieval
  index (section-level just-in-time lookup). Most sub-steps shallow, a minority deep. Metric is
  **citation-grounding rate** (failure mode = fabricated/mis-attributed section), not answer correctness.
- **Evidence:** Medium-strong; corpus-grounded in MRP/Volere + RE-practice surveys, RAG-mapping via
  Adaptive-RAG (16).
- **Open questions:** Does a per-task router beat fixed k=20-always on accuracy *and* query count?
  Breadth (high-k single shot) vs depth (iterative) on a synthesis prompt scored against the Volere
  Knowledge Model cross-reference graph (a rare clean recall gold set)?

### T12 — Implementing papers (theory→code) *(Medium)*
- **Strategy:** Non-linear, staged (Keshav three-pass): triage-first, deepen only on the
  re-read "Approach in Details" section. **N small targeted queries (one per equation/component)
  beat one high-k pull** — the implementer reads one section repeatedly. Notation disambiguation is
  the top failure mode (~30% of paper-grounded code implements equations); add a dedicated "what
  does symbol X mean here" query type. State-dependent iterative retrieval (ITER-RETGEN/ARCS:
  retrieve→synthesize→execute→repair). Stop when every equation in the data-flow chain has a
  grounded mapping; skip proof math. Validate by reproducing the paper's intermediate numerical values.
- **Evidence:** Medium; practitioner guides + agentic-RAG coding results, transferred (18).
- **Open questions:** N-small-targeted vs one-high-k on implementation correctness + notation
  faithfulness? Does intermediate-value reproduction localise per-component errors as predicted?

---

## 3. The dials, summarised (what changes across tasks)

| Dial | Lookup end (T1/T3/T6) | Synthesis end (T2/T5/T9) |
|---|---|---|
| **Retrieve-or-not** | gate / often skip | always |
| **# searches** | 0–1, hard-stop | 3–7, iterate |
| **Trajectory** | none (single specialized query) | broad→narrow→intent-shift; decompose + berrypick |
| **Breadth vs depth** | depth, one thread | breadth, multi-source |
| **k (retrieve / answer)** | small / smallest (≈5/3) | broad / narrow (20/~5) |
| **Granularity** | tight chunk | chunk→expand to section per node |
| **Stopping rule** | first confident hit | coverage saturation + source-diversity floor |
| **Generation** | quote-first verbatim | plan-first, per-section, claim-source matrix, verify-revise |
| **Reuse** | reuse, drop prior chunks fast | iterative re-retrieve, verify sufficiency before each call |
| **Cost regime** | cheapest point | 4–15× tokens; only for high-value breadth-first |

---

## 4. The highest-value experiments (the questions only an experiment settles)

Ranked by recurrence and expected leverage across the 50 notes:

1. **Task-type / complexity router vs fixed policy** — the central Round-2 experiment. Establish
   the *oracle* ceiling first (run all arms on a labelled set), then measure how close a cheap
   prompt-classifier gets. Report the oracle gap. *(Every note; Adaptive-RAG protocol.)*
2. **Per-task search-budget sweep** {0,1,2,3,5,8} × task type. Locate each task's knee; confirm the
   degradation cliff past ~8 and "query drift" on already-good queries. Anthropic effort tiers
   (1 / 2–4 / 10+ calls) as the prior. *(03, 19, 24, 38, 46, 48.)*
3. **Confidence label as a forward-looking stop/route signal.** Wire the existing label into a
   FLARE/Stop-RAG-style gate; calibrate to a ~10–20% re-query budget; instrument over-search vs
   under-search directly (target the ~70% over-search overhead). *(20, 24, 27, 29, 38, 49.)*
4. **Decouple retrieve-k from answer-k by task** + a rerank/RCS pass (PaperQA2: retrieve 30, answer
   on 5). Does precision rise and token cost fall? Does k=8 beat k=20 on citation precision for
   reasoning tasks? *(08, 10, 28, 44, 47.)*
5. **Breadcrumb expansion / "breadcrumb-snowball" mode**, A/B by task: chunk→parent-section expansion
   for derivations/synthesis vs tight chunk for lookup; embedding-diverse re-queries to manufacture
   the missing citation-graph edges. *(28, 32, 36, 40.)*
6. **Plan-first vs query-first**, conditioned on compositional depth; instrument the plan as a
   measurable artifact (STORM: outline quality predicts final quality). Crossover at the single-node
   (lookup) case. *(04, 37, 48.)*
7. **Factored verify-revise loop gated by abstention**, synthesis/writing only; predicted ~zero or
   harmful on lookup/maths/coding (which have stronger external verifiers — compiler, unit test,
   exact value). *(27, 39, 48.)*
8. **HyDE-for-math exception** (verbatim / target-statement rewrite / draft-against-answer) — the one
   predicted reversal of the Round-1 verbatim win. *(07, 08.)*
9. **Coding confidence-gate (DAG++)** — does forced retrieval hurt well-known-API answers on our
   stack? Frequency-stratified eval set to find the crossover. *(12, 44.)*
10. **Reuse-or-re-retrieve gate across turns** — replicate the zero-cost prompt fix (consult prior
    chunks first) before any architectural change; measure retrieval-laziness on our corpus. *(25.)*

---

## 5. Measurement backbone (so results are comparable across tasks)

- **Never one number.** Per task: (a) correctness (task-appropriate: EM/exact-signature for
  lookup/code; step-level earliest-error for maths; nugget/vital-recall for synthesis), (b)
  **attribution** scored separately (ALCE citation recall/precision via NLI, at *atomic-claim*
  granularity to avoid over-penalising multi-source sentences), (c) cost (searches × k × tokens).
  *(41, 42, 43, 44, 45.)*
- **Hallucination Ratio = 0 is a hard pass/fail gate**, reported separately, not folded into F1
  (the Round-1 abstention contract). The next axis is **recall without re-introducing unsupported
  claims**.
- **Ban LLM-as-judge for attribution** (CiteGuard: recall as low as 16%; CiteFix: LLM re-matching is
  worst and ~100× slower). Use NLI/keyword+semantic alignment. For synthesis *content* quality,
  ensemble judges from a *different* model family, randomize order, anchor to a small human-ranked
  set, and treat ρ≈0.5 as the ceiling. *(05, 41, 43.)*
- **Synergy, not raw accuracy, is the human+AI headline** (Vaccaro: naive teaming usually *loses*,
  g=−0.23; gains only when each party owns its stronger subtask). Pre-register human-alone and
  AI-alone-from-memory arms so synergy is computable. The librarian sits in the *favourable* corner
  (human directs/verifies, AI drives high-recall retrieval) — but the fix for overreliance is
  *friction* (cognitive forcing / cheap verification via breadcrumb-to-source), not more explanation.
  *(50.)*

---

## 6. Caveats on transferability

Magnitudes do **not** transfer; directions do. The external numbers come from web/news/Wikipedia
corpora, code-localization benchmarks, and graph-RAG systems the librarian does not run. Our corpus
is **curated, version-stable, breadcrumb-chunked, prose-embedding-only, two-collection, small/
imbalanced in particle-physics**. Consequences flagged repeatedly: (a) curation means *fewer*
searches may suffice than open-web tools need (15, 32); (b) prose embeddings have a structural blind
spot on notation-exact/formula lookups — fall back to name/grep search (07); (c) no citation graph —
synthesis breadth must come from embedding-diverse re-queries, and breadcrumb traversal only buys
intra-document depth (32, 36); (d) a *good* retriever surfaces more dangerous hard-negative
distractors, so the over-retrieval penalty is real for us (49); (e) two collections make collection
routing low-stakes, but cross-domain "software for physics detectors" queries are a first-class cell
where single-collection routing silently drops evidence — use RRF + rerank and hold them out as
their own condition (29). Treat every threshold above (k=8, 2–3 sources, ≤5 verify queries, 2–3
debug cycles, ρ≈0.5) as a **starting hypothesis to recalibrate on our own labelled queries**, not a
target to reproduce.
