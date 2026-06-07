# Breadth-First Survey vs Depth-First Drill-Down: Retrieval Ordering for Research Tasks

**Scope.** Round-1 settled *single-query* mechanics on the librarian (verbatim query > rewrite/HyDE, k=20 best with k=8 as the value point, quote-first generation, abstention contract 12%->0%). This note surveys the *multi-search* layer: given a task, how should an assistant sequence and stop a series of librarian calls — broad survey of many facets first, vs deep drill-down on one thread first — and what does the evidence say about the explore/exploit ordering.

## 1. The uninformed-search baseline maps cleanly onto retrieval

BFS explores all facets at one "depth" before descending; DFS follows one thread to exhaustion before backtracking (Wikipedia; Codecademy). The classic trade-offs transfer directly to retrieval. DFS is memory-cheap and wins when the answer lies deep along a *known* good branch, but "may get lost in an infinite branch"; BFS is complete and "finds high-quality pages faster since important content typically sits closer to the homepage" — the documented reason web crawlers prefer breadth for general indexing and reserve depth for focused/narrow topics (Firecrawl). The load-bearing caveat: these are *uninformed* — "they just explore without guidance" — so any real retrieval strategy is BFS/DFS **plus a relevance heuristic** (information scent). This is exactly the librarian's confidence label playing the heuristic role.

## 2. Human information-seeking: ordering is a stable individual/task trait

The exploration-exploitation framing of search is grounded in **information foraging** (Pirolli & Card): a seeker following information scent trades off exploring new patches (unknown reward) against exploiting the current one (known reward) (academia.edu; eye-tracking study, NCBI PMC5112274). Crucially, *ordering* is empirically bimodal: **"analytic searchers focus on exploitation first, interspersed with bouts of exploration, whereas wholistic searchers prefer to explore the search space first and consume later"** — i.e. depth-first vs breadth-first as a cognitive style, modulated by perceived risk and task difficulty. Domain knowledge matters: prior knowledge (low vs high) shifts the explore/exploit balance and the choice between thematic vs navigational strategies (web-search task-complexity study). Implication: the *right* ordering is task- and knowledge-conditioned, not universal.

## 3. The stopping rule has a normative form: Marginal Value Theorem

When to stop drilling (leave a patch) has a clean optimal rule. Charnov's **Marginal Value Theorem**: leave the current patch "when the marginal rate of return declines to the average for the environment," formally R'(t\*) = R(t\*)/(t\*+τ) (emergentmind; Nature s41598-017-11763-3). Two predictions matter for us: (a) within-patch returns *diminish* (the gain curve flattens), so over-drilling one query wastes budget; (b) **longer travel time -> longer optimal residence** — when issuing a new librarian query is "expensive" (latency, context budget, reformulation effort), it is optimal to drill deeper before switching facets. MVT extends empirically to human *memory search and information retrieval* in semantic space (emergentmind), giving the librarian a principled stop signal: stop drilling a facet when its marginal new-information rate falls below the average across open facets.

## 4. Agentic-RAG evidence: adaptive control beats fixed breadth/depth

Iterative RAG makes the search count *emergent*, not preset, and the open problem is the stopping rule. **Stop-RAG** (arXiv 2510.14337) frames iterative retrieval as a finite-horizon MDP and learns a value-based controller; it "consistently outperforms both fixed-iteration baselines and prompting-based stopping with LLMs," because each extra loop "increases latency, costs, and the risk of introducing distracting evidence." Self-assessment prompts and fixed iteration counts are both called out as weak. **Agentic-R** (arXiv 2601.11888) cuts average search turns ~10% on HotpotQA and ~15% on TriviaQA by extracting more per retrieval — depth-efficiency, not just more breadth.

## 5. Query decomposition IS the breadth-vs-depth dial — with measured crossover

