# Deep Research Products: Iterative Loops, Source Budgets, and Task-Conditioned Usage

**Scope.** How OpenAI Deep Research, Gemini Deep Research, Perplexity Deep Research, and Anthropic's Research feature structure the search→read→reason→synthesize loop, what published numbers exist on search/source budgets and effort scaling, and what this implies for designing *task-conditioned* usage experiments on the librarian (a private RAG over textbooks/papers queried by Claude via a top-k CLI).

## The common architecture

All four products converge on the same shape: an **agentic loop** that *plans → retrieves → reads → reasons about what is missing → retrieves again*, terminating on a model-judged sufficiency condition, then a **separate synthesis pass** and (for Anthropic/OpenAI) a **dedicated citation pass**. Retrieval and reasoning are interleaved, not separate modules — the model decides *when* to search and *what* to search next based on interim findings ([Anthropic](https://www.anthropic.com/engineering/multi-agent-research-system); [Gemini](https://gemini.google/overview/deep-research/); [Perplexity](https://www.perplexity.ai/hub/blog/introducing-perplexity-deep-research)). This is a ReAct-style plan-act-observe orchestration with explicit backtracking from dead ends (paywalls, irrelevant hits) ([PromptLayer](https://blog.promptlayer.com/how-deep-research-works/)).

Two design philosophies split the field:
- **Single-agent end-to-end RL loop** (OpenAI): o3 trained with reinforcement learning on real browsing tasks, learning *when to search, when to read, how to combine* — one reasoning trajectory, end-to-end optimizable ([OpenAI](https://openai.com/index/introducing-deep-research/)).
- **Orchestrator-worker multi-agent** (Anthropic, and the documented general pattern): a lead agent plans, spawns parallel subagents with separate context windows, then synthesizes ([Anthropic](https://www.anthropic.com/engineering/multi-agent-research-system)).

## Published numbers

**Anthropic (the most quantified primary source):**
- Agents use **~4× the tokens** of chat; multi-agent systems use **~15× the tokens** of chat.
- **Token usage alone explains 80% of performance variance** on BrowseComp; number of tool calls and model choice explain most of the remaining ~15%.
- Multi-agent (Opus 4 lead + Sonnet 4 subagents) **outperformed single-agent Opus 4 by 90.2%** on their internal research eval.
- **Explicit effort-scaling rules embedded in the orchestrator prompt:** simple fact-finding → **1 agent, 3–10 tool calls**; direct comparisons → **2–4 subagents, 10–15 calls each**; complex research → **10+ subagents with divided responsibilities**. Without these rules, agents "spawned 50 subagents for simple queries."
- Parallelism (3–5 subagents at once; 3+ tools in parallel per subagent) **cut research time up to 90%**. A separate **CitationAgent** does attribution after synthesis.
- Multi-agent *helps* for breadth-first work exceeding one context window; *hurts* for shared-context, high-dependency work — "most coding tasks involve fewer truly parallelizable tasks than research."

**OpenAI:** o3-based, RL-trained for browsing; **26.6% on Humanity's Last Exam** (vs ~9% for o1/R1) and **67.4 on GAIA**; runs 5–30 min ([OpenAI](https://openai.com/index/introducing-deep-research/); [Fortune](https://fortune.com/2025/02/12/openai-deepresearch-humanity-last-exam/)).

**Gemini:** built on 2.5 Pro; standard runs **~20–30 search iterations**, "Max" runs significantly more; **3–15 min** latency; planning produces sub-questions deliberately including counter-arguments/conflicting evidence; performs **multiple self-critique passes** before finalizing ([Gemini](https://gemini.google/overview/deep-research/); [MindStudio](https://www.mindstudio.ai/blog/google-gemini-deep-research-max-api)).

**Perplexity:** official messaging — "**dozens of searches**, **hundreds of sources**"; third-party estimates range **3–5** (conservative) to **20–50** queries and **200+** sources (treat as directional, not official) ([Perplexity](https://www.perplexity.ai/hub/blog/introducing-perplexity-deep-research)). Standard search retrieves 60+ sources shallowly for speed; Deep Research trades speed for depth.

## Task-conditioned signal

The clearest published task conditioning is Anthropic's **effort-tiering by query type** (fact-finding vs comparison vs open-ended) and their **breadth-vs-depth rule**: parallel multi-agent breadth wins on retrieval/exploration; single-context depth wins where reasoning steps depend on each other (coding, proofs). Gemini and Perplexity differentiate only on a *depth dial* (standard vs Max/Pro: more iterations, more sources, more self-critique) rather than on task *kind*. The survey ([arXiv 2506.18096](https://arxiv.org/pdf/2506.18096)) frames the axis as **static vs dynamic planning** and **single- vs multi-agent**, and notes domain-specialized agents (scientific vs web vs writing) but does not publish per-task budgets.

## Implications for librarian usage experiments

1. **Adopt explicit effort tiers as the primary experimental factor.** Mirror Anthropic's three-tier rule on the librarian: factoid → **1 query, k=8**; comparison/definition-with-context → **2–4 queries, k=20**; synthesis/literature → **many queries (iterative), k=20**, with a refinement trajectory. This is the cheapest high-leverage manipulation and is directly grounded in production practice.

2. **Treat "number of queries × k" as a budget knob and measure its variance contribution.** Anthropic's finding that *token usage explains 80% of variance* predicts that, for synthesis tasks, **breadth of retrieval dominates clever query rewriting** — consistent with Round-1's result that verbatim beats HyDE. Test whether added marginal queries (not better single queries) drive answer quality on synthesis vs factoid.

3. **Implement a sufficiency-based stopping rule and benchmark it against fixed budgets.** Products stop when the model judges coverage adequate; with the librarian's confidence label already returned per chunk, a natural stop condition is *"new query yields no high-confidence chunks not already retrieved."* Compare against fixed-N to find where adaptivity pays off (likely synthesis > factoid).

4. **Map breadth-vs-depth to task type, not just one depth dial.** For *learning* and *writing*, breadth (diverse chunks, multiple sources) likely helps; for *maths/proofs* and *coding*, depth on a single canonical source likely helps and extra parallel queries add noise. Design the matrix so breadth and depth are separable factors.

5. **Add a synthesis/citation pass as a separate stage and measure abstention interaction.** All four products separate retrieval from a final synthesis+citation pass. Test whether Round-1's quote-first + abstention contract composes with a multi-query gather: does a verify-then-write pass over a *larger* retrieved set keep hallucination at 0% while raising recall?

6. **Cost-gate the high-effort tier.** The 15× token multiplier means the synthesis tier is only worth it for high-value queries. Record cost-per-task so experiments report a quality/cost frontier per task type, not just quality.
