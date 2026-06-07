# ALCE and Citation Precision/Recall for Long-Form Grounded Generation

**Scope:** Metric definitions, judge protocols, typical published scores, and what they imply for designing *task-conditioned* usage experiments on the librarian (the question of how an AI assistant should use a top-k RAG tool differently across literature synthesis, maths, science, writing, coding, and learning).

## ALCE: the de-facto standard for automatic citation evaluation

ALCE (Gao et al., EMNLP 2023) is the first reproducible automatic benchmark for "generate text *with* citations." It scores three orthogonal dimensions — **fluency** (MAUVE), **correctness** (EM/recall against gold answers or claims), and **citation quality** — over three datasets that span the task spectrum that matters to us: ASQA (factoid, ambiguous QA, Wikipedia corpus), QAMPARI (list answers), and ELI5 (open-ended long-form, web corpus). Citation quality is the part everyone reuses, and it is split into two metrics computed by an NLI entailment model:

- **Citation recall** (per statement): a sentence scores 1 if it has >=1 citation **and** the *concatenation* of all its cited passages entails the sentence under the NLI model phi(premise=cited passages, hypothesis=sentence). Averaged over all statements. This is "is every claim actually supported by what it cites."
- **Citation precision** (per citation): a citation scores 1 only if (a) its sentence already has recall=1, **and** (b) the citation is not "irrelevant" — irrelevant means it alone cannot support the sentence *and* removing it does not change whether the remaining citations still support it. Averaged over all citations. This is "no padded / decorative citations."
- These combine as **citation F1** (harmonic mean). Note the asymmetry: precision is gated on recall=1, so a hallucinated sentence drags both metrics down.

**Judge protocol.** The entailment judge is **TRUE**, a T5-11B fine-tuned on a mixture of NLI sets (SNLI, MNLI, FEVER, SciTail, PAWS, VitaminC); it emits binary entailment. Multi-hop support is handled by concatenating the cited passages before the single entailment call. Default retrieval depth is **top-5 passages** for 4K-context models (ChatGPT) and **top-3** for 2K-context models. ALCE validates these automatic metrics against humans: Cohen's kappa is **0.698** for recall (substantial) and **0.525** for precision (moderate); standalone human annotator accuracy was 85.1% / 77.6%. The known failure mode is that TRUE cannot register "partial support," so automatic **precision systematically *under*-reports** versus humans.

## Typical published scores (anchors for what "good" looks like)

ChatGPT vanilla, top-5 (ALCE original): ASQA recall **73.6** / precision **72.5**, EM 40.4, MAUVE 66.6; ELI5 recall **51.1** / precision **50.0**, claim-correctness only **12.0**; QAMPARI recall/precision ~**20.5/20.9**. GPT-4 (5-psg) on ASQA ~68.5 / 75.6; LLaMA-2-70B-chat ~62.9 / 61.3; LLaMA-2-7B/13B collapse to F1 ~17. The durable pattern: **factoid tasks (ASQA) sit at ~70+ citation F1; open-ended synthesis (ELI5) sits at ~45-50; list/enumeration (QAMPARI) is hardest at ~20.** Long-context citation benchmarks using a different scoring scheme (LongCite/LongBench-Cite) report much higher recall — GPT-4o ~88%, Claude-3-sonnet ~99% citation recall — but those numbers are *not* comparable to ALCE because the protocol and granularity differ.

## Known limitations directly relevant to our design

1. **Sentence-level granularity over-penalizes.** ALCE evaluates a whole sentence against its sources; a sentence with several sub-claims must have *all* of them entailed, and a citation that legitimately supports only one sub-claim is flagged "redundant." ALiiCE (NAACL 2025) parses **atomic claims** via dependency trees and scores per-claim, fixing both the multi-sub-claim recall penalty and the redundancy false-positives, and incidentally avoiding NLI context-overflow from long concatenations.
2. **Granularity is not monotonically good.** A 2026 study ("Are Finer Citations Always Better?") finds attribution quality *peaks at intermediate granularity*; forcing sentence-atomic citation fractures semantic dependencies and disproportionately hurts larger models' synthesis — and crucially shows attribution quality and answer correctness are *decoupled* (you can calibrate citation granularity to improve attribution without hurting correctness).
3. **Correctness != attribution.** ALCE deliberately separates them; a fluent, well-cited answer can still be wrong, and vice versa. Any librarian experiment must measure both, separately.

