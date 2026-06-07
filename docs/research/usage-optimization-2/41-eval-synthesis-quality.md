# 41 — Evaluating Literature-Synthesis / Related-Work Quality

How do you *score* a synthesis the librarian produces? Three axes have stable, published methodology: **coverage / nugget recall**, **citation accuracy** (precision/recall + support), and **content quality** (LLM-as-judge). This note is the measurement backbone for the task-conditioned usage experiments: for literature-synthesis tasks specifically, the metric is not "did it answer" but "did it cover the field and attribute every claim".

## Coverage / nugget recall

The dominant scalable approach is the **nugget** methodology, revived from the TREC 2003 QA Track and refactored for RAG as **AutoNuggetizer** (Pradeep, Thakur, Upadhyay, Campos, Craswell, Lin). It runs in two LLM-driven steps: (1) *nugget creation* — extract atomic facts from relevant docs, labelled **vital** or **okay**; (2) *nugget assignment* — judge whether each nugget is supported by the system response. The headline metric is **strict vital recall** (recall over vital nuggets fully supported by the response). In the TREC 2024 RAG Track, fully-automatic nugget rankings **strongly correlated with (mostly) manual human rankings across 21 topics / 45 runs** ([arXiv 2411.09607](https://arxiv.org/pdf/2411.09607), [arXiv 2504.15068](https://arxiv.org/pdf/2504.15068)). TREC 2025 added a Step 2, **sub-narrative mapping**, to measure how comprehensively a response covers the *intended* facets, not just total fact count ([arXiv 2603.09891](https://arxiv.org/pdf/2603.09891)). Caveat: AutoNuggetizer explicitly scopes itself to *recall only* — citation support and fluency are out of scope and need separate metrics.

## Citation accuracy

Two distinct definitions co-exist and must not be conflated:

1. **Support-based (ALCE)** — Gao et al.'s ALCE ([arXiv 2305.14627](https://ar5iv.labs.arxiv.org/html/2305.14627)) defines **citation recall** (is the sentence fully entailed by its cited docs?) and **citation precision** (is each individual citation necessary — remove it, does entailment break?), checked by an NLI model (TRUE, a T5-11B). Reported as recall/precision/F1. Validated against humans: Cohen's κ showed *substantial agreement* on citation recall, and gains on ALCE metrics tracked human preference. This is the right model for our setting because the librarian returns the exact chunks — entailment between chunk and generated sentence is directly checkable.
2. **Reference-set coverage** — fraction of ground-truth (human-cited) references the system reproduces. Used by survey-generation work; SurveyGen matches references with a 0.95 textual-similarity threshold ([arXiv 2508.17647](https://arxiv.org/pdf/2508.17647)).

A complementary pair from related-work evaluation ([arXiv 2508.07955](https://arxiv.org/pdf/2508.07955)): **Missing Ratio** (provided papers not cited) and **Hallucination Ratio** (cited papers not in the provided list). The latter is the direct analogue of our Round-1 hallucination finding — a *valid* synthesis should score 0. They also score citation–context **coherence** via NLI and require a *perfect 1.0* as a hard constraint.

## Content quality (LLM-as-judge)

ROUGE is inadequate here: it is lexical overlap, does not measure faithfulness, and **does not correlate with factual correctness** — and 25–30% of SOTA summaries contain factual errors ([arXiv 1908.08960](https://arxiv.org/pdf/1908.08960), [arXiv 2008.11293](https://arxiv.org/pdf/2008.11293)). The field has moved to **multi-LLM-as-judge** on named rubrics. AutoSurvey ([arXiv 2406.10252](https://arxiv.org/html/2406.10252v2)) scores **Coverage / Structure / Relevance** plus citation recall/precision, reporting **82.25% recall, 77.41% precision at 64k tokens** vs human **86.33 / 77.78** and naive RAG **68.79 / 61.97**. Follow-ups on the same protocol: SurveyForge 88.34/75.92, SurveyG 91.40 recall / 83.49 F1, SurveyX 78.12 precision. LLM-judge reliability is real but conditional: human–judge correlation for summarization sits around r≈0.4–0.6, rising to r≈0.65–0.82 in scientific/clinical settings with reasoning models (one clinical study: GPT-o3-mini ICC 0.818) — but judges carry self-preference and position bias, so **ensembling multiple judges** measurably improves human correlation ([arXiv 2411.02448](https://arxiv.org/pdf/2411.02448), [Reference-Guided Verdict, arXiv 2408.09235](https://arxiv.org/pdf/2408.09235)).

## Benchmark datasets

- **Multi-XScience** ([arXiv 2010.14235](https://arxiv.org/abs/2010.14235)) — related-work-section generation from abstract + cited abstracts; the canonical RWG benchmark.
- **SciReviewGen** ([arXiv 2305.15186](https://arxiv.org/abs/2305.15186)) — 10k+ CS literature reviews over 690k cited papers (S2ORC), query-focused MDS; uses abstracts because ~30% lack full text. Keeps citation sentences + citation network.
- **ALCE** (ASQA/ELI5/QAMPARI) for attributed long-form QA; **TREC RAG 2024/25** + NeuCLIR Report Generation for nugget-based report scoring.

## Implications for task-conditioned librarian experiments

1. **Use a three-metric panel, not one number**: strict vital-recall (coverage) + ALCE-style support precision/recall (attribution) + ensembled LLM-judge on Coverage/Structure/Relevance. Each axis catches a failure the others miss; recall alone rewards padding, precision alone rewards terseness.
2. **Hallucination Ratio = 0 is a hard pass/fail gate**, mirroring the Round-1 abstention contract. Report it separately, not folded into F1.
3. **Build a librarian-native gold set** by reverse-engineering related-work sections from the indexed corpus (Multi-XScience / SciReviewGen recipe): held-out section + its cited chunks = ground-truth reference set, enabling reference-coverage recall over *our own* chunks.
4. **Make coverage saturation the stopping-rule signal for synthesis tasks**: track marginal new vital-nuggets per additional query; stop when the derivative ≈ 0 (citation-recapture rule) or after N consecutive zero-yield queries (the "disgust"/consecutive-irrelevant heuristic). This is the per-task knob that distinguishes synthesis (high-recall, breadth-first, late stop) from known-item lookup (early stop).
5. **NLI-check every claim against its returned chunk** before emitting — cheap given we already hold the top-k chunks; turns citation precision from a post-hoc metric into a generation-time gate.
6. **Ensemble judges and randomize order** to neutralize self-preference/position bias when ranking usage strategies; a single judge will not reliably separate close strategies (r≈0.4–0.6 noise floor).
