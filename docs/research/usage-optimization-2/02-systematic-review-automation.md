# Systematic-Review Automation: What PRISMA Pipelines Teach Us About Task-Conditioned Retrieval

## Why this matters for the librarian

A literature-synthesis task is the one work mode where retrieval is the *whole job*, not a sub-step. The
systematic-review (SR) / technology-assisted-review (TAR) community has spent fifteen years measuring exactly
the variables our Round-2 question asks about: how many searches, breadth vs. depth, when to stop, and what
recall you can *guarantee*. Their pipeline decomposes into three stages that map cleanly onto librarian usage —
(1) search-strategy construction, (2) screening, (3) stopping — and each has hard numbers.

## 1. Search-strategy construction: recall comes from *breadth*, not a clever single query

SR search design is explicitly recall-first: missing one eligible study is the cardinal sin, so precision is
sacrificed. Two paradigms exist — the **objective method** (expand a seed set by term co-occurrence / relevance
feedback) and the **conceptual method** (decompose into PICO concepts, then OR-union synonyms within concept,
AND across concepts) [Boolean-query review, arXiv:2505.07155]. Both converge on the same structural lesson:
coverage is achieved by **OR-unioning many term variants and many seeds**, not by one well-phrased query.

LLM query generation confirms this. **AutoBool** (EACL 2026) RL-trains a small model to optimize retrieval
metrics directly and beats zero-shot GPT-4o/o3 in high-recall regimes; the paper's headline guidance is that
"ensemble or OR-combination strategies across seeds or LLM outputs are warranted" for high-recall settings
[arXiv:2602.00005]. The manually crafted Boolean query still wins, "combining MeSH terms and diverse free-text
synonyms to maximise recall" — i.e. the human's edge is *breadth of vocabulary*, exactly what multi-query
fan-out buys. A reproducibility study found off-the-shelf LLM Boolean generation gives "significantly lower
recall" than originally claimed [arXiv:2505.07155], a caution against trusting one model-generated query.

**Implication for us:** Round-1 found verbatim single-query beats rewriting. SR evidence says that finding is
*task-local* — true for point lookups, but a synthesis task should issue a **fan-out of concept-decomposed
queries (one OR-bundle per sub-concept)** and union the chunk sets, not a single verbatim query.

## 2. Screening: high recall is cheap, precision collapses under imbalance

LLM abstract screening reliably hits **90–100% recall**. On a balanced Cochrane development set GPT-4o reached
sensitivity 0.911 / precision 0.818 / F1 0.862; on the *realistic* full corpus sensitivity stayed 0.756–1.000
but precision fell to **0.004–0.096** purely from class imbalance (eligible studies are a tiny fraction)
[Cochrane 23-review study, link.springer 13643-024-02609-x]. An environmental SR with ~12,000 records hit
**100% recall at the 0.5 cutoff** and saved >50% of screening time with zero false negatives; accepting 95%
recall raised work-saved to 75% [Environmental Evidence, 13750-025-00360-x]. Ensembles push further: parallel
LLM combinations held **perfect sensitivity at ~42% workload reduction** on a 119k-record corpus, and a series
configuration reached **99.13% workload reduction but only 69% sensitivity** — a direct recall/effort dial
[LLM ensembles, PMC12012331]. The cautionary counterpoint: **SESR-Eval** (2507.19027) found *no* LLM held high
recall *and* reasonable precision (best: GPT-4.1-mini at 0.60P/0.43R), and that "the choice of the secondary
study has more significance than the choice of LLM."

**Implication:** For our quote-first generation, the analogue of "high recall, low precision" is an over-broad
top-k feeding the model many irrelevant chunks. SR practice tolerates this *because a downstream filter
(the human, or here the abstention/quote-grounding contract) removes false positives*. So for synthesis, bias
toward **higher k and recall**, and lean on the already-proven abstention contract as the precision stage.

## 3. Stopping rules: the real research gap, and it's quantifiable

