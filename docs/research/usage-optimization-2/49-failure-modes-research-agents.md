# Failure Taxonomies of Research/RAG Agents and Task-Conditioned Mitigation

**Scope:** Round-2 synthesis on *how* an assistant misuses a retrieval tool, framed for the librarian (private RAG over SE textbooks + physics papers, top-k chunks + confidence label). Round-1 settled single-query mechanics (verbatim query, k=20/k=8, quote-first, abstention contract: 12%->0% hallucination). This note covers the *trajectory-level* failure modes that abstention alone does not fix, and the evidence that optimal usage is **task-conditioned**.

## The named failure modes

The most-cited structural taxonomy is **MAST/MASFT** (Cemri et al., 2025): 1,600+ annotated traces across 7 frameworks, 14 failure modes, inter-annotator Cohen's kappa 0.88. Three top-level buckets: specification/design (41.8%), inter-agent misalignment (36.9%), and **verification/termination (21.3%)**. The last bucket contains the modes that map directly onto this round's question:

- **Premature termination (FM-3.1, ~6.2%)** — ending before enough information is gathered. Topology-dependent: it spikes in flat/star setups with no predefined workflow, i.e. exactly an assistant freely calling a query CLI.
- **No/incomplete verification (FM-3.3, ~8.2%) and incorrect verification (~9.1%)** — unchecked claims propagate. This is the structural face of **overconfident synthesis**.

