# Routing by task/question complexity at the strategy level (Adaptive-RAG and beyond): classifiers deciding no-retrieval / single / iterative, and measured savings

## Scope

This note covers the literature that makes the *strategy-level* routing decision explicit:
given a query, choose **how much retrieval machinery to spend** — none, one shot, or an
iterative loop — rather than always running a fixed pipeline. This is the direct
mechanization of Round 2's task-conditioned thesis: a single tool (the librarian) should be
*driven differently per task*, and the routing literature quantifies both the savings and the
failure mode (the router is the bottleneck). Round 1 settled single-query mechanics; this is
the layer above it — *how many queries, with what loop, for what kind of question.*

## The canonical framework: Adaptive-RAG

Jeong et al. (NAACL 2024, arXiv:2403.14403) is the reference point. A small classifier
(FLAN-T5-XL) labels each query into one of three classes and routes accordingly:

- **A — no retrieval** (answer from parametric knowledge),
- **B — single-step retrieval** (one retrieve-then-read),
- **C — multi-step / iterative retrieval** (interleaved retrieve-reason loop, IRCoT-style).

The headline efficiency/accuracy tradeoff (FLAN-T5-XL, averaged):

| Strategy        | Steps/query | Time/query | F1     |
|-----------------|-------------|------------|--------|
| No retrieval    | 0.00        | 0.11 s     | 21.12  |
| Single-step     | 1.00        | ~1.00 s    | 44.31  |
| **Adaptive-RAG**| **1.08**    | **1.46 s** | **46.94** |
| Multi-step      | 4.69        | 8.81 s     | 48.85  |

The point: Adaptive-RAG buys **~96% of multi-step's F1 at ~23% of its steps and ~17% of its
time** — it spends the expensive iterative loop only on the queries that need it. On GPT-3.5
its F1 on complex queries (50.91) matches always-multi-step (50.87) while collapsing cost on
the easy majority.

**How labels were made (this matters for our experiments).** There is no gold complexity
dataset, so silver labels are bootstrapped two ways: (1) **outcome-based** — run the three
strategies and assign the *cheapest* strategy that answers correctly (A if no-retrieval is
right, else B, else C); (2) **dataset-bias fallback** — unlabeled single-hop queries (SQuAD,
NaturalQuestions, TriviaQA) default to B; unlabeled multi-hop queries (MuSiQue, HotpotQA,
2WikiMultiHopQA) default to C.

**The router is the ceiling.** The trained classifier reaches only **~54% three-way accuracy**
(its own Fig. 3 confusion matrix shows heavy A↔B↔C bleed). The paper's **oracle classifier**
(perfect routing) is both *more accurate and cheaper* than the learned one — the entire gap
between real and oracle is router error, not strategy error. The authors flag this explicitly
as the main limitation and the main lever for future work.

## The design space the router can use

The "when to retrieve / how much" signal has been operationalized several ways; these are the
candidate router *inputs*:

- **Query-complexity classifier (Adaptive-RAG).** Cheap, pre-retrieval, one decision per
  query. Brittle: silver labels, ~54% acc, sensitive to phrasing (see below).
