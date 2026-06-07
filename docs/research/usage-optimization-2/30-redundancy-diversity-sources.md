# Redundancy, Diversity, and Sources-per-Claim for Task-Conditioned Librarian Usage

**Scope.** Round 1 settled single-query mechanics (verbatim query, k=20 best / k=8 value
point, quote-first generation, abstention contract). This note addresses the *task-conditioned*
question: how many **independent** sources should back a synthesised claim, and whether
diversity-aware retrieval (MMR / DPP) improves synthesis quality enough to justify the extra
machinery. The short answer: redundancy targets and diversity weighting are both real levers, but
their value is sharply conditioned on task type (synthesis/literature vs. lookup vs. reasoning).

## How many independent sources per claim

The corroboration and truth-discovery literature converges on a small set of operational rules.

- **Two-source minimum for verification.** Cross-source claim-verification systems treat a claim
  as unverifiable when fewer than 2 *independent* sources are available, and only mark it
  "Verified" once the agreement ratio across matching sources exceeds ~0.8; κ=0 is flagged
  "suspicious," intermediate values "disputed" ([Contradiction to Consensus, arXiv 2602.18693](https://arxiv.org/pdf/2602.18693)).
  This mirrors the journalistic "two-source rule" ([Number Analytics](https://www.numberanalytics.com/blog/power-of-corroboration-in-journalism)).
- **Counting is necessary but not sufficient.** Naive frequency/voting is consistently *outperformed*
  by methods that weight sources by quality, relevance, and prominence (TRUTHFINDER-style fixpoint
  iteration; Bayesian truth discovery) ([Marian & Wu, IEEE Data Eng. Bull.](http://sites.computer.org/debull/A11sept/p11.pdf);
  [Zhao et al., arXiv 1203.0058](https://arxiv.org/pdf/1203.0058)). Corroboration patents set explicit
  document-count thresholds (e.g. >=30 supporting docs) *plus* a containment check so that a more
  specific value ("Fred Mertz") beats a substring ("Fred") at equal support ([USPTO 8954412](https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/8954412)).
- **Independence must be enforced, not assumed.** Corroboration is meaningful only across genuinely
  independent sources; near-duplicates, shared authorship, and citation-graph proximity must be
  down-weighted (High/Medium/Low independence ratings) before counting agreement. Two chunks from
  the same book chapter are *one* source, not two — this is the single biggest pitfall for our
  breadcrumb-chunked corpus.
- **Preserve disagreement.** Newer pipelines surface source-level dispersion rather than collapsing
  to majority vote; high dispersion is itself an informative signal (uncertainty / contested claim)
  ([arXiv 2602.18693](https://arxiv.org/pdf/2602.18693)). For synthesis this argues for an
  "evidence-strength" label per claim rather than a binary supported/unsupported.

## Diversity-aware retrieval (MMR) and synthesis quality

MMR (Carbonell & Goldstein 1998) reranks a candidate pool with
`score = λ·rel(q,d) − (1−λ)·max sim(d, selected)`; λ=1 is pure relevance, λ=0 pure diversity.

- **The mechanism is exactly the redundancy problem.** Top-k dense retrieval over a single corpus
  tends to return near-duplicates, so the generator "sees the same fact five times instead of five
  distinct angles" ([Azure AI Search / ESPC](https://espc.tech/learning-hub/blog/enhancing-rag-with-maximum-marginal-relevance-mmr-in-azure-ai-search/)).
  MMR's documented benefit is recall of distinct subtopics and reduced redundancy in the context window.
- **Defaults and tuning.** Practitioner consensus and framework defaults sit at λ≈0.5 (LangChain
  `lambda_mult`) to 0.7 (relevance-leaning start), applied to a pre-fetched `fetch_k` pool for cost
  (MMR is O(k·n·d)). Effectiveness "varies by dataset and parameter tuning" — there is no universal
  optimum, so λ must be tuned per task and measured (RAGAS-style) ([controlled biomedical RAG benchmark, arXiv 2605.02520](https://arxiv.org/abs/2605.02520)).
- **Diversity helps synthesis, can hurt lookup.** Diversification is reported to most benefit
  multi-perspective queries (legal, regulatory, comparison, survey-style synthesis) where coverage
  of distinct angles matters. For pinpoint factual lookup, lowering λ risks pushing the one correct
  passage out of the budget — diversity is the wrong objective when the answer is a single fact.

## Task-conditions: depth and integration

Two findings strongly constrain *how much* to retrieve, by task:

- **More is not better, and the damage is task-shaped.** Appending more passages can degrade answers
  via positional bias and the U-shaped "lost in the middle" curve ([Liu et al.], summarised in
  [CARE-RAG, arXiv 2511.15994](https://arxiv.org/pdf/2511.15994)). Simple factual *retrieval* stays
  robust as depth grows; *aggregation, multi-hop, and uncertainty* tasks degrade fastest because
  they must integrate dispersed evidence that gets fragmented or buried. "Distractingly relevant"
  hard distractors (same entities/jargon as the truth) are worse than random noise.
- **Sampling-side analogue: independent paths saturate early.** Self-consistency gains are largest
  in the first 5–10 independent samples and flatten by 20–40; on frontier models gains are now
  marginal (0.4% HotpotQA, 1.6% MATH-500 over 20 samples) and can even *decline* at high counts
  ([arXiv 2511.00751](https://arxiv.org/html/2511.00751)). Sample complexity scales Θ(1/Δ²) in the
  answer margin Δ. This is the redundancy law in another guise: a handful of *independent* sources
  buys most of the reliability; piling on correlated ones adds cost and noise, not confidence.

## Actionable implications for librarian usage experiments

1. **Set a per-claim corroboration target of 2–3 independent sources, with dedup before counting.**
   Treat chunks sharing a breadcrumb root (same book/chapter, same paper) as one source. Measure
   synthesis faithfulness vs. the *deduplicated* independent-source count, not raw chunk count.
2. **A/B MMR vs. plain top-k, stratified by task.** Hypothesis: MMR (λ≈0.5–0.7 on a fetch_k≈40 pool)
   improves *synthesis/literature* coverage and reduces redundant quotes, but is neutral-to-harmful
   for *single-fact lookup* (definition, threshold, API). Report coverage of distinct sub-claims and
   answer correctness separately.
3. **Make retrieval depth task-conditioned, not fixed.** Keep k≈8–20 for lookup; for synthesis,
   prefer *broader-but-deduplicated* (high fetch_k → MMR → ~8 distinct sources) over deep top-k that
   buries evidence. Test for "lost in the middle" by permuting chunk order and checking answer stability.
4. **Emit an evidence-strength label per claim.** Map independent-source count + cross-source
   agreement to a 3-level label (corroborated / single-source / contested) and surface dispersion.
   This extends Round 1's abstention contract from "no evidence → abstain" to "weak/contested
   evidence → hedge," which should be measurable as reduced overconfident errors on uncertainty tasks.
5. **Cap redundant sampling.** If multi-query or re-retrieval is added, expect saturation by ~3–5
   *independent* retrievals; budget there rather than scaling blindly, and measure marginal coverage
   gain per added retrieval to locate the corpus-specific value point.
6. **Distinguish independence from duplication in the corpus itself.** Particle-physics papers cite
   and copy each other; software textbooks restate canonical patterns. Build an independence weight
   (author/citation/source-id overlap) so "five sources agree" cannot be five reprints of one claim.

**Caveats.** Most MMR-in-RAG evidence is practitioner blogs plus a few controlled benchmarks; the
diversity→synthesis-quality link is plausible and mechanistically grounded but not yet quantified for
a textbook+paper corpus. The corroboration thresholds (κ>0.8, 2-source minimum) come from web/news
fact-checking, not curated technical libraries, so treat the specific numbers as starting hypotheses
to recalibrate on our own queries.
