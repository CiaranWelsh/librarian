# SYNTHESIS: Metrics for Judging Task-Conditioned Librarian Usage Experiments

*Round-2 measurement synthesis over the 50 numbered research notes in this folder. The companion
`SYNTHESIS-taxonomy.md` defines **what strategy** the assistant should use per task type (T1–T12);
this document defines **how to score** whether a given strategy actually worked, per task type.
Round-1 mechanics (verbatim query, k=20 retrieve / k=8 value point, quote-first generation,
abstention contract that drove hallucination 12%→0%) are taken as given.*

---

## 0. Two non-negotiable framing rules (true for every metric below)

1. **Never one number.** For every task, score at least three orthogonal axes —
   **(a) correctness** (task-appropriate: EM / exact-signature for lookup & code; step-level
   earliest-error for maths; nugget/vital-recall for synthesis), **(b) attribution** scored
   *separately* from correctness, and **(c) efficiency / cost** (searches × k × tokens). The ALCE
   line is explicit that *correctness ≠ attribution* — a fluent well-cited answer can be wrong, and
   a correct answer can be unattributed; measuring one hides failures in the other (41, 42, 43).

2. **Hallucination Ratio = 0 is a hard pass/fail gate, reported separately, never folded into F1.**
   This is the Round-1 abstention contract operationalized as a metric (cited papers / sections that
   do not exist in the returned set, must be 0). Only once that gate is passed does the *next* axis —
   **recall without re-introducing unsupported claims** — become the optimization target (41, 49).

**Judge-selection meta-rule.** *Ban LLM-as-judge for attribution.* CiteGuard shows plain GPT-4o
citation judging hits recall as low as 16–17% (rejects valid citations for lack of field context);
CiteFix shows LLM re-matching is the *worst and ~100× slowest* correction method (+1.9% at 1.586 s
vs +15.5% at 0.015 s for keyword+semantic). Use NLI entailment or keyword+semantic alignment for
attribution; reserve LLM-as-judge for *content quality* only, and there ensemble ≥3 judges from a
**different model family than the generator**, randomize answer order (>30-pt position-bias swings
documented), and anchor to a small human-ranked set (ρ≈0.5 is the realistic ceiling) (05, 36, 41, 43).

---

## 1. The metric panel by task type (which axes to compute for which task)

| Task type (taxonomy) | Correctness metric | Attribution metric | Coverage metric | Efficiency metric | Abstention/calibration |
|---|---|---|---|---|---|
| T1 Known-item / fact lookup | Exact Match | quote-is-verbatim-subsequence | — | searches-to-answer (target 1) | skip-search precision; 0% halluc. |
| T2 Literature synthesis | strict vital-nugget recall | ALCE recall/precision (atomic) | nugget coverage + distinct-source count + citation density | tokens-per-coverage-unit | conflict-recall; coverage-saturation |
| T3 Maths / derivation | step-level earliest-error F1 | ALCE per-step (NLI) | — | searches-per-missing-fact | retrieve-or-not correlation w/ correctness |
| T4 Science / factual QA | FEVER-score (answer **and** right chunk) | per-hop chunk-faithfulness | hop-recall | searches per hop | sufficiency (support/refute/insufficient) |
| T5 Grounded writing | claim correctness | ALCE claim recall/precision + % unsupported | per-section citation density | per-section retrieve count | flag-uncited-or-drop rate |
| T6 Coding / API | exact-signature / valid-invocation rate | symbol-appears-verbatim-in-chunk | — | retrieve-k vs context-k; tokens | confidence-gate skip rate; context-dominance flips |
| T7 Debugging | cost-per-resolved-question | fix-pattern grounded-in-chunk | — | hypothesis-cycles (cap 2–3) | re-retrieve-vs-patch decision |
| T8 Learning / tutoring | learner-rated helpfulness; free-recall accuracy | level-appropriateness | prerequisite coverage | passes run | answer-leakage rate (Socratic) |
| T9 Fact-checking | label accuracy + set-level verdict | NLI stance per claim | claim decomposition coverage | searches/claim (cap ≤5) | sufficiency FP rate; conflict-recall |
| T10 Design / ADR | semantic-sim to gold ADR; distinct trade-off points | unsupported-clause rate | QA-branch coverage | exemplars used (3 vs all) | single-source-overreliance flag |
| T11 Requirements | citation-grounding rate | section-attribution correctness | Knowledge-Model cross-ref recall | queries/artifact-field | escalate-on-conflict rate |
| T12 Implementing papers | intermediate-value reproduction | equation→source mapping faithfulness | equation-coverage of data-flow | queries-per-equation | notation-disambiguation queries |