A complementary *behavioral* taxonomy (Pan et al., 2025) names four archetypes that recur across model scale: **premature action** (acting before grounding — e.g. answering before inspecting the chunk), **over-helpfulness** (substituting fabricated entities rather than admitting a gap — overconfident synthesis), **context pollution** (distractors corrupting reasoning — source fixation's input side), and **fragile execution**.

**Citation drift / source fixation** has a mechanistic account in FACTUM (long-form RAG): "attributional drift," where the FFN/parametric pathway pushes a claim while the attention pathway fails to ground the citation, so a reference is emitted by parametric likelihood rather than verified context. Agentic-RAG SoK and **CHARM** add **cascading hallucination** — a four-type taxonomy where an early-step error amplifies across reasoning steps into a confident-but-wrong final output. Relevant to us: the librarian returns a confidence label, so drift here means citing a chunk that is retrieved but does not actually support the claim — distinct from, and not caught by, the abstention contract.

## Stopping is a distinct, measurable capability

**DeepSearchQA** (900 tasks, 17 fields) is built explicitly around stopping. It reports two opposing failure modes: **premature stopping / under-retrieval** (missing long-tail items) vs. **over-hedging** (casting a wide net of low-confidence answers to fake recall). SOTA gap is the "last mile": Gemini DR scores F1 81.9% but strict-correct only 66.1% (~15-pt gap); GPT-5 Pro 79.0% F1 vs 65.2% strict (~13 pt). Test-time compute moves strict-correct 67.2% (n=1) -> 85.7% (n=8), i.e. more sampling buys completeness. **BrowseComp-Plus** adds the key efficiency law: stronger retrievers both raise accuracy *and reduce the number of search iterations needed* — better retrieval substitutes for more loops.

Adaptive-RAG literature gives the stopping-rule design space: prompting-based (stop on final answer), confidence-based (FLARE: token-probability threshold; DRAGIN: entropy+attention), and trained controllers (Self-RAG reflection tokens; **Stop-RAG** and DeepRAG cast iterative RAG as a finite-horizon MDP). Adaptive-RAG's query classifier (no-retrieval / single / multi-step) is the cleanest task-conditioned precedent.

## Over-retrieval actively hurts — breadth is not free

More context is non-monotone. Distractors degrade reasoning: in a controlled benchmark, GPT-4.1 step-accuracy fell 26%->2% as irrelevant contexts went 1->15; Grok-3 43%->19%. "Lost in the middle" gives a U-shaped 30%+ drop. Two counterintuitive results matter for a *good* retriever like ours: **stronger retrievers surface more semantically-similar-but-irrelevant distractors, which are more damaging** (hard vs weak distractors, ~9-pt accuracy drop), and **context length hurts even with perfect retrieval** (Chroma "context rot," 18 frontier models). This is the empirical case against "always pull k=20."

## Task-conditioning: retrieval helps facts, hurts pure reasoning

The sharpest task-type evidence: on math, **static retrieval underperforms the no-retrieval baseline (-6.3pp GSM8K)**, while **adaptive on-demand retrieval beats it (+1.1pp GSM8K, +6.4pp MATH-500)**. Generic textbook corpora suit factual recall but not reasoning, which needs process-level signals (worked solutions), not entity definitions. TARG (training-free gating) matches always-RAG EM/F1 while **cutting retrieval 70-90%**. For multi-hop work, *external* features (question type, context relevance) predicted retrieval need better than the model's own uncertainty.

## Implications for librarian usage experiments

1. **Add a task-type pre-classifier as the primary experimental factor.** Hypothesis (well-supported): literature-synthesis and science/definition tasks reward broad multi-query retrieval; maths and design-reasoning tasks reward *minimal, on-demand* retrieval (verify a formula/API, then reason unaided). Test retrieve-always vs retrieve-on-demand vs never per task class — expect static retrieval to *lose* on the maths/reasoning arms.
2. **Instrument trajectory metrics, not just final answers.** Log search count, refinement trajectory, and the under-retrieval vs over-hedging split (DeepSearchQA style). The "last-mile" gap (F1 vs strict-correct) is the metric that exposes premature stopping.
3. **Test a confidence-gated stopping rule.** The librarian already emits a confidence label — wire it into a Stop-RAG/FLARE-style stop condition and benchmark fixed-k vs adaptive-stop on tokens, latency, and accuracy.
4. **Treat citation-grounding as a separate eval from hallucination.** Abstention killed unsupported *claims*; it does not catch a claim attributed to a retrieved-but-non-supporting chunk (FACTUM drift). Add a chunk-faithfulness/attribution check (atomic claim -> exact supporting span).
5. **Probe the strong-retriever distractor risk directly.** Because our retriever is good, sweep k (verify the round-1 k=8 value-point holds for reasoning tasks) and test reranking/quote-first ordering that places top chunks at head+tail to counter lost-in-the-middle and context rot.
6. **Prefer better single retrieval over more loops** (BrowseComp-Plus): measure whether improving chunking/breadcrumbs reduces needed iterations before adding multi-step orchestration.

## Sources

- Cemri et al., *Why Do Multi-Agent LLM Systems Fail?* (MAST/MASFT), arXiv:2503.13657
- Pan et al., *How Do LLMs Fail In Agentic Scenarios?* arXiv:2512.07497
- Vadlamudi, *Why AI Agents Fail: A Taxonomy of Failure Modes*, SSRN 6572478
- *FACTUM: Mechanistic Detection of Citation Hallucination in Long-Form RAG*, arXiv:2601.05866
- *CHARM: Cascading Hallucination in Agentic RAG*, arXiv:2606.04435
- *SoK: Agentic Retrieval-Augmented Generation*, arXiv:2603.07379
- *Attribution Techniques for Mitigating Hallucinated Information in RAG: A Survey*, arXiv:2601.19927
- *DeepSearchQA*, arXiv:2601.20975
- *BrowseComp-Plus*, arXiv:2508.06600
- *Stop-RAG: Value-Based Retrieval Control for Iterative RAG*, arXiv:2510.14337
- FLARE (Jiang et al.) and Self-RAG (Asai et al.); DRAGIN; Adaptive-RAG; DeepRAG
- *How Is LLM Reasoning Distracted by Irrelevant Context?* arXiv:2505.18761
- Liu et al., *Lost in the Middle*, TACL 2024; Chroma, *Context Rot* (2025); *Context Length Alone Hurts...*, arXiv:2510.05381
- *Do RAG Systems Really Suffer From Positional Bias?* arXiv:2505.15561; *Redefining Retrieval Evaluation in the Era of LLMs*, arXiv:2510.21440
- *Adaptive Retrieval helps Reasoning in LLMs -- but mostly if it's not used*, arXiv:2602.07213
- *TARG: Training-Free Adaptive Retrieval Gating*, arXiv:2511.09803; Adaptive-RAG (Jeong et al.); SKR; Probing-RAG
