# How AI Literature Tools Structure Search -> Extract -> Synthesize

*Research note for the librarian usage-optimization round 2 (task-conditioned usage). Focus: retrieval patterns, claim extraction, reported accuracy, and what they imply for designing per-task usage experiments on our private RAG.*

## The shared three-stage pipeline

All five tools (Elicit, Consensus, SciSpace, Undermind, Scite) converge on the same skeleton our librarian already has — **retrieve a large candidate set -> filter/classify down -> generate a cited synthesis over a small top-k** — but they differ sharply in *how many retrieval rounds*, *what does the filtering*, and *when they stop*. They also share back-ends: Elicit, Consensus and Undermind all lean on Semantic Scholar / OpenAlex, so "the differentiator is the layer on top" (Aaron Tay, *Musings about librarianship*; *Substack* 2025). That is precisely the layer our experiments control.

## Two retrieval architectures: one-shot vs agentic-iterative

The sharpest published finding is the split between **single-pass embedding search** (Elicit's original mode, SciSpace, Consensus) and **iterative agentic search** (Undermind). Undermind's whitepaper (Hartke & Ramette) is the only one with hard numbers on iteration and stopping:

- It runs **successive rounds** of keyword + semantic + citation search, using **GPT-4 as a full-text relevance classifier** into {highly relevant, closely related, ignorable} (whitepaper §1, Appendix 3.2).
- Crucially it models a **stopping rule**: the discovery rate of relevant papers **decays exponentially**, `f = 1 - e^(-n/tau)`, where `n` = papers evaluated and `tau` = a per-query time constant. A "converged search" is one predicted to find no further results.
- Concrete trajectory for a typical query (tau ~ 80, ~24 relevant papers): reading **150 papers finds ~85%** of all relevant results; **300 papers finds ~98%** (whitepaper §Fig.3). This is a published *breadth-vs-depth stopping curve* — directly relevant to our "how many searches / stopping rule" question.
- Reported quality: **~10x higher density** of relevant results in top hits vs Google Scholar's top 50; top-flagged papers are relevant with **>92% probability** (Table 2); classifier accuracy **~98%** (never flips highly-relevant <-> irrelevant); **<3% miss** of Google Scholar's relevant hits once converged.

Independent commentary corroborates the *direction*: Tay found Undermind beat one-shot embedding search "in both recall and precision," attributing it to multi-iteration search + LLM-as-relevance-judge + minutes-not-seconds runtime. The community heuristic is blunt: **"fast responses indicate lower quality."**

## Claim extraction patterns

- **Elicit** — structured **extraction tables**: user-defined (or auto-suggested) columns populated per paper, each cell carrying **sentence-level citations + quote + reasoning**. Synthesis spans up to ~200 papers with per-claim citations. This is RAG-grounded explicitly to suppress hallucination.
- **Consensus** — parses the query into **entities/outcomes/populations/modalities**, builds per-paper **Study Snapshots** (population, n, design, results, limitations), then aggregates into a **Consensus Meter** (supports / contradicts / mixed) over the **top 10-20** results. Stated principle: **"AI only *after* searching, never before"** — a sequencing rule to bound hallucination. Uses separate **"checker" models** to gate sources on a relevance threshold before they enter synthesis.
- **SciSpace** — Deep Review funnel: in one test analyzed **1,750 papers -> 320 relevant -> answer from top 20**; custom columns run per-PDF extraction prompts.
- **Scite** — different premise entirely: classifies **citation statements** (the citing sentence + neighbours + section) as **supporting / mentioning / contrasting** via a deep-learning model trained on hundreds of thousands of human labels (1.6B+ statements now).

## Reported accuracy and its caveats (the important part)

Vendor vs independent numbers diverge, and the *failure modes* are what should shape our experiments:

- **Elicit (vendor):** 95% search recall, 97% abstract / 99% full-text screening, 96% extraction across 994 Cochrane reviews; a case study claims 99.4% (1502/1511) extraction. **Independent:** Hilkenmeier et al. 2025 — **81.4%** Elicit vs **86.7%** human (n.s.); only **48%** of points needed human verification. Bianchi et al. 2025 (Cochrane) — Elicit deviated on only 4.3% but had "deficits" on complex variables. **Reproducibility is the real weakness:** repeating extractions gave **90% agreement on values but only 46% on supporting quotes and 30% on reasoning**; high-accuracy mode was *worse* (77% / 10% / 0%). Rephrasing the question changed both details and cited papers.
- **Consensus:** vs Google Scholar across 500 queries, **+4.6%** average precision; flagged for **oversimplifying live methodological disputes** (PMC 2025) — "a starting point, not a conclusion." No structured extraction tables.
- **Scite:** vendor precision high-80s/low-90s for support/contrast; **independent (Bakker et al. 2023)** found it **over-labels "mentioning"** (40 of 96 "mentioning" calls mischaracterized) — the rare, high-value supporting/contrasting signal is exactly where it is weakest. Natural label skew is **92.6% mentioning / 6.5% supporting / 0.8% contrasting**.

Cross-cutting lessons: (1) **query phrasing dominates extraction variance**; (2) accuracy is **high for empirical/quantitative claims, low for narrative/theoretical/contested** ones; (3) **reasoning/quote reproducibility lags value reproducibility** — agreement on *what* outpaces agreement on *why*.

## Actionable implications for task-conditioned librarian experiments

1. **Test an iterative/agentic mode against our settled single-query baseline.** Round 1 fixed single-query mechanics; the strongest external signal (Undermind) is that **literature-synthesis tasks specifically benefit from multi-round search with an LLM relevance gate** between rounds. Make "number of searches" a *task-conditioned* variable, not a global constant.

2. **Adopt an explicit convergence stopping rule for synthesis tasks.** Instrument the librarian to track the marginal relevant-chunk discovery rate and fit `1 - e^(-n/tau)`. Hypothesis to test: synthesis tasks need many searches to "converge"; maths/coding/lookup tasks saturate after 1-2 (depth, not breadth). This operationalizes our "stopping rules per task" question with a published functional form.

3. **Make claim type the primary moderator.** Bake in a contrast between **empirical/numeric claims vs theoretical/contested claims** — the literature predicts the librarian (like Elicit/Scite) will be strong on the former, weak on the latter. Particle-physics thresholds/APIs should behave like Elicit's "empirical" sweet spot; software-design tradeoffs like Scite's weak "contested" zone.

4. **Add a reproducibility metric, separate for values vs reasoning.** The Elicit 90%/46%/30% split says **answer stability and rationale stability must be measured independently.** Re-run identical task prompts and report value-agreement vs quote/reasoning-agreement separately. Round 1's quote-first generation should *help* quote stability — test whether it does.

5. **Treat query rephrasing as a controlled factor, not noise.** Multiple studies show rephrasing flips extracted details and cited sources. Since Round 1 found *verbatim query beats rewriting*, run a per-task rephrasing-sensitivity sweep: confirm verbatim's robustness holds for synthesis/learning tasks, where the external tools are most fragile.

6. **Keep the abstention contract as the hallucination floor and verify it transfers.** Consensus's "search-then-AI" sequencing and "checker" relevance gate are our abstention contract by another name (12%->0% in Round 1). Test that abstention holds under multi-round/agentic conditions, where added retrieval rounds could reintroduce low-relevance chunks into synthesis.

### Sources
- Undermind whitepaper (Hartke & Ramette), undermind.ai/whitepaper.pdf — iteration, exponential discovery curve `1-e^(-n/tau)`, ~98% classifier accuracy, >92% top-hit relevance, 85%/98% at 150/300 papers.
- Elicit independent evals: Hilkenmeier et al. 2025 (Sage); Bianchi et al. 2025 (Cochrane Evidence Synthesis & Methods); ecoevorxiv life-sciences feasibility study (reproducibility); PMC11921719 (repeatability). Vendor: elicit.com/solutions.
- Consensus: consensus.app features; UMaryland HSHSL guide (accuracy/limitations); PMC 2025 oversimplification critique.
- Scite: Nicholson et al. 2021 (*Quantitative Science Studies*, MIT Press); Bakker et al. 2023 (independent accuracy critique).
- SciSpace Deep Review: effortlessacademic.com hands-on tests; RAG mechanics from arXiv 2507.18910, 2512.09370 (query-phrasing -> extraction variance).
- Cross-tool: Aaron Tay, *Musings about librarianship* & *Substack* 2025 (shared back-ends, one-shot vs Deep Search).