---

## 2. Metric families (definition · judge protocol · caveats)

### A. Synthesis coverage / nugget recall (T2, T5, T10-tradeoff, T12)

**Definition.** Did the answer cover the *facts the field requires*, not just produce fluent text?
The dominant scalable operationalization is the **nugget** method (TREC 2003 QA → AutoNuggetizer for
RAG): extract atomic facts from the relevant docs, label each **vital** or **okay**, then judge
whether each nugget is supported by the system response. Headline metric = **strict vital recall**
(recall over vital nuggets fully supported). TREC 2025 adds **sub-narrative mapping** to measure
coverage of the *intended facets*, not raw fact count. Complementary first-class coverage metrics:
**Number of References (NR)** = distinct works cited; **Citation Density (CD)** = unique citation
markers per character/100 words (a synthesis can be correct yet under-grounded); **distinct
source_id / cluster count** after dedup (41, 05, 04).

**Judge protocol.**
- *Programmatic + LLM (nuggets):* AutoNuggetizer two-step LLM pipeline (create→assign). Validated:
  fully-automatic nugget rankings strongly correlated with manual human rankings across 21 topics /
  45 runs in TREC 2024 RAG (41).
- *Reference-set coverage:* fraction of ground-truth human-cited references reproduced; SurveyGen
  matches at a 0.95 textual-similarity threshold (41). Build a **librarian-native gold set** by
  reverse-engineering related-work sections from the indexed corpus (Multi-XScience / SciReviewGen
  recipe): held-out section + its cited chunks = the ground-truth reference set, enabling
  reference-coverage recall over *our own* chunks (41).
- *Programmatic (density/count):* CD = regex-count citation markers ÷ word count; distinct-source =
  count unique breadcrumb roots **after deduplication** (chunks sharing a book/chapter = one source).
- *Outline-coverage proxy (STORM):* heading **soft recall** and **entity recall** of a generated
  outline vs a gold one — a cheap *leading indicator* (outline quality correlated with final-article
  quality) computable before the full answer (04, 37).

**Known caveats.**
- AutoNuggetizer scopes itself to *recall only* — citation support & fluency are out of scope and
  need separate metrics; recall alone rewards padding (41).
- ROUGE / lexical overlap is **inadequate**: does not correlate with factual correctness; 25–30% of
  SOTA summaries contain factual errors yet score well on ROUGE (41).
- The corpus has **no citation graph** — only breadcrumbs. Breadcrumb traversal buys *intra-document*
  depth, not the *inter-cluster* breadth real snowballing gives; coverage gold sets must account for
  this, and synthesis breadth must be manufactured via embedding-diverse re-queries (32, 36).
- Coverage saturates: marginal new vital-nuggets per added query → 0 is the *stopping* signal
  (citation-recapture / N consecutive zero-yield queries); track the derivative, not the absolute (41).
- Magnitudes (82% AutoSurvey recall, 99.8→39.6 STORM reference collapse) come from web/Wikipedia
  corpora — **directions transfer, magnitudes do not** (taxonomy §6).

---

### B. Citation precision / recall — attribution (T2, T4, T5, T9, T11; core of every grounded task)

**Definition (ALCE, the de-facto standard).** Per generated sentence/claim:
- **Citation recall** = the sentence scores 1 if it has ≥1 citation **and** the *concatenation* of
  all cited passages **entails** it under an NLI model. "Is every claim actually supported."
- **Citation precision** = a citation scores 1 only if (a) its sentence already has recall=1 **and**
  (b) the citation is not irrelevant (it alone supports the sentence, *or* removing it breaks
  entailment). "No padded / decorative citations." Asymmetry: precision is gated on recall=1, so a
  hallucinated sentence drags both down.
- **Citation F1** = harmonic mean (42).
- Complementary related-work pair: **Missing Ratio** (provided papers not cited) and **Hallucination
  Ratio** (cited papers not in the provided list — must be 0 as a hard gate) (41).

**Judge protocol.**
- *NLI entailment (default, cheap, pinned):* **TRUE** (T5-11B fine-tuned on SNLI/MNLI/FEVER/SciTail/
  PAWS/VitaminC) or a DeBERTa-v3 NLI checkpoint; binary entailment. Multi-hop support handled by
  concatenating cited passages before one entailment call. **Pin the version** so cross-experiment
  scores stay comparable; reserve LLM-as-judge only for partial-support cases TRUE misses (42).