The most on-point work, **"Query Decomposition for RAG: Balancing Exploration-Exploitation"** (arXiv 2510.18633), defines breadth = exploring multiple sub-queries (facets), depth = retrieving more docs per sub-query, and allocates a fixed budget via **Thompson sampling**: observe one doc, update belief, then exploit (same sub-query) or explore (switch). Hierarchical/correlated bandits beat flat decomposition **only at small budgets (10-20%)** — e.g. a "24% performance boost" and precision 0.303 -> 0.401 at 10% budget — and "gains converge as budget increases." So **breadth-first pays off precisely when budget is tight; under generous budget the ordering stops mattering.** This complements EfficientRAG (arXiv 2408.04259): decomposition's ~20-chunk recall matched direct retrieval's ~200-chunk recall, but DMQR-RAG/MQRF caution that **naive splitting can underperform the original verbatim query** — consistent with Round-1's verbatim-wins finding.

## 6. The depth ceiling: over-retrieval actively degrades answers

Breadth and depth both hit a hard wall: **"lost in the middle"** (Liu et al., TACL 2024) — accuracy drops >30% when the relevant chunk sits mid-context, and "when the total number of documents retrieved increases, accuracy goes down," replicated across six model families. The decomposition paper independently confirms "longer contexts can degrade downstream performance." This bounds *both* strategies: dumping all k=20 chunks from many sub-queries into one context is counterproductive. Favors quote-first extraction per search (Round-1) + reranking before synthesis over raw accumulation.

## Actionable implications for librarian usage experiments

1. **Make ordering an explicit task-conditioned factor.** Test breadth-first (one librarian pass across N facets, shallow) vs depth-first (drill one thread, backtrack on miss) per task type — predicting breadth-first wins for *literature synthesis* (coverage/gap-mapping, per scoping-review methodology: broad sweep, then thematic depth) and depth-first wins for *maths/debugging* (single deep dependency chain).
2. **Operationalize an MVT stopping rule and benchmark it against fixed-count.** "Stop drilling a facet when its marginal new-information rate (new non-duplicate chunks above the confidence threshold) falls below the running average across open facets." Compare to fixed-iteration and to Stop-RAG-style learned stopping.
3. **Tie residence time to query cost.** Per MVT, deliberately vary the per-query "travel cost" (latency / reformulation effort) and measure whether optimal drill-depth rises — calibrates how aggressively the assistant should reuse vs re-issue queries.
4. **Budget-conditioned hypothesis.** Per 2510.18633, expect breadth-first ordering to help most under *tight* total-chunk budgets and to wash out under generous budgets — test at 10%/20%/100%-equivalent chunk budgets to locate the crossover for our corpus.
5. **Guard the depth ceiling.** Cap chunks fed to synthesis well below naive N×k; force quote-first extraction + dedup/rerank between search and generation, and measure "lost-in-the-middle" degradation as breadth grows.
6. **Condition on assistant's prior knowledge.** Like domain-knowledge effects in human search, test whether the model should drill-first when it already has a strong hypothesis (exploit) vs survey-first on unfamiliar topics (explore) — a learnable router on the confidence label.

### Sources
- BFS/DFS trade-offs: Wikipedia (Breadth-first search); Codecademy BFS-vs-DFS; Firecrawl glossary (breadth vs depth crawling).
- Information foraging / explore-exploit ordering: academia.edu (Exploration & exploitation during information search); NCBI PMC5112274 (eye-gaze risk/ambiguity); web-search task-complexity & domain-knowledge study.
- Marginal Value Theorem: emergentmind (MVT topic); Nature s41598-017-11763-3 (social foraging follows MVT); Charnov 1976 (via above).
- Agentic-RAG stopping: arXiv 2510.14337 (Stop-RAG); arXiv 2601.11888 (Agentic-R).
- Decomposition as explore-exploit: arXiv 2510.18633 (Thompson-sampling budget allocation); arXiv 2408.04259 (EfficientRAG); arXiv 2411.13154 (DMQR-RAG, verbatim can win).
- Over-retrieval degradation: Liu et al. TACL 2024 / arXiv (Lost in the Middle).