- **Entity popularity threshold (Mallen et al., ACL 2023, "When Not to Trust Language
  Models", arXiv:2212.10511).** Retrieve only below a per-relation popularity threshold;
  parametric memory handles the popular head. Robustly beats always-retrieve on PopQA and cuts
  inference cost — but needs popularity scores that don't exist for arbitrary corpora.
- **Generation-confidence / self-knowledge (during generation).** FLARE (Jiang et al., EMNLP
  2023, arXiv:2305.06983) triggers a retrieval when the next predicted sentence contains
  low-probability tokens (threshold θ≈0.8); it ends up retrieving for only **~30–60% of
  sentences** vs every-sentence baselines. SeaKR (ACL 2025, arXiv:2406.19215) reads
  *internal-state* uncertainty rather than output probability to decide both *when* to
  retrieve and *which* snippet to keep, beating FLARE/DRAGIN while being tuning-free.
- **Dual cost/accuracy classifiers with a user knob** ("Fast or Better?", arXiv:2502.12145):
  one classifier trained to favor accuracy, one to favor efficiency, exposing the speed/quality
  tradeoff as a deployable control rather than a fixed policy — a critique that pure-complexity
  routing gives the operator no dial.

## Known weaknesses (pre-register against these)

1. **Silver labels are circular** — "complexity" is defined by *which strategy happened to
   succeed on this model*, so labels encode the base model's gaps, not intrinsic difficulty.
2. **Phrasing brittleness** — semantically-equivalent human rewrites raise "query loss" and
   destabilize the retrieval trajectory, degrading accuracy without changing true complexity
   (arXiv:2604.10745). The router reacts to surface form.
3. **Three classes is coarse** — real requests interleave lookup inside synthesis (the
   non-linearity Marchionini/Bates flag in note 33); one label per query mis-serves mixed
   sessions.

## Implications for librarian usage experiments

1. **Build the three-arm harness directly.** The librarian already exposes the primitives: arm
   A = answer from parametric knowledge (no `query`); arm B = one verbatim `query` (Round 1's
   settled k≈8–20, quote-first); arm C = an iterative loop (`query` → reason → reformulate /
   `extract`, with a stopping rule). Adaptive-RAG's table is the template — log **searches/query,
   wall-time/query, tokens, and answer quality** per arm so we can reproduce the steps-vs-F1
   knee on our own corpus.

2. **Establish the oracle ceiling first, then the router.** Adaptive-RAG's key lesson is that
   *the router, not the strategies, is the bottleneck.* Run all three arms on a labelled task
   batch to get the **oracle-routed** cost/quality point; only then measure how close a cheap
   pre-classifier (prompt-based on the user's request, or a rule on task type from note 33)
   gets to it. Report the oracle gap as the headline metric.

3. **Route on task type, not just "complexity."** Our split is richer than single/multi-hop:
   *definition/API-lookup → arm B-narrow; maths/derivation → parametric-first with
   arm-B verification (note 08); literature synthesis → arm C breadth-first; learning →
   arm B with `extract` depth.* Map task category → strategy arm and test whether
   task-type routing beats Adaptive-RAG-style complexity routing on a mixed batch.

4. **Borrow popularity/self-knowledge as the no-retrieval gate.** Mallen's result — skip
   retrieval for head knowledge — is the principled basis for arm A. The librarian analog:
   abstain-or-answer-from-memory for well-known software/physics facts, retrieve for the tail.
   Test a confidence gate (model's own calibrated confidence, FLARE-style) as the A↔B trigger
   and measure how often it correctly skips a search without losing the Round-1 0% hallucination
   guarantee.

5. **Make C's stopping rule explicit and measured.** Iterative retrieval's cost lives in the
   loop (4.69 steps, 8.81 s in Adaptive-RAG's worst arm). Pair arm C with a source-saturation
   stop (note 20/33: new queries return already-seen source_ids) and uncertainty-drop stop
   (SeaKR: keep iterating only while retrieval reduces uncertainty). Report cost saved vs quality
   lost.

6. **Control phrasing as a factor, not a confound.** Given the rewrite-brittleness result, run
   each task in 2–3 paraphrases and measure routing *stability*. A router that flips arms on
   paraphrase is unsafe for an autonomous assistant; verbatim-query robustness (a Round-1 win)
   should extend to the routing layer.

## Sources

- Jeong, S. et al. (2024). Adaptive-RAG: Learning to Adapt Retrieval-Augmented LLMs through Question Complexity. NAACL 2024. arXiv:2403.14403 / aclanthology.org/2024.naacl-long.389. (Three-arm router; ~54% classifier acc; oracle > learned; 1.08 steps / 1.46 s / 46.94 F1 vs 4.69 / 8.81 / 48.85 for always-multi-step.)
- Mallen, A. et al. (2023). When Not to Trust Language Models: Investigating Effectiveness of Parametric and Non-Parametric Memories. ACL 2023. arXiv:2212.10511. (Popularity-threshold adaptive retrieval; skip retrieval on head knowledge; cuts cost on PopQA.)
- Jiang, Z. et al. (2023). Active Retrieval Augmented Generation (FLARE). EMNLP 2023. arXiv:2305.06983. (Confidence-token trigger θ≈0.8; retrieves for ~30–60% of sentences.)
- Asai, A. et al. (2023). Self-RAG: Learning to Retrieve, Generate, and Critique through Self-Reflection. arXiv:2310.11511. (On-demand retrieval via reflection tokens; per-segment retrieve/skip; inference-time tunable threshold.)
- Yao, Z. et al. (2024/2025). SeaKR: Self-aware Knowledge Retrieval for Adaptive RAG. ACL 2025. arXiv:2406.19215. (Internal-state uncertainty for when-to-retrieve and snippet selection; tuning-free; beats FLARE/DRAGIN.)
- "Fast or Better? Balancing Accuracy and Cost in RAG with Flexible User Control" (2025). arXiv:2502.12145. (Dual accuracy/efficiency classifiers + user knob; critiques complexity-only routing for offering no operator control.)
- "How You Ask Matters! Adaptive RAG Robustness to Query Variations" (2026). arXiv:2604.10745. (Human paraphrases raise query loss, destabilize retrieval trajectory, degrade accuracy — routing is phrasing-brittle.)