- *Atomic-claim granularity (ALiiCE), not whole-sentence:* parse the answer into atomic claims via
  dependency trees and score per claim — fixes the multi-sub-claim recall penalty and the redundancy
  false-positives that punish legitimately multi-source synthesis sentences (exactly the case
  literature synthesis produces) (42).
- *Generation-time gate:* because the librarian already holds the top-k chunks, NLI-check every claim
  against its returned chunk *before emitting* — turns citation precision from a post-hoc metric into
  a generation-time gate (41).
- *Cheap citation correction (not an LLM):* CiteFix keyword+semantic re-matching (+15.5%, 0.015 s);
  report a **citation-correction rate** separately from a **claim-correction rate** (27).

**Validation anchors (what "good" looks like, by task — do NOT use one threshold).**
- Factoid (ASQA) ≈ **70+** citation F1; open-ended synthesis (ELI5) ≈ **45–50**; list/enumeration
  (QAMPARI) hardest ≈ **20**. This 70/50/20 spread is direct evidence that synthesis and enumeration
  are intrinsically lower-citation-quality regimes and need **task-specific** "good enough" bars (42).
- Human-validation of the metric: Cohen's κ = 0.698 recall (substantial), 0.525 precision (moderate)
  vs human annotators (42).

**Known caveats.**
- **Sentence-level granularity over-penalizes** — use atomic claims (ALiiCE) (42).
- **Granularity is not monotonically good** — attribution quality *peaks at intermediate granularity*;
  forcing sentence-atomic citation fractures semantic dependencies and hurts larger models'
  synthesis; attribution quality and answer correctness are *decoupled* (42).
- TRUE cannot register *partial* support, so automatic **precision systematically under-reports** vs
  humans (42).
- **Correctness ≠ faithfulness**: up to 57% of citations are post-rationalized (model answers from
  memory then token-matches a doc) — faithful-looking but ungrounded; "difficult to spot, fosters
  misguided trust." Verify each emitted quote is a verbatim subsequence of a returned chunk (09, 27).
- LongCite/LongBench-Cite report much higher recall (GPT-4o ~88%, Claude-3-sonnet ~99%) under a
  *different* protocol — **not comparable** to ALCE; fix the protocol before comparing runs (42).
- The quote-first compression trade-off: snippetting cost ~8.3 pts citation recall (73.6→65.3 ASQA)
  while letting the model see more passages — a tunable, not a free win; test whether breadcrumb
  metadata recovers the lost recall (11).

---

### C. Grounded-correctness (task-appropriate; the "did it work" axis)

This axis is **task-specific** — the wrong correctness metric (final-answer matching on maths,
label-only on fact-checking) systematically overstates quality.

**C1 — Maths / derivation: step-level, not final-answer (T3, T12).**
- *Definition:* **earliest-erroneous-step identification** — label = index of first wrong step (or −1
  if all correct), scored by F1 over error/no-error classes. Final-answer matching is wrong:
  ProcessBench shows >50% of right-answer solutions reach it via flawed reasoning ("false positives")
  at Olympiad difficulty (43).
- *Judge protocol:* **generative process reward model / "critic" verifier** with a verification CoT
  (ThinkPRM beats LLM-as-judge by +7.2% on ProcessBench; R-PRM +8.5 F1, and accuracy rises with
  sampled verification trajectories — treat **verifier-samples as a tunable factor**). **Measure the
  verifier's own precision/recall** against human labels before trusting it (one Lean pipeline
  certified 50 correct but produced 85 false positives). Reserve formal Lean/Coq verification for the
  rare tractable closed-form claim; compilation ≠ correctness (43).
- *The interesting cell to score:* **attributed-but-wrong** (faithful citation, misapplied step) vs
  **right-but-unattributed** (correct from parametric knowledge) — both invisible to answer-matching;
  ProcessBench predicts the former is common (43).

**C2 — Coding: executable / signature correctness (T6).**
- *Definition:* exact-signature accuracy or **valid-invocation rate** (CloudAPIBench style); for paper
  implementation, **intermediate-value reproduction** of the paper's worked numerical example
  (localizable, per-component) (44, 18).
