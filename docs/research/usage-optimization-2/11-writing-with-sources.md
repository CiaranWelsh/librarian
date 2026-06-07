# Writing With Sources: Task-Conditioned RAG Strategy for Grounded Technical/Scientific Writing

## Scope

How should an AI assistant *use* a retrieval tool (our librarian) differently when the task is **writing grounded prose** (literature synthesis, a methods section, a design doc with citations) rather than answering a single factual question? Round-1 settled single-query mechanics (verbatim query, k=20, quote-first generation, abstention contract). Writing is fundamentally different: it is **long-form, multi-claim, and the information need evolves as the text unfolds**, so a single upfront retrieval cannot serve the whole document. This note synthesizes the published evidence on attribution patterns, retrieve-per-paragraph vs upfront, revision loops, and measured citation accuracy.

## Key findings

**1. Citation accuracy is the dominant failure mode, and it is severe.** Across long-form generation, roughly **half of statements are not fully supported by their cited sources** even with strong models: ALCE reports ~50% of ChatGPT/GPT-4 generations on ELI5 are not fully supported, and best-case citation recall/precision is only 84.8%/81.6% on ASQA, collapsing to 27.4%/28.5% on the harder QAMPARI ([Gao et al., ALCE](https://aclanthology.org/2023.emnlp-main.398.pdf)). In medical RAG, SourceCheckup finds **50–90% of citations in long-form answers are not fully supported** by the cited source even when a source is provided ([Attribution survey](https://arxiv.org/html/2508.15396v1)). Bibliographic *fabrication* (inventing references that don't exist) ranges **18–95%** depending on model and elicitation, and is essentially eliminated only by forcing citations to come from a retrieved, real corpus rather than parametric memory ([scientific-writing hallucination survey](https://arxiv.org/html/2510.06265v2); [LLM4SR](https://arxiv.org/pdf/2501.04306)). For the librarian this is the central design lever: writing must cite *retrieved chunks*, never recalled titles.

**2. Upfront vs retrieve-per-paragraph: interleave for long-form.** A single retrieval on the original prompt is sufficient for well-defined short answers but is "insufficient when generating long text whose information needs evolve as the answer unfolds" ([Jiang et al., FLARE](https://arxiv.org/pdf/2305.06983)). Active/interleaved retrieval reformulates the query from *both* the original prompt and the text generated so far, fetching evidence for the section currently being written. FLARE triggers retrieval when the upcoming sentence contains low-confidence tokens; Self-RAG trains "retrieve" and "critic" reflection tokens to decide per-segment ([RAG survey](https://arxiv.org/pdf/2312.10997)). Stuffing all evidence upfront also triggers "lost in the middle" — mid-context passages go unused ([Long-form QA study](https://arxiv.org/html/2310.12150)).

**3. But more retrieval is not monotonically better — stopping rules matter.** Iterative retrieval **converges in 2–3 iterations with diminishing returns past that**; extra loops add latency, cost, and *distracting evidence that degrades answers*, plus "query drift" (later queries wander from intent) and "retrieval laziness" (the model stops requesting evidence as context fills) ([Stop-RAG](https://arxiv.org/html/2510.14337v1)). Fixed iteration counts are the culprit: simple sub-claims waste loops, hard ones halt early. The corpus matters — on broad corpora where recall is the bottleneck, continued retrieval still wins. This is a per-*claim* (not per-document) tuning problem.

**4. Revision loops repair attribution but assume the evidence exists.** RARR (Retrofit Attribution using Research and Revision) is the canonical post-hoc loop: generate → ask questions about each claim → retrieve evidence → edit unsupported text while preserving style ([Attribution survey](https://arxiv.org/html/2508.15396v1)). It improves attributability without changing the base model but is expensive (many retriever calls, cascaded edits); PURR and latency-reduced verifier modules cut cost at similar quality. Per-claim verification pipelines (CiteAudit: Claim-Extractor → Retriever → Evidence-Matcher → Reasoner → Judge) are the emerging best-practice architecture, and one multilayer-QC review system drove hallucination **below 0.5%** ([CiteAudit](https://arxiv.org/pdf/2602.23452); [automated review system](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC12125968/)). Crucial caveat: revision repairs *unsupported* claims but cannot fix a *fabricated* citation — that requires grounding at generation time.

**5. Extract-then-write (quote-first) carries a real tradeoff.** Quote-first / snippet prompting improves grounding discipline but ALCE shows compressing passages to snippets *cost* 8.3 points of citation recall (73.6%→65.3% on ASQA) while letting the model see 10 passages instead of 5 and nudging correctness up (40.4%→41.4%). So compression trades citation fidelity for breadth — a tunable, not a free win.

## Implications for librarian usage experiments

1. **Test retrieval cadence as the primary IV.** Compare (a) single upfront k=20, (b) per-paragraph re-query using the just-written sentence as the FLARE-style query, (c) per-claim retrieval after a claim-extraction pass. Hypothesis from the literature: (b)/(c) beat (a) on multi-claim writing but converge by ~2–3 queries per section.
2. **Make claim-level citation recall/precision the headline metric** (ALCE-style NLI entailment of each sentence against its cited chunks), not document-level fluency. Report % unsupported statements — the field baseline is ~50%.
3. **Add a RARR-style revision pass as a separate condition** and measure its marginal lift over grounded-first-draft, plus its query budget. Expect repair of *unsupported* claims, not *fabricated* ones.
4. **Instrument a stopping rule** (confidence/value-based) and measure the over-retrieval penalty: at what point do extra librarian queries inject distractors that lower precision?
5. **Quantify the quote-first compression tradeoff on our chunks**: full-chunk context (fewer chunks) vs snippet/breadcrumb-only (more chunks) — does our breadcrumb metadata recover the recall ALCE lost to snippetting?
6. **Keep the round-1 abstention contract active throughout writing**: any sentence whose supporting chunk falls below the confidence label must be flagged uncited or dropped — this is what converts the 18–95% fabrication range to ~0%.

## Sources
- Gao et al., ALCE — citation recall/precision benchmark: https://aclanthology.org/2023.emnlp-main.398.pdf
- Attribution/Citation/Quotation survey (RARR, SourceCheckup, NLI metrics): https://arxiv.org/html/2508.15396v1
- Jiang et al., FLARE — active/forward-looking retrieval: https://arxiv.org/pdf/2305.06983
- Understanding Retrieval Augmentation for Long-Form QA (lost-in-the-middle): https://arxiv.org/html/2310.12150
- Stop-RAG — adaptive stopping, over-retrieval harms: https://arxiv.org/html/2510.14337v1
- RAG for LLMs: A Survey (FLARE/Self-RAG adaptive retrieval): https://arxiv.org/pdf/2312.10997
- LLM hallucination comprehensive survey (fabrication rates): https://arxiv.org/html/2510.06265v2
- LLM4SR — LLMs for scientific research/writing: https://arxiv.org/pdf/2501.04306
- CiteAudit — per-claim verification pipeline: https://arxiv.org/pdf/2602.23452
- Automated literature review-generation (<0.5% hallucination via QC): https://www.ncbi.nlm.nih.gov/pmc/articles/PMC12125968/
