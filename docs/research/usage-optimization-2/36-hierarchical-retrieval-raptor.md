# Hierarchical / Summary-Tree Retrieval (RAPTOR) for Thematic & Global Questions

**Round:** Usage-optimization-2 (task-conditioned librarian usage)
**Scope:** When and how an AI assistant should reach for *hierarchical* (summary-tree) retrieval vs. our current *flat* top-k chunk retrieval, and what this implies for task-conditioned usage of the librarian.

## What RAPTOR is and the problem it targets

RAPTOR (Sarthi et al., ICLR 2024, arXiv:2401.18059) recursively **embeds → clusters → summarizes** chunks bottom-up, producing a tree where leaves are raw chunks and parents are LLM summaries of clusters. Query time uses a **"collapsed tree"**: all nodes (leaves *and* summaries) are flattened into one index and ranked by similarity under a token budget, rather than the rigid layer-by-layer "tree traversal". The motivating failure of flat RAG is structural: top-k of short contiguous chunks "limits their ability to represent and leverage large-scale discourse structure" — fatal for **thematic / global** questions like "what are the main themes?" or NarrativeQA's "how did Cinderella reach her happy ending?", where evidence is *distributed* across a document rather than localized in one chunk.

## Published numbers (the conditions matter as much as the deltas)

- **QuALITY (5k-token contexts, multiple-choice, whole-doc reasoning):** RAPTOR+GPT-4 = **82.6%** vs prior SOTA CoLISA 62.3% → **+20.3pts absolute**; QuALITY-HARD = **76.2%** vs 54.7% (+21.5pts). This is the headline "+20%".
- **Controlled retriever swap (same LM):** much smaller. RAPTOR vs DPR ≈ **+2.0pts**, vs BM25 ≈ **+5.1pts** on QuALITY. The 20pt figure is *RAPTOR + a stronger reader*; the *retrieval-only* gain is single-digit. Important caveat for us: the large number conflates retrieval and reader.
- **NarrativeQA (book-length, thematic):** METEOR 19.1% (new SOTA vs prior 10.6%); ROUGE-L 30.8% vs DPR 29.6%, BM25 23.5%.
- **QASPER (papers):** RAPTOR+GPT-4 = 55.7% F1 vs DPR 53.0%, BM25 50.2% (~+2.7pts).
- **Where the gain comes from:** Figure 7 shows **18.5–57% of retrieved nodes are non-leaf** (summary) nodes depending on dataset/retriever; the layer ablation hits **73.7% vs 57.9% leaf-only (+15.8pts)** on a thematic story, but story-by-story variance is wide (47–94%). Best config: **collapsed tree, ~2000-token budget (~20 nodes)**.
- **Build cost:** ~0.28 compression ratio (72% reduction), linear token/time scaling, tested to 78k-token docs; no entity/relation extraction (cheaper to index than GraphRAG).

## Boundary conditions — when hierarchy does NOT help

The gain is concentrated on **multi-hop / thematic / "integrate-across-document"** queries. For **single-hop factoids** the answer already sits in one leaf chunk; flat dense retrieval finds it and the summary layers contribute ~nothing while adding index cost. (No paper directly publishes "zero gain on factoids" — it is an inference from the layer-contribution analysis, where factoid answers come from the leaf layer alone.) Two further failure modes are documented: **summary fidelity loss** — over-abstract summaries drop the exact API/threshold/number, dangerous for code and detector specs — and **stale static trees** on dynamic corpora (pre-computed summaries don't update). Native hierarchy (our breadcrumbs/sections) beats *reconstructed* hierarchy: rebuilding a tree over arbitrary chunks "introduces noise" and is "less effective than native hierarchical structures."

## Comparison anchor: GraphRAG (Edge et al., arXiv:2404.16130)

For corpus-wide sensemaking ("main themes across the whole corpus"), GraphRAG's community summaries report **72–83% comprehensiveness win** and **62–82% diversity win** over vector RAG, and root-level summaries answer global queries at **9×–43× lower token cost** for repeated global queries. **Caveat:** these are LLM-as-judge wins; an audit (arXiv:2506.06331) found position bias (>30pt swings from answer ordering), length bias, and trial bias, collapsing some reported advantages (e.g. LightRAG 66.7% → 39.1%). Treat any LLM-judged comprehensiveness number as soft. Vector/flat RAG remains strongest on **directness** and specific lookups; on GraphRAG-Bench (textbook data) RAPTOR's tree actually topped graph methods — hierarchy aligns with naturally hierarchical content.

## Implications for task-conditioned librarian experiments

Our librarian is flat top-k (text-embedding-3-large, qdrant, breadcrumbs, k=20 best / k=8 value, quote-first, abstention). RAPTOR-style retrieval is a **task-conditioned upgrade**, not a global replacement. Concrete experiments:

1. **Build a synthetic-summary layer over existing chunks, keep leaves canonical.** Generate per-section and per-chapter summaries, embed them into the *same* qdrant collection (collapsed-tree style), tag node level. Leaf chunks remain the only citation source — summaries are *routing aids*, never quoted — directly mitigating fidelity loss for code/detector specs. Reuse native breadcrumbs as the tree skeleton rather than re-clustering (native > reconstructed).
2. **Split the eval set by query type.** Tag each test question as factoid / single-hop vs thematic-global / multi-hop. Hypothesis to test: summary nodes lift thematic recall by ~10–20pts while factoid accuracy is flat — quantify the crossover so the assistant knows *when* to opt in.
3. **Measure non-leaf node share retrieved per task type** (RAPTOR's Figure-7 metric). If summaries are rarely retrieved for coding/maths queries, that empirically justifies disabling the summary layer there and saving budget.
4. **Token-budget sweep, not just k.** RAPTOR's win is at a ~2000-token / ~20-node budget mixing granularities — re-run our k-sweep as a *budget* sweep with mixed leaf+summary nodes; compare to our flat k=20 baseline on thematic questions specifically.
5. **Define an abstain/escalate rule for global questions.** When a query is corpus-wide thematic ("what are the main approaches to X across the books"), flat top-k is structurally weak; the assistant should either route to summary nodes or *issue multiple decomposed searches* (cheaper than building GraphRAG). Test multi-query decomposition vs single-query as the task-conditioned strategy for global questions.
6. **Guard against LLM-judge bias** in any comprehensiveness eval: randomize answer order, control length, run multiple trials (per arXiv:2506.06331) so our "did it help" verdict is real.

## Sources

- Sarthi et al., *RAPTOR* — arXiv:2401.18059 / https://arxiv.org/html/2401.18059v1 (ICLR 2024)
- Edge et al., *From Local to Global: GraphRAG* — arXiv:2404.16130
- LLM-as-judge audit of GraphRAG-family evals — arXiv:2506.06331
- GraphRAG-Bench — arXiv:2506.02404 (RAPTOR tops graph methods on textbook data)
- HiRAG / HSG-RAG / ArchRAG (2025) — hierarchical-vs-flat gains and native-vs-reconstructed hierarchy finding