- *Judge protocol:* programmatic — execute, or check the cited symbol appears verbatim in a retrieved
  chunk (MARIN existence check folded into the quote-first gate). Stratify the eval set by **API
  frequency in the corpus** (CloudAPIBench recipe) so the grounding-helps-vs-hurts crossover is
  locatable. Track **context-dominance**: rate of cases where a retrieved chunk *flipped a correct
  parametric answer to wrong* (the −39% common-API regression risk) (44, 12).
- *Caveat:* hallucination tracks API *frequency*, not difficulty (GPT-4o 93.66% high-freq vs 38.58%
  low-freq); the detector/Rust niche is low-frequency by construction (44).

**C3 — Science / factual QA: evidence-conditioned, not label-only (T4, T9).**
- *Definition:* **FEVER-score** = verdict correct **AND** the correct evidence chunk retrieved (vs
  label-only accuracy). The FEVER gap is the lesson: 50.9% label-only collapses to 31.9% when correct
  evidence is required (15). For multi-hop, log **per-hop recall** to catch early-hop error
  propagation; optimal depth is hop-count-matched (FEVER ~2, HoVer 2/3/4 by subset) (15).
- *Judge protocol:* SciFact-style three-stage decomposed score (chunk-retrieval / rationale-quote-
  selection / claim-stance) so verification quality is separable from generation quality; CliVER
  anchors at 79.0% retrieval / 67.4% sentence-selection / 63.2% label precision (09).

**Cross-cutting caveat.** Accuracy is **high for empirical/quantitative claims, low for
narrative/theoretical/contested** ones (Elicit/Scite split) — stratify the eval set by claim type or
the strategy effect is unattributable (06). Retrieval helps facts and **hurts pure reasoning**
(static retrieval −6.3 pp GSM8K) — so for maths arms, *declining to retrieve* should correlate with
correctness, and that correlation is itself a metric (08, 49).

---

### D. Attribution (faithfulness — distinct from citation P/R and from factuality)

**Definition.** **AIS — Attributable to Identified Sources** (Rashkin et al.): statement *s* is
attributable to source *P* iff "According to *P*, *s*" holds. AIS deliberately **sidesteps
factuality** — *faithfulness ≠ factuality* is the central caveat (a cited theorem can genuinely
support a step yet the step still be misapplied). This is why Section C (correctness) and Section D
(attribution) must be scored independently (43).

**Per-citation verdict schema (richer than binary).** Emit a **4-way label per cited chunk**:
- AttrScore: *attributable / extrapolatory / contradictory / non-attributable*;
- SemanticCite: *supported / partial / unsupported / uncertain* (~84% accuracy).
Wire it to the abstention contract: **contradictory / unsupported** on a load-bearing claim forces
re-search-or-abstain (reuse the 12%→0% mechanism); **partial / uncertain** triggers a soften-the-claim
edit (27, 43).

**Judge protocol.**
- *Do NOT use LLM-as-judge.* CiteGuard: plain GPT-4o citation judging recall 16–17%; a
  retrieval-augmented validator recovers to 65.4% on CiteME (vs 69.7% human, +12.3% over the
  LLM-judge baseline). Use NLI entailment over cited chunks, with a CiteGuard-style re-retrieval pass
  for low-recall cases (43).
- *Mechanistic drift check (FACTUM):* attributional drift = a claim emitted by the FFN/parametric
  pathway while attention fails to ground the citation; the resulting "retrieved-but-non-supporting"
  citation is *not* caught by the abstention contract and needs a separate atomic-claim→exact-span
  faithfulness check (49).

**Known caveats.** Over-/under-attribution and attribution-hallucination are core failure modes
(awesome-llm-attributions). SourceCheckup found **50–90% of medical-RAG citations not fully
supported** even when a source was provided — a citation is necessary but not sufficient (11, 31).
Generation-time citation tends to beat post-hoc attribution; the librarian's quote-first contract is
the generation-time variant (43).

---

### E. Efficiency per token / search (every task; the cost axis)

**Definition.** Quality is bought with search/token volume, so every result must carry a cost figure:
**searches-per-task**, **k (retrieve) vs k (answer) split**, **tokens-per-task**, and the derived
**Tokens-Per-Correctness (TPC)** and **cost-per-resolved-question** (the headline for debugging) (13,
24, 38).

