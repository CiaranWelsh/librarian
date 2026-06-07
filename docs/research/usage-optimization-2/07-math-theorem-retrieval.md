# Retrieval for Mathematical Work: Theorem/Definition Lookup, Premise Selection, Formula Search

**Scope.** Does retrieval help for *mathematical* work, and how should it be *queried*? This is the
task-conditioned question for the librarian: math is the clearest case where the corpus is not a
narrative to synthesize but a precise lemma store to *probe*, with high precision demands and a
query-formulation problem distinct from prose lookup. Findings below transfer to how an assistant
should query the librarian for definitions, theorems, thresholds, and algorithm statements.

## Does retrieval help? Yes, and the magnitude is task-dependent

- **End-to-end proving.** LeanDojo/ReProver is the canonical ablation: adding premise retrieval
  raises Pass@1 from **47.6% (no-retrieval baseline) to 51.2%** on the random split of the LeanDojo
  Benchmark (98,734 theorems from mathlib), and both beat GPT-4 at 29.0% (Yang et al., NeurIPS 2023,
  arXiv:2306.15626). The gain is real but modest end-to-end because retrieval is only one stage.
- **The gain is conditional on library dependence.** REAL-Prover (arXiv:2505.20613) reports retrieval
  helps far less on MiniF2F (high-school olympiad problems that rarely cite mathlib lemmas) than on
  college-level, library-dependent mathematics. **Implication: retrieval pays off exactly when the
  answer lives in the corpus and the task genuinely depends on it** — not for self-contained reasoning.
- **Retriever quality propagates.** A graph-augmented retriever (text embeddings + GNN over the
  state-premise/premise-premise dependency graph) beats the ReProver language-only retriever by **>25%
  across R@k/MRR** on LeanDojo (arXiv:2510.23637); a domain-tuned premise model lifts MiniF2F Pass@1
  from 28.28% to 30.74% (arXiv:2501.13959). Better retrieval → better proofs, prover loop held fixed.

## How is it queried? Query formulation dominates

Math retrieval has solved the query-formulation problem in ways directly relevant to the librarian:

1. **Two query modes.** *Proof-state* queries (the current goal as query, used by ReProver/LeanSearch-PS
   for premise selection at each step) vs. *natural-language* queries (a mathematician describing what
   they want, used by LeanSearch, Moogle, Lean Finder). The librarian's use case is the second.
2. **Informalization closes the modality gap.** LeanSearch (arXiv:2403.13310) translates each formal
   mathlib statement into an *informal* NL statement, embeds the pair, and matches NL queries against the
   NL side — because users query in prose, not formal syntax. The librarian already stores prose; the
   lesson is to **match the query register to the stored register.**
3. **The intent-mismatch finding (most actionable).** Lean Finder (arXiv:2510.15940, ICLR 2026) shows
   informalizations still **mismatch real user queries**; fine-tuning embeddings on synthesized queries
   that emulate actual mathematician intent (mined from Lean Zulip discussions) yields **>30% relative
   improvement over LeanSearch/Moogle and GPT-4o.** The query a user *would* type differs systematically
   from a clean restatement of the target — the gap is in *intent*, not vocabulary.
4. **Query augmentation helps in math** even though Round-1 found verbatim beats HyDE for prose.
   LeanSearch explicitly augments queries for context; ProofNet retrieves against the *generated*
   formalization ŷ rather than the raw question x because that was "significantly more performant"
   (arXiv:2302.12433). This is a notable divergence: in math, an intermediate generated artifact can be
   a better query key than the literal user text.
5. **Retrieve-against-output for in-context examples.** ProofBridge (arXiv:2510.15681): random
   in-context examples *raise* syntactic correctness but *degrade* semantic correctness; semantically
   *retrieved* examples lift semantic correctness **+23.77%**. CRAMF concept-definition retrieval over
   26k mathlib definitions gives up to **62.1% (avg 29.9%) relative** autoformalization gains
   (arXiv:2508.06931). Relevance of retrieved units, not their mere presence, is what helps.

## Formula/MathIR: structure vs. semantics, and hybrids win

For symbolic formula search (ARQMath Task 2), **structural** methods (operator-tree path tokens,
Approach0, Tangent-CFT) dominate, while **answer retrieval (Task 1) leans on text/semantic signals.**
BM25 remains a resilient first-stage baseline but lags neural methods on newer benchmarks (MIRB,
arXiv:2505.15585). The state of the art is **hybrid**: structural substructure matching fused with
dense semantic embeddings (SSEmb + Approach0 beats embedding-only by >5pp on P'@10/nDCG'@10 on
ARQMath-3, arXiv:2508.04162). The librarian, being prose-embedding only, should expect weakness on
*notation-exact* lookups and lean on natural-language descriptions of the math instead.

## Actionable implications for task-conditioned librarian experiments

1. **Gate retrieval on corpus-dependence, not task type.** Add a "does the answer live in the corpus?"
   pre-check; expect large gains for definition/threshold/lemma lookup, near-zero for self-contained
   derivation. Measure abstention rate vs. true corpus coverage as the headline math metric.
2. **Test query register and intent rewriting.** Run an A/B/C: (A) verbatim user phrasing, (B) Claude
   rewrites the query as a *target-statement* ("the lemma that says…"), (C) retrieve against a *drafted
   answer* (HyDE-for-math / ProofNet ŷ trick). Round-1 killed HyDE for prose; math is the predicted
   *exception* — this is the highest-value experiment.
3. **Definition/theorem lookup is precision-first, so favor depth over breadth.** A single well-formed
   query at k=8 with quote-first generation likely suffices; the refinement trajectory is "reformulate
   the query," not "issue more parallel queries." Test stopping rule: stop when top chunk is an exact
   definitional match (high confidence label), else reformulate once toward the canonical name.
4. **Exploit canonical naming.** Math terms have stable names (e.g., "Cauchy-Schwarz", "timewalk
   correction"); test whether injecting the canonical term into the query beats descriptive paraphrase,
   mirroring Loogle/`exact?` name-based search alongside semantic search.
5. **Concept-before-claim retrieval.** Per CRAMF, retrieve the *definition* of a concept first, then the
   theorem that uses it — a two-hop trajectory worth testing against single-shot retrieval for layered
   lookups (define detector quantity → find the correction formula that uses it).
6. **Expect a structural blind spot.** For notation-heavy or formula-exact queries the prose embedding
   will underperform; the experiment should log query type (conceptual vs. symbolic) and report
   accuracy split by type, so we know when to tell the assistant to fall back to grep/name search.