Active-learning SR tools (ASReview) report **WSS@95 of 67–92%** (95% of eligibles found after screening only
8–33% of records) [Nature Mach. Intel. s42256-020-00287-7]; later simulations span 63.9–91.7% WSS@95 and show
*simpler* models (Naive Bayes + TF-IDF, SVM + TF-IDF) often win [Syst Rev 13643-023-02257-7]. But **knowing when
to stop is unsolved**: a large evaluation found "almost all existing stopping methods either fail to reliably
stop without missing relevant records or fail to utilize the full potential work-savings." Statistically
principled rules exist — the **Target method guarantees ≥70% recall** by re-discovery of a sampled seed set;
**Quant/QuantCI** hit a *range* of recall targets via survey-estimation; **CMH** stops when P(target recall
reached) exceeds a confidence level [arXiv:2106.09871; 2311.08597]. A flexible statistical stopping rule still
delivered only ~17% work reduction when forced to *guarantee* recall [PMC7700715]. The recurring tension: more
searching gives more information about whether you're done, so stopping and recall are in direct opposition.

**Implication:** Our "number of searches / stopping rule" question is *exactly* the TAR stopping problem at
small scale. We can borrow their estimators: a **seed-rediscovery stopping rule** (keep issuing refined queries
until a held-out set of known-relevant chunks all re-surface) gives a cheap, auditable recall proxy.

## Actionable implications for librarian usage experiments

1. **Make query count a task-conditioned variable.** Test single verbatim query (Round-1 optimum) vs.
   concept-decomposed fan-out (N=3–6 OR-bundles, union top-k) on *synthesis* prompts; expect fan-out to win on
   coverage despite Round-1's single-query result for lookups.
2. **Measure recall against a gold chunk set, not just answer quality.** Borrow WSS@95 / Recall@K: annotate the
   truly-relevant chunks per synthesis question, then report recall at fixed retrieval budget.
3. **Bias synthesis retrieval to high recall, let the abstention contract be the precision stage** — mirroring
   SR's "screen broad, filter downstream." Validate that quote-grounding still drives hallucination to 0% at
   larger k (k=20+).
4. **Implement a seed-rediscovery stopping rule** for multi-search synthesis: stop refining when all seeded
   known-relevant chunks reappear (Target-method analogue, ≥70% recall guarantee).
5. **Test breadth-vs-depth refinement trajectories explicitly:** "spiral"/interleaved refinement beat staged
   pipelines by up to 90% on sparse corpora [PMC10792832] — relevant because our particle-physics collection is
   small and imbalanced.
6. **Prefer cheap retrieval where it suffices:** SR repeatedly finds simple TF-IDF/SVM matches dense models at
   95% recall; worth a librarian A/B of lexical-union vs. pure-dense retrieval per task type before assuming
   embeddings dominate.

## Sources
- Cochrane 23-review LLM screening — https://link.springer.com/article/10.1186/s13643-024-02609-x
- Environmental SR GPT screening — https://link.springer.com/article/10.1186/s13750-025-00360-x
- LLM ensembles (119k records) — https://pmc.ncbi.nlm.nih.gov/articles/PMC12012331/
- SESR-Eval — https://arxiv.org/pdf/2507.19027
- ASReview (Nature Mach. Intel.) — https://www.nature.com/articles/s42256-020-00287-7
- ATD simulation study — https://systematicreviewsjournal.biomedcentral.com/articles/10.1186/s13643-023-02257-7
- AutoBool (EACL 2026) — https://arxiv.org/abs/2602.00005
- Boolean query reassessment — https://arxiv.org/html/2505.07155v2
- Heuristic stopping rules (Quant/QuantCI) — https://arxiv.org/pdf/2106.09871
- Point-process stopping methods — https://arxiv.org/pdf/2311.08597
- Statistical stopping criteria — https://www.ncbi.nlm.nih.gov/pmc/articles/PMC7700715/
- Spiral ML workflow — https://www.ncbi.nlm.nih.gov/pmc/articles/PMC10792832/