**Judge protocol (all programmatic — logged from the trajectory).**
- *Search-budget curve:* sweep search count {0,1,2,3,5,8} × task type; report the **knee** and the
  **degradation cliff** (peak 3–7 turns for multi-step, then turns 11+ degrade *below* the turn-5
  baseline from context pollution). Anchor: PaperQA2's measured **1.26 ± 0.07 searches/question** on
  hard literature QA; Anthropic effort tiers (fact-finding 3–10 calls, comparison 10–15, complex 10+
  subagents) (19, 10, 46).
- *Over-/under-search instrumentation (DAS / "Search Wisely"):* per step, label **over-search** (answer
  was already derivable from context) and **under-search** (a no-retrieval step that erred). Anchors:
  models run **~70.5% more searches than necessary** (0.620 vs optimal 0.364/query); one model could
  have skipped **27.7%** of search steps; TPC rises ~300–400 (base) → ~730–812 (search-augmented) →
  **38.9k for a Deep Research system (221×)**; noisy corpus amplifies searching **3.6×** (24, 23, 01).
- *VOC stop signal:* continue only while marginal expected gain (proxied by the chunk confidence
  label) exceeds cost; target the De Sabbata **20–37% token savings at maintained accuracy** (38).
- *Decouple retrieve-k from answer-k:* PaperQA2 retrieves 30, answers on ~5; report precision and
  cost as a function of both independently (10, 28).

**Known caveats.**
- **Long trajectories are a *symptom*, not a lever — never reward call count.** On SWE-bench the
  action distribution of solved vs failed runs was nearly identical (capability, not count, gated
  success; successful runs median 11 steps); on browsing benchmarks fewer-turns-win. Per-task call
  count is a *model property* (GPT-5 13–15 calls vs Qwen 35–50 on the same task) (24).
- **The effort paradox:** on retrieval-bound research, *more* reasoning budget often *degrades*
  quality (GPT-5 49.6→48.1% low→high effort) because the bottleneck is retrieval/source-evaluation,
  not deduction — so the maths-style "more thinking helps" intuition does **not** transfer; test it
  per task rather than assume it (38).
- **Test-time scaling saturates / can invert:** self-consistency gains flatten by ~20–40 samples and
  can decline; best-of-N needs a *real verifier* to pay off (best-of-N > weighted > majority vote)
  (30, 45, 24).
- **Magnitudes are benchmark-specific** — read all of the above as *shape* guidance, recalibrate the
  knee on our own labelled queries (19, taxonomy §6).

---

### F. Abstention calibration (every task; the safety axis the contract already opened)

**Definition.** Two sub-axes: **(1) abstain-when-you-should** (no supporting chunk → don't answer;
the Round-1 0%-hallucination floor) and **(2) the calibration of the confidence label itself** — does
the librarian's per-chunk confidence label *predict* whether the answer was actually entailed by the
retrieved chunks? This turns the contract from a heuristic into a measurable calibration target (42).
Extends to **abstaining from *querying*** (skip-search) on head-knowledge / reasoning tasks, where the
*decision to abstain from retrieving* correlates with correctness (08, 49).

**Judge protocol.**
- *Calibration:* treat ALCE citation-recall as ground-truth "answer was entailed by retrieved chunks";
  measure whether the confidence label predicts it (reliability diagram / ECE). On BrowseComp,
  **calibration degrades with tool use** (65% error o1-no-browsing → 82% GPT-4o+browsing → 91% Deep
  Research) — so *re-measure calibration as k and search count grow*, do not assume it holds (45).
- *Answerability as a first-class label (RaCGEval):* 3-way classification *answerable / partial /
  unanswerable*; baselines hit only 46.7%. For fact-checking add **set-level sufficiency** over the
  *accumulated* chunk set (support / refute / insufficient); track **false-positive sufficiency**
  (premature stop) and wasted-continue rates; SAFE correlation with humans peaks at 5 queries/claim
  and is flat after (S2G-RAG reports 6.44% sufficiency FP) (44, 15).
- *Conflict-recall (the documented weak spot):* extend the contract to a fourth verdict —
  **conflicting** — judged over the chunk set, and **measure recall** (not just accuracy): best
  detector config (Claude-3 Sonnet + CoT) only 71.0% acc / 0.71 F1, **high precision but low recall —
  models miss real contradictions**; conflict-type classification harder still (65.3–74.3%). Score
  whether genuine multi-answer ambiguity is *reported* (RAMDocs multi-answer EM), not collapsed; ban
  silent majority-voting (suppresses correct minority sources) (31).
- *Skip-search precision:* log how often the assistant *declines* to query and correlate with
  correctness; on coding, measure context-dominance flips; target the TARG 70–90% retrieval cut at
  matched EM/F1 (08, 49, 12).

