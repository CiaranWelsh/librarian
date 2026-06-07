# Benchmarks for Deep-Research Agents: What They Measure, Who Wins, and Why

**Scope.** Four benchmarks define the current evaluation landscape for deep-research / browsing agents: **GAIA** (general assistant tasks), **BrowseComp** (hard-to-find facts), **DeepResearch Bench** (open-ended report quality), and **ResearchQA / ResearchRubrics** (scholarly synthesis against rubrics). They split cleanly into two regimes — *short verifiable answers* vs. *long synthesised reports* — and that split is the central lesson for task-conditioned tool use.

## What each benchmark measures

**GAIA** (Mialon et al., 2023): 466 real-world assistant questions (300 held out) needing reasoning, multimodality, web browsing, and tool use, organised into three difficulty *levels* by the number of steps/tools required. Design philosophy is deliberately inverted from "harder for humans" benchmarks: tasks are *conceptually simple for humans, hard for AI*. Humans score **92%**; GPT-4 with plugins **15%** (Level 1 9.68, Level 2 1.89, Level 3 0). GPT-4o later reached ~**29%** average (L1 39.78 / L2 27.04 / L3 14.58). Sharp monotonic decay with step count is the signature.

**BrowseComp** (Wei et al., OpenAI, 2025): 1,266 short-answer questions built by *inversion* — start from a verified fact, construct a question whose answer is "hard to find but easy to verify," and confirm GPT-4o/o1 can't answer it and it isn't in top search results. Humans solved only **29.2%** within two hours. Models without browsing score ~0%; GPT-4o+browsing **1.9%**; **Deep Research ~51.5%** — a 27x gap that proves *tool access alone is worthless without a search strategy*.

**DeepResearch Bench** (Du et al., 2025, arXiv 2506.11763): 100 PhD-level open-ended tasks across 22 fields, domain-weighted from ~96k real queries. Scored two ways: **RACE** (Reference-based Adaptive Criteria-driven Evaluation) generates *task-specific, dynamically-weighted* criteria across Comprehensiveness / Insight-Depth / Instruction-Following / Readability and scores against a reference report (92.7% human agreement); **FACT** measures effective citation count and citation accuracy.

**ResearchQA** (Yifei et al., 2025, arXiv 2509.00496) and **ResearchRubrics** (2025, arXiv 2511.07685) evaluate *scholarly synthesis* against fine-grained rubrics mined from survey articles. ResearchQA: 21.4k queries / 160k rubric items from 54k surveys across 75 fields; an automatic pairwise judge hits 74% expert agreement. ResearchRubrics: 101 prompts / 2,593 criteria (avg 26/task), ~2,800 expert-hours.

## Score distributions and what separates winners

**Open-ended reports (DeepResearch Bench, RACE):** Gemini-2.5-Pro DR **48.88**, OpenAI DR **46.98**, Perplexity DR **42.25** — a tight 6-point band. Per-dimension scores barely move (all four ~45–49 for the leaders), so winners are separated by *evidence gathering*, not prose. FACT exposes the real gap: **Gemini 111.2 effective citations** vs OpenAI ~40.8 vs Perplexity 31.3. But there is a precision/recall trade: **Perplexity hits 90.2% citation accuracy** with few citations; Gemini 81.4% with many. *No system balanced breadth and precision.*

**Rubric synthesis (ResearchRubrics):** Gemini DR 67.7% / OpenAI 66.4% / Perplexity 56.6% rubric compliance (ternary); stricter binary grading drops everyone (61.5 / 59.7 / 48.7%). The decisive finding: **implicit reasoning + multi-document synthesis = 45–50% of all failures**, while explicit retrieval fails <20%. Performance *collapses universally* once a task needs **4+ sequential inference steps**. Response length correlates only weakly with quality (r≈0.24–0.28) — and that correlation is genuine density, not padding. **No retrieval-augmented system exceeds 70–75% rubric coverage** (ResearchQA).

**Short verifiable answers (BrowseComp):** Calibration *degrades* with tool use — calibration error 65% (o1, no browsing) → 82% (GPT-4o+browsing) → 91% (Deep Research). Yet accuracy scales smoothly with test-time compute, and **best-of-N over 64 samples beats single-shot by 15–25%** (best-of-N > weighted > majority vote) — the model "knows when it's right" even when poorly calibrated in absolute terms.

## What strategy actually moves the needle

The open-source **ODR+** study (Allabadi & Malof, arXiv 2508.10152) is the cleanest causal evidence. Baseline ODR, Anthropic, and Google all scored **0% on BC-Small (60 Q)**. Three additive improvements — **sub-question decomposition, iterative planning, and structured synthesis with a persistent state object** — lifted ODR+ to a SOTA **10%**, and ablations confirm all three contributed. So the wins come from *decompose → plan iteratively → maintain structured state*, not from more raw retrieval.

## Implications for task-conditioned librarian experiments

1. **Use two metric families, matched to task type.** For verifiable/maths/coding lookups, score exact-match + abstention-correctness (BrowseComp/GAIA style). For synthesis/writing/learning, build a **RACE/ResearchRubrics-style rubric** with task-specific, dynamically-weighted criteria and quote-grounded citation accuracy (FACT-style). Round-1 single-query metrics won't capture report quality.
2. **Treat decomposition + iterative planning as the primary experimental variable.** ODR+ shows these drive gains where raw search doesn't. Test: single verbatim query vs. *N decomposed sub-queries with a maintained state object* — this is the per-task strategy lever.
3. **Make step-depth a controlled factor.** Both GAIA and ResearchRubrics show universal collapse at 4+ sequential inference steps. Stratify librarian tasks by required reasoning depth; expect breadth (parallel sub-queries) to help shallow tasks and to *fail* deep multi-hop ones regardless of k.
4. **Exploit verifiability for stopping/aggregation.** Where answers are easy to verify (facts, code, theorems), best-of-N/self-verification yields 15–25% — so a "search, draft, self-check, re-search" loop should beat one-shot. Where they are not (open synthesis), invest in coverage and citation precision instead.
5. **Measure the breadth/precision trade explicitly.** No leader balanced effective-citation *count* against *accuracy*. For the librarian, report both: chunks-cited vs. quote-faithfulness. The k=20-vs-k=8 round-1 result likely interacts with task type — coverage-bound synthesis wants high k, verifiable lookups want low k + verification.
6. **Trust the abstention contract, but watch calibration under tool use.** Tool access *raised* overconfidence on BrowseComp. The librarian's confidence label + abstention contract (12%→0% hallucination) is exactly the mitigation; test whether it holds as the number of retrieved chunks grows.

## Sources
- GAIA — Mialon et al., arXiv 2311.12983; Klu glossary.
- BrowseComp — Wei et al., OpenAI, arXiv 2504.12516 (html v1) / openai.com/index/browsecomp.
- DeepResearch Bench — Du et al., arXiv 2506.11763; deepresearch-bench.github.io (leaderboard).
- ResearchQA — Yifei, Chang, Malaviya, Yatskar, arXiv 2509.00496.
- ResearchRubrics — arXiv 2511.07685 (html v1).
- Improving & Evaluating Open Deep Research Agents (ODR+) — Allabadi & Malof, arXiv 2508.10152; OpenReview N0d6tG377V.
