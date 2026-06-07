# Known-item lookup vs exploratory search (Marchionini): how strategies differ and how systems should adapt

## Scope

Marchionini's taxonomy is the standard lens for the task-conditioned question in
Round 2: *the same retrieval tool should be driven differently depending on whether
the assistant is doing fact-lookup or open-ended sense-making.* This note pulls the
canonical framework, the one controlled study that quantifies the behavioral gap, and
the system-side adaptation literature, then maps them onto the librarian.

## The taxonomy

Marchionini (2006, *CACM* 49(4):41-46, "Exploratory search: from finding to
understanding") splits information seeking into three overlapping activity classes:

- **Lookup (known-item search).** Requires "well-defined, unambiguous search queries to
  retrieve discrete, well-structured facts or specific documents." Precise goal, simple
  path, goal immediately attainable. This is what traditional ranked retrieval handles
  well.
- **Learn** and **Investigate.** The *core* of exploratory search. Ill-structured,
  open-ended needs that "necessitate the use of browsing strategies, multiple search
  iterations, and the assessment of many potentially relevant search results," and aim at
  the higher levels of Bloom's taxonomy (comprehension, synthesis, evaluation).

Key structural claims: the classes have subclasses, they **overlap in time, and movement
between them is non-linear** — a session bounces between lookup and learn. In his earlier
work the task is placed on continua of *unspecificity, volume, and timeliness*; lookup sits
at the low-unspecificity / low-volume end, exploration spans the rest. This builds directly
on **Bates' berrypicking model** (1989, *Online Review*): the need and the query *evolve
bit-by-bit*, each retrieved document reshaping the next query (footnote-chasing, scanning),
versus the "classic model" of one query → one matched result set. The headline tension:
"present-day web search engines have proven immensely powerful for lookup tasks, but
ill-suited for exploratory tasks."

## How behavior actually differs (the one quantitative study)

Athukorala et al. (2016, *JASIST* 67(11):2635-2651, "Is exploratory search different?")
is the only controlled study that directly contrasts the two — a recent survey (arXiv
2312.13695) notes only **5.9% of empirical exploratory-search studies even compare against
lookup.** Design: 6 tasks over a custom academic IR system (machine-learning literature),
3 lookup (fact-finding, navigational, question-answering) and 3 exploratory (comparison,
knowledge-acquisition, planning). Findings:

- **Most discriminative signals: query length, maximum scroll depth, task completion time.**
- Exploratory sessions show **longer queries, deeper scrolling, longer completion times**,
  and crucially spend **more time examining clicked documents than scanning result pages**
  (strongest in knowledge-acquisition). Lookup is dominated by SERP scanning and fast exit.
- A related design point (Athukorala et al. 2014): in exploratory tasks users want to scan
  **~33 items**, so the system shows **40 per result page** (vs the ~7 visible without
  scrolling) — i.e. exploration needs wider candidate sets.
- A Random Forest (10-fold CV) separated core lookup from core exploratory tasks at
  **~85% accuracy, 0.859 AUC**, and classified four exploratory subtypes at **84.75%.**
  Conclusion: task type is **detectable mid-session** from interaction signals, which is
  what makes adaptive IR feasible.

## How systems should adapt

White & Roth (2009, *Exploratory Search: Beyond the Query-Response Paradigm*) define
exploratory search as "open-ended, persistent, multifaceted" needs met by "opportunistic,
iterative, multitactical" processes, and argue the turn-taking query→response paradigm is
the wrong fit. Their feature taxonomy for exploratory support: **faceted navigation,
query/term suggestion, result diversification, visualization/overview, history and
trajectory tracking, and a symbiotic "guidance" relationship** that helps the user
traverse an unfamiliar landscape — rather than just ranking for one query. Lookup, by
contrast, is best served by *precision-first* single-query response and early stopping.

## Implications for librarian usage experiments

1. **Make task-type a controlled factor, not an afterthought.** Build a fixed task set
   labelled lookup (definition / API signature / single-threshold / locate-the-source) vs
   exploratory (synthesis / compare-methods / survey-a-subfield). Round 1 settled
   *single-query* mechanics; Round 2 should measure whether optimal *strategy* (number of
   searches, refinement, breadth/depth, stopping) differs by this label — the Athukorala
   result predicts it will, and predicts the gap is large enough to be detectable.

2. **Expect inverted optimal strategies, and test for them.** Hypotheses worth pre-registering:
   lookup is best served by **1-2 verbatim queries, narrow k (the k=8 value point), quote
   and stop on first confident hit**; exploration needs **multiple reformulated queries,
   wide k (k=20), breadth-first across distinct source_ids, then depth via `extract` on the
   2-3 best**. The librarian already exposes the right primitives: `query` for breadth,
   `extract` for depth — the experiment is *when to switch from one to the other.*

3. **Operationalize a task-conditioned stopping rule.** For lookup: stop at first
   above-confidence hit answering the question. For exploration: stop on **source
   saturation** (new queries return already-seen source_ids — the librarian analog of
   "scanning ~33 items" and diminishing-returns berrypicking). Measure cost (searches,
   tokens) vs answer quality to find the per-task-type budget knee.

4. **Borrow the detectability result for routing.** Athukorala shows task type is ~85%
   classifiable from cheap signals. For an autonomous assistant the analog is a lightweight
   pre-classifier (prompt-based or rule-based on the user's request) that picks the strategy
   profile before searching — and an experiment comparing fixed-strategy vs classified-strategy
   agents on mixed task batches.

5. **Don't assume linearity.** Marchionini and Bates both stress non-linear interleaving:
   a synthesis task contains embedded lookups. The harness should allow strategy to *shift
   within a session* (verbatim lookup sub-queries inside an exploratory trajectory), and the
   metrics should capture trajectory shape, not just totals.

6. **Watch the confound the survey flags.** Prior knowledge and task difficulty co-vary with
   exploratory-ness and were explicitly controlled by Athukorala. Control them here too
   (hold corpus and question difficulty fixed across the lookup/exploratory split) or the
   strategy effect will be unattributable.

## Sources

- Marchionini, G. (2006). Exploratory search: from finding to understanding. *CACM* 49(4):41-46. DOI 10.1145/1121949.1121979.
- Athukorala, K., Głowacka, D., Jacucci, G., Oulasvirta, A., Vreeken, J. (2016). Is exploratory search different? *JASIST* 67(11):2635-2651. DOI 10.1002/asi.23617.
- White, R. W., Roth, R. A. (2009). *Exploratory Search: Beyond the Query-Response Paradigm.* Morgan & Claypool.
- Bates, M. J. (1989). The design of browsing and berrypicking techniques for the online search interface. *Online Review* 13(5):407-424.
- "Unexplored Frontiers: A Review of Empirical Studies of Exploratory Search" (2023). arXiv:2312.13695 — notes only 5.9% of studies contrast lookup vs exploratory.
- Agarwal, M. K. et al. (2021). Lookup or Exploratory: What is Your Search Intent? arXiv:2110.04640 (intent-classification framing).