## Actionable implications for task-conditioned librarian experiments

1. **Adopt ALCE's two-axis split as the per-task scoreboard, but report correctness separately.** For each task type, log citation-recall, citation-precision, and a task-appropriate correctness metric independently. Round-1's abstention win (12% -> 0% hallucination) is essentially a *recall-gating* result; ALCE gives the vocabulary to show it did not silently cost precision or correctness.
2. **Expect, and target, different score floors per task — do not use one threshold.** The ASQA/ELI5/QAMPARI spread (70 / 50 / 20) is direct evidence that synthesis and enumeration are intrinsically lower-citation-quality regimes than factoid lookup. Task-conditioned usage should set *task-specific* "good enough to stop" bars, mirroring Adaptive-RAG's routing of factoid -> single-step, multi-hop -> richer, summarization -> iterative retrieval. This is the strongest external justification for varying search count/breadth by task.
3. **Use an NLI/LLM entailment judge to score the librarian's own confidence label.** The librarian already returns a confidence label per chunk; treat ALCE recall as ground-truth "was the answer actually entailed by retrieved chunks" and measure whether the confidence label predicts it. This turns the abstention contract into a measurable calibration target rather than a heuristic.
4. **Score at atomic-claim granularity, not sentence.** Given ALiiCE's findings and our breadcrumb-chunk corpus (where one sentence may cite several chunks), decompose generated answers into atomic claims before entailment to avoid the redundancy false-positives that would otherwise punish legitimately multi-source synthesis sentences — the exact case literature synthesis produces.
5. **Make stopping rules explicit and task-conditioned, and measure them.** ALCE-style citation F1 plateaus give an empirical stopping signal: add searches/refinements only while citation recall is still rising. For factoid/maths/lookup, expect a single verbatim query (Round-1's k=20/k=8 finding) to saturate; for synthesis, expect iterative breadth (cluster -> dedup -> synthesize) to keep paying off. Instrument "marginal citation-recall gain per extra search" as the core dependent variable of the Round-2 experiments.
6. **Keep the judge cheap and pinned.** TRUE (T5-11B) or a DeBERTa-v3 NLI checkpoint is sufficient and reproducible; reserve LLM-as-judge only for the partial-support cases TRUE misses, and pin the version so cross-experiment scores stay comparable.

## Sources

- Gao, Yen, Yu, Chen — *Enabling LLMs to Generate Text with Citations* (ALCE), EMNLP 2023: https://aclanthology.org/2023.emnlp-main.398.pdf , https://ar5iv.labs.arxiv.org/html/2305.14627
- ALiiCE — *Evaluating Positional Fine-grained Citation Generation*, NAACL 2025: https://aclanthology.org/2025.naacl-long.23.pdf , https://arxiv.org/html/2406.13375
- *Are Finer Citations Always Better? Rethinking Granularity for Attributed Generation*: https://arxiv.org/pdf/2604.01432
- LongCite — *Enabling LLMs to Generate Fine-grained Citations in Long-context QA*: https://arxiv.org/pdf/2409.02897
- *Attribution, Citation, and Quotation: A Survey of Evidence-based Text Generation with LLMs*: https://arxiv.org/pdf/2508.15396
- Adaptive-RAG — *Learning to Adapt RAG through Question Complexity*: https://arxiv.org/pdf/2403.14403
- Stop-RAG — *Value-Based Retrieval Control for Iterative RAG*: https://arxiv.org/abs/2510.14337