**Known caveats.**
- Abstention killed unsupported *claims* but does **not** catch a claim attributed to a
  retrieved-but-non-supporting chunk (FACTUM drift) — needs the separate Section-D faithfulness check
  (49).
- Search *improves* answerable-query accuracy (+24%) but *degrades* abstention on unanswerable queries
  (−12.8%) — the two move in opposite directions, so report them separately, not as one accuracy
  number (24).
- Conflict thresholds (κ>0.8, 2-source minimum) come from web/news fact-checking; treat as starting
  hypotheses (30, 31).

---

## 3. Cross-task design rules for a comparable, defensible experiment

1. **Establish the oracle ceiling first.** Run every strategy arm on a labelled set to get the
   *oracle-routed* cost/quality point, then measure how close a cheap prompt-classifier gets; report
   the **oracle gap** as the headline (Adaptive-RAG's lesson: the router, not the strategy arms, is the
   bottleneck — ~54% three-way classifier accuracy; the oracle is both cheaper *and* more accurate)
   (34, 45-style).

2. **Report augmentation AND synergy, not raw accuracy (human-in-the-loop tasks).** Augmentation =
   team vs human-alone; **synergy** = team vs the *best* of either alone. Pre-register human-alone and
   AI-alone-from-memory arms or synergy is uncomputable. Naive teaming usually *loses* (g=−0.23); gains
   only when each party owns its stronger subtask. The fix for overreliance is **friction / cheap
   verification** (breadcrumb-to-source), not more explanation (explanations/confidence displays were
   n.s. moderators). Outcome metric for overreliance = error-detection on *seeded wrong/absent
   evidence* (Buçinca 0.03→0.09), not user-reported trust (50).

3. **Reproducibility is two metrics, not one.** Re-run identical prompts and report **value-agreement**
   separately from **quote/reasoning-agreement** — Elicit showed 90% agreement on values but only 46%
   on supporting quotes and 30% on reasoning (quote-first generation should *help* quote stability;
   test whether it does) (06).

4. **Control phrasing as a factor.** Adaptive-RAG routing is phrasing-brittle; run each task in 2–3
   paraphrases and measure routing/answer *stability* — verbatim-query robustness (a Round-1 win)
   should extend to the routing layer (34, 06).

5. **Guard LLM-judge bias on any content-quality score.** Randomize answer order (>30-pt swings),
   control length and trial count; some reported GraphRAG-family advantages collapsed under audit
   (LightRAG 66.7%→39.1%) (36).

6. **Magnitudes do not transfer, directions do.** Every external anchor (k=8, 1.26 searches, 2–3
   sources, ≤5 verify queries, 70/50/20 citation-F1 spread, ρ≈0.5 judge ceiling) is a *starting
   hypothesis* to recalibrate on our curated, breadcrumb-chunked, two-collection, prose-embedding-only
   corpus — not a target to reproduce (taxonomy §6).

---

## 4. The judge-tool shortlist (pinned, reproducible)

| Axis | Primary judge | Fallback / escalation | Cost note |
|---|---|---|---|
| Coverage / nuggets | AutoNuggetizer (LLM create→assign) | reference-set match @0.95 sim | LLM, but validated vs humans (41) |
| Citation P/R (attribution) | NLI entailment (TRUE T5-11B / DeBERTa-v3), atomic-claim | CiteGuard re-retrieval for low-recall | cheap, pin version (42, 43) |
| Citation correction | keyword+semantic (CiteFix) | fine-tuned BERTScore | 0.015 s; **never LLM** (27) |
| Maths correctness | generative step-verifier (ThinkPRM/R-PRM) | Lean for closed-form | measure verifier P/R first (43) |
| Code correctness | execute / verbatim-symbol check | frequency-stratified set | programmatic (44) |
| Science correctness | FEVER-score; SciFact 3-stage | per-hop recall log | programmatic + NLI (15, 09) |
| Content quality | ≥3 LLM judges, diff. family, randomized order | anchor to human-ranked set | ρ≈0.5 ceiling (05, 41) |
| Efficiency | trajectory log (searches/k/tokens, TPC) | over/under-search labels (DAS) | programmatic (24, 38) |
| Abstention/calibration | ECE vs ALCE-entailment ground truth | RaCGEval 3-way; conflict-recall | re-measure as k grows (42, 45, 31) |
