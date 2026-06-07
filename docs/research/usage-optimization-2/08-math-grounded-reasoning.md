# Retrieval and Mathematical/Grounded Reasoning: When the Librarian Helps, When It Distracts

**Scope.** Task-conditioned usage question: for maths-heavy or physics-derivation work, when should an AI assistant query the librarian, what should it ask for (definitions/formulas/theorems vs. derivation steps), and what usage strategy (search count, breadth, stopping) is optimal. Findings below are drawn from the LLM-reasoning + RAG literature and translated into concrete experiment hooks for our setup (text-embedding-3-large, qdrant, top-k chunks, abstention contract).

## Key finding 1: Reasoning is not retrieval — but the *inputs* to reasoning are

The strongest theoretical anchor is Ruis et al. ("Procedural Knowledge in Pretraining Drives LLM Reasoning," 2024), an EK-FAC influence-function study over 5M pretraining docs for Command R 7B/35B. Their separation of factual vs. reasoning queries is decisive:

- For **factual** questions, the literal answer appeared in the top-500 most-influential documents **55% of the time (7B), 30% (35B)**. Factual influence was highly *volatile* — the model leans on a few specific documents (i.e. retrieval-like behaviour).
- For **reasoning** (arithmetic, slope, linear equations), the answer appeared in top docs only **~7% (7B, arithmetic only), 0% (35B)**. Influence was *spread thinly* across many documents that share the *procedure* (code/maths implementations), not the answer. Same-procedure queries correlated at **~0.9 Pearson R**; different procedures, no correlation.

Implication: reasoning draws on a **generalisable synthesis of procedural knowledge**, not lookup. Retrieving "how to do the derivation" rarely helps because the model already has the procedure; retrieving the *facts the procedure consumes* (a definition, a constant, a theorem statement, a detector-specific formula) can help because those are genuinely outside parametric knowledge. Notably, reasoning-influential sources were StackExchange/ArXiv/code (overrepresented 10x+), exactly the genre of our particle-physics corpus.

## Key finding 2: Retrieval on reasoning benchmarks often does nothing — or hurts

Multiple direct studies converge:

- "Adaptive Retrieval helps Reasoning in LLMs — but mostly if it's not used" (2026) on GSM8K and MATH-500: **static retrieval underperforms a plain CoT baseline**; under an adaptive policy, traces that *retrieved* did slightly worse than CoT while traces that *abstained from retrieving* did better. Retrieval frequency scaled with problem difficulty — i.e. the **decision to retrieve is itself a metacognitive difficulty signal**, and choosing *not* to retrieve correlates with success.
- "When Retrieval Succeeds and Fails" (Wang et al., 2510.09106): retrieval helps knowledge-intensive/specialised/current-fact tasks and degrades reasoning-heavy tasks where pathways must be constructed internally; hybrid tasks split by which component dominates.

## Key finding 3: Where retrieval *does* lift maths — supplying the static knowledge

When the bottleneck is a missing fact rather than missing reasoning, grounding helps, sometimes a lot:

- **KG-RAR** (Graph-Augmented Reasoning, 2503.01642): step-wise retrieval of theorems/definitions from a structured KG over PRM800K gave **+20.73% relative (Llama-3B, MATH-500)**, +15.22% (Llama-8B, MATH-500), +8.68% (Llama-8B, GSM8K). Crucially, **structured retrieval beat unstructured RAG over the same source**, and a post-retrieval *refine* step beat raw retrieved chunks — noise filtering mattered. Small models (Qwen-1.5B, Llama-1B) got *worse* on hard problems, so the benefit is capability-gated.
- Formalization/theorem-proving work repeatedly uses retrieval to anchor **definitions, lemmas, formulas** (the propositional "knowledge-that"), which reduces confabulation. A documented research-maths case: with no DB the model confidently produced a wrong algebraic-geometry argument; with a theorem-DB tool it retrieved the right results and corrected itself.

## Key finding 4: The distraction mechanism is real and quantified

- **Inverted-U with k** (ICLR 2025, Long-Context LLMs Meet RAG): more passages → higher recall but lower precision; **RAG accuracy falls below recall** — relevant info is present yet the model is misled by co-retrieved "hard negatives." Stronger retrievers can make degradation *worse* (they surface more plausible-but-irrelevant near-matches).
- Cuconasu et al. "The Power of Noise" (SIGIR'24): top-scoring-but-non-answer-bearing documents *hurt*; their sweet spot was **3–5 relevant docs** then stop. (Their headline that random noise can *help* is a quirk of short-answer QA and likely does not transfer to derivation work — treat as a warning that "semantically near" ≠ "useful," not as advice to inject noise.)

## Actionable implications for librarian usage experiments

1. **Condition retrieval on knowledge-type, not topic.** Test a router that retrieves for *definition / formula / theorem / detector-constant* sub-questions ("knowledge-that") and abstains for *derivation / algebra / computation* steps ("knowledge-how"). Predicted: retrieving derivation steps adds no accuracy and raises latency; retrieving the consumed facts does.
2. **Decompose then retrieve per-fact, don't bulk-retrieve the problem.** Per the KG-RAR step-wise result and the "treat query as one unit fails" critique, run targeted small queries for each missing fact rather than one query over the whole maths problem.
3. **Use a smaller k for maths than for synthesis.** Round-1 found k=20 best globally with k=8 a value point; the inverted-U + 3–5-doc sweet spot suggests maths-grounding wants **k≈3–8 with aggressive precision/rerank**, since hard negatives hurt derivations more than they hurt literature survey. Run a k-sweep stratified by task type.
4. **Make "no retrieval" a first-class outcome and measure it.** Treat retrieval frequency as a difficulty signal; log how often the assistant *declines* to query on maths and correlate with correctness (expect: declining = good, mirroring the adaptive-retrieval result). Extend the abstention contract so the model can abstain from *querying* as well as from answering.
5. **Verify the retrieved fact is consumed, not pasted.** The "improves comprehension, not knowledge" failure (and refine > raw in KG-RAR) implies a quote-first-then-use protocol: cite the retrieved definition/constant, then derive independently. Measure derivation correctness, not just citation presence.
6. **Pair with a computer/code tool for the arithmetic.** Hybrid RAG + code-interpreter gave +10–15pp in cross-domain reasoning; for any numeric Timepix/detector calculation, the librarian should supply the formula/constant and a tool should do the maths, not the LM.

## Sources
- Ruis et al., Procedural Knowledge in Pretraining Drives LLM Reasoning (2024) — https://lauraruis.github.io/2024/11/10/if.html
- Adaptive Retrieval helps Reasoning in LLMs — but mostly if it's not used (2026) — https://arxiv.org/pdf/2602.07213
- Wang et al., When Retrieval Succeeds and Fails (2025) — https://arxiv.org/pdf/2510.09106
- Wu et al., KG-RAR / Graph-Augmented Reasoning (2025) — https://arxiv.org/abs/2503.01642
- Long-Context LLMs Meet RAG (ICLR 2025) — https://arxiv.org/pdf/2410.05983
- Cuconasu et al., The Power of Noise (SIGIR'24) — https://arxiv.org/abs/2401.14887
- RaDeR: Reasoning-aware Dense Retrieval (2025) — https://arxiv.org/pdf/2505.18405
- Formal Mathematical Reasoning: A New Frontier in AI (2024) — https://arxiv.org/pdf/2412.16075
