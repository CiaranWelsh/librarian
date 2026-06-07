# STORM & Co-STORM: Outline-Driven, Multi-Perspective Retrieval for Long-Form Grounded Articles

**Sources:** Shao et al., "Assisting in Writing Wikipedia-like Articles From Scratch with Large Language Models," NAACL 2024 (arXiv:2402.14207); Jiang et al., "Into the Unknown Unknowns: Engaged Human Learning through Participation in Language Model Agent Conversations," EMNLP 2024 (arXiv:2408.15232, Co-STORM); `stanford-oval/storm` codebase / DeepWiki; OmniThink (arXiv:2501.09751) for WildSeek figures. Numbers below are quoted from these.

## 1. The thesis: research is a question-asking problem, and the win is at pre-writing

STORM's central claim is that the hard part of a grounded long-form article is **not the writing pass but the pre-writing pass** — researching the topic and building the outline. It decomposes pre-writing into three steps: (1) **discover diverse perspectives** by surveying related Wikipedia articles on the topic; (2) **simulate conversations** in which a writer holding each perspective interrogates a retrieval-grounded "topic expert," asking follow-ups as understanding updates; (3) **curate** the collected references into an outline, which then drives section-by-section generation with citations. This is directly relevant to our task-conditioned question: STORM is empirical evidence that, for **literature-synthesis-style work**, *who asks and how questions evolve* dominates single-query mechanics.

## 2. Measured gains (STORM, FreshWiki, N=100 articles, B-class+)

Outline quality (the pre-writing payoff) over an outline-driven RAG baseline (oRAG):

| Metric | Direct Gen | RAG/oRAG | STORM (GPT-3.5) | STORM (GPT-4) |
|---|---|---|---|---|
| Heading Soft Recall % | 80.23 | 73.59 | **86.26** | **92.73** |
| Heading Entity Recall % | 32.39 | 33.85 | **40.52** | **45.91** |

Full-article rubric (1–5): Coverage **4.88**, Coherence/Organization **4.82**, Relevance **4.45**, Interest **3.99**; ROUGE-1 45.82; citation recall **84.83%**, precision **85.18%**. Human Wikipedia editors rated **25% more** STORM articles "well-organized" and **10% more** "broad in coverage" than oRAG.

**Ablation is the load-bearing result for us.** Removing simulated conversation collapses references collected from **99.83 → 39.56** and soft recall to 77.97%; removing perspective-guided questioning drops references to **54.36** and entity recall to 40.12%. So **breadth of retrieval is driven almost entirely by multi-turn, multi-perspective questioning** — not by the final writer. Config: **N=5 perspectives × M=5 conversation rounds**, ~1 question/round; GPT-3.5 for questioning, GPT-4 for the article.

## 3. Co-STORM: turn-policy, a moderator for "unknown unknowns," and a stopping rule

Co-STORM turns the simulated conversation into a steerable multi-agent **round-table**: LLM experts (each a perspective) answer or raise follow-ups; a **moderator** injects thought-provoking questions **grounded on retrieved snippets that prior turns did not use** (the mechanism for surfacing *unknown unknowns*); a human may observe or inject. A `DiscourseManager` picks the next speaker. A **dynamic mind map** (hierarchical concept tree, each node holding the snippets + the question that retrieved them) reduces user cognitive load and **becomes the report outline** — node names are section headings. Reported human preference: **70% prefer Co-STORM over a search engine, 78% over a RAG chatbot**, with greater breadth/depth and less effort. Critically, the evaluation **terminates a session at 30 search queries** (for Co-STORM and both baselines) — an explicit, budget-based stopping rule. Reports are scored on a 4-axis rubric — **Relevance, Breadth, Depth, Novelty** — via Prometheus-2 (5-point). WildSeek: 6,608 unique topic–intent pairs from 8,777 users, downsampled to a **100-sample, 24-domain** benchmark of (topic, user-intent) pairs.

## 4. Actionable implications for task-conditioned librarian experiments

1. **Test a "perspectives" multiplier as the breadth lever, not query rewriting.** Round-1 killed rewriting/HyDE for *single* queries; STORM's ablation shows breadth instead comes from issuing several *perspective-conditioned* queries. Experiment: for a synthesis prompt, have Claude first enumerate 3–5 sub-perspectives, run our verbatim-query mechanic per perspective at k=8, and compare coverage vs. one k=20 query. Hypothesis: perspective-fan-out beats a single large-k pull for *breadth-bound* tasks.

2. **Make the task type select the trajectory shape.** STORM/Co-STORM (deep, ~25 queries, multi-turn) sits opposite PaperQA2's measured **1.26 searches/question**. This is the core Round-2 dial: literature synthesis = fan-out + iterate; a maths/API lookup = one verbatim query, stop. Design the experiment matrix so trajectory length is the *dependent* variable conditioned on task class.

3. **Adopt an explicit budget-based stopping rule and measure where breadth saturates.** Co-STORM caps at 30 queries; sweep our equivalent (e.g. 1, 3, 5, 10 queries/perspective) and plot unique-chunk / unique-source recall vs. budget to find the librarian's saturation knee per task class.

4. **Use the unused-retrieval trick to drive refinement.** Co-STORM's moderator questions are grounded on *retrieved-but-unused* chunks. Our top-k already returns more chunks than get cited; an experiment can feed the **uncited tail** of a k=20 pull back to Claude as the seed for the next query — a cheap, retrieval-grounded "what did I miss" step instead of free-form follow-ups.

5. **Build the outline/mind-map from retrieved breadcrumbs before writing, and grade outline quality separately.** STORM's biggest, most reliable gains are at the *outline* stage (heading soft/entity recall), not the prose. For writing-class tasks, instrument an intermediate outline artifact built from chunk breadcrumbs and grade it (heading entity recall analogue) independently of the final answer.

6. **Reuse the 4-axis report rubric (Relevance, Breadth, Depth, Novelty) as our task-conditioned eval, and weight axes by task.** Synthesis weights Breadth+Novelty; science/maths weights Relevance+Depth (correctness). This gives a per-task scoring function rather than one global quality score — the right shape for a task-conditioned study.
