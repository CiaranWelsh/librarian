# Evaluating Source-Grounded Mathematical Work: Correctness + Attribution Metrics for Derivations and Proofs

**Scope.** When the librarian is used for maths/physics-derivation work, "did it work?" splits into two orthogonal axes that must be scored separately: **correctness** (is the derivation/proof actually sound?) and **attribution** (is each step grounded in a cited source that supports it?). This file surveys the published metrics for each axis and translates them into concrete evaluation harnesses for our setup (text-embedding-3-large, qdrant top-k, abstention contract, breadcrumbed markdown chunks). It is the *measurement* companion to file 08 (when retrieval helps maths) and file 27 (verify-revise loops); this one is about how to *score* the output.

## Axis 1 — Correctness: outcome-checking is not enough; go step-level

The decisive empirical finding is that **answer-matching massively overstates quality on hard maths**. ProcessBench (3,400 expert-annotated competition/Olympiad solutions; ACL 2025, [2412.06559](https://arxiv.org/abs/2412.06559)) shows that as difficulty rises, the share of solutions that reach the *right answer via flawed reasoning* ("false positives") exceeds **50%** at Omni-MATH level. So a final-answer oracle is the wrong correctness metric for derivations; you need **earliest-erroneous-step identification** (label = index of first wrong step, or −1 if all correct; scored by F1 over error/no-error classes). This is exactly the granularity a grounded-derivation evaluator needs, because a derivation can be globally wrong while every cited fact is individually correct.

The verifier landscape:
- **Formal verification (Lean/Coq)** gives a hard correctness guarantee but only for what the formal library can express. Autoformalization surveys ([2505.23486](https://arxiv.org/html/2505.23486v1)) split the metric into *syntactic accuracy* (does it typecheck) and *semantic equivalence* (does the formal statement mean the NL one) — and warn that **compilation ≠ correctness**. A reported pipeline ([2602.20770](https://arxiv.org/html/2602.20770)) certified all 50 correct solutions but produced **85 false positives** before user feedback, i.e. the verifier's *own* precision/recall must be measured. This is too heavy for most detector/textbook derivations.
- **Generative process reward models / "critic" verifiers** are the practical middle ground. ThinkPRM ([2504.16828](https://arxiv.org/abs/2504.16828)) — a long-CoT verifier fine-tuned on **1% of PRM800K labels (~8K)** — beats discriminative PRMs trained on ~100x more data and beats LLM-as-judge by **+7.2% on ProcessBench**; R-PRM ([ACL 2025](https://aclanthology.org/2025.emnlp-main.679.pdf)) gains **+8.5 F1** and scales with sampled trajectories (62.8→67.6 F1 from 2→4 samples). Takeaway: a *generative* step-verifier with a verification CoT is the cheapest reliable correctness oracle, and verification accuracy *improves with more verifier samples* — a tunable knob.

## Axis 2 — Attribution: AIS, then automate it with NLI entailment

The canonical framework is **AIS — Attributable to Identified Sources** (Rashkin et al. 2023): a statement *s* is attributable to source *P* iff "According to *P*, *s*" holds. It deliberately **sidesteps factuality** — *faithfulness ≠ factuality* is the central caveat (a survey, [awesome-llm-attributions](https://github.com/HITsz-TMG/awesome-llm-attributions), lists over-/under-attribution and attribution-hallucination as core failure modes). For maths this distinction is sharp: a cited theorem can genuinely support a step (faithful) yet the step still be misapplied (incorrect) — which is precisely why Axis 1 and Axis 2 must be scored independently.

The automatable operationalization is **ALCE** (Gao et al., EMNLP 2023, [2305.14627](https://ar5iv.labs.arxiv.org/html/2305.14627)), which uses an NLI model (TRUE) to score, per sentence:
- **Citation recall** = does the *concatenation* of all cited chunks entail the sentence? (is it fully supported)
- **Citation precision** = is each cited chunk pulling weight? A citation scores 1 iff the full set entails the sentence AND that chunk either entails it alone or its removal breaks entailment (catches irrelevant padding citations).

These NLI definitions transfer directly: our chunks already carry breadcrumbs, so per-step recall/precision is computable offline. Finer schemes exist — **AttrScore** relabels each citation *attributable / extrapolatory / contradictory / non-attributable*; SemanticCite uses *supported / partial / unsupported / uncertain* at ~84% accuracy (see file 27). The 4-way schema is the right output for our confidence label.

**Critically, do not use LLM-as-judge as the attribution oracle.** CiteGuard (2025, [2510.17853](https://arxiv.org/html/2510.17853v1)) shows plain GPT-4o citation judging has **recall as low as 16–17%** (rejects valid citations for lack of field context) at 100% precision; a retrieval-augmented agent recovers this to **65.4% on CiteME vs 69.7% human**, a +12.3% lift over the LLM-as-judge baseline. The cheap-heuristic result from CiteFix (file 27) agrees: keyword+semantic matching beats LLM re-judging at ~100x lower latency.

## Actionable implications for librarian experiments

1. **Two scorecards, never one number.** For every grounded-maths task, emit (a) a **correctness** score and (b) an **attribution** score, and report their *joint* distribution. The interesting cell is *attributed-but-wrong* (faithful citation, misapplied) vs *right-but-unattributed* (correct from parametric knowledge) — ProcessBench's >50% false-positive rate predicts the former will be common and is invisible to answer-matching.
2. **Score correctness at step granularity, not final-answer.** Build a small ProcessBench-style set of detector/physics derivations with expert earliest-error labels; use a generative step-verifier (ThinkPRM-style) as the oracle and *measure the verifier's own precision/recall* against the human labels before trusting it (the 85-false-positive warning).
3. **Use ALCE NLI recall/precision per step, computed offline from chunk text + breadcrumbs.** This is deterministic and cheap and exploits assets we already have. Reserve formal (Lean) verification only for the rare closed-form algebraic claim where it is tractable.
4. **Ban LLM-as-judge for attribution; use retrieval-augmented or NLI scoring.** CiteGuard/CiteFix both show LLM judging is the worst and slowest option; an NLI-entailment scorer over the cited chunks is the defensible default, with a CiteGuard-style re-retrieval pass for low-recall cases.
5. **Emit a 4-way per-citation verdict** (supported/partial/contradictory/unsupported, AttrScore/SemanticCite) and wire it to the abstention contract: *contradictory* or *unsupported* on a load-bearing step should force re-search-or-abstain, reusing the 12%→0% mechanism from Round 1.
6. **Treat verifier sampling as a tunable.** R-PRM/ThinkPRM show step-verification accuracy rises with sampled verification trajectories; include verifier-samples as an experimental factor so the correctness oracle's cost/accuracy is itself characterized rather than assumed.

## Sources
- ProcessBench (ACL 2025) — https://arxiv.org/abs/2412.06559
- ThinkPRM / Process Reward Models That Think (2025) — https://arxiv.org/abs/2504.16828
- R-PRM: Reasoning-Driven Process Reward Modeling (EMNLP 2025) — https://aclanthology.org/2025.emnlp-main.679.pdf
- Autoformalization in the Era of LLMs: A Survey (2025) — https://arxiv.org/html/2505.23486v1
- Pipeline for verifying LLM-generated mathematical solutions (2026) — https://arxiv.org/html/2602.20770
- An Evaluation Benchmark for Autoformalization in Lean4 — https://arxiv.org/pdf/2406.06555
- ALCE: Enabling LLMs to Generate Text with Citations (Gao et al., EMNLP 2023) — https://ar5iv.labs.arxiv.org/html/2305.14627
- CiteGuard: Faithful Citation Attribution via Retrieval-Augmented Validation (2025) — https://arxiv.org/html/2510.17853v1
- A Survey of Attributions for LLMs (AIS, faithfulness≠factuality) — https://github.com/HITsz-TMG/awesome-llm-attributions
- Generation-Time vs. Post-hoc Citation (2025) — https://arxiv.org/pdf/2509.21557
