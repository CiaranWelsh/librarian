# How Human Researchers Actually Conduct Literature Reviews

*Round-2 input for task-conditioned librarian usage. Round 1 settled single-query mechanics (verbatim > rewrite/HyDE, k=20 best, k=8 value point, quote-first generation, abstention contract 12%->0% hallucination). This note asks how the **process** of literature work differs from a single lookup, and what that implies for usage strategy.*

## 1. Information-seeking is iterative, not one-shot — the foundational models

The dominant theoretical finding, replicated across three independent models, is that real literature work is an **evolving, non-linear, multi-query trajectory**, not a single query matched against a corpus.

- **Bates' berrypicking (1989).** Explicitly rejects the "single query -> single output set" IR paradigm. Two claims: (a) the information need *itself shifts* as the searcher reads, so queries continually reformulate; (b) relevant material is gathered "bit at a time" from scattered sources, never as one ideal retrieved set. Searchers move along a path of query (Q) -> document -> new thought (T) -> new query. They mix techniques: keyword search, **citation chasing**, area scanning, browsing. ~414 citations by 2014. ([Bates 1989 PDF](https://pages.gseis.ucla.edu/faculty/bates/articles/berrypicking.pdf); [Hearst Ch.3](https://searchuserinterfaces.com/book/sui_ch3_models_of_information_seeking.html))
- **Ellis (1989/1993) behavioural model.** Six *features* (not stages, deliberately not sequenced): **starting, chaining** (backward+forward citation following), **browsing, differentiating** (filtering by known source quality), **monitoring** (current awareness), **extracting** (selectively mining one source). Later Meho & Tibbo (2003) added accessing, networking, verifying, information-managing from a 60-scholar study; confirmed activities are "not entirely or always sequential." Key: these are *different kinds* of act — chaining/browsing are search procedures, differentiating is filtering, extracting is an action on a source. ([Ellis overview](https://ebooks.inflibnet.ac.in/lisp15/chapter/models-of-information-seeking-behavior/); [Meho & Tibbo 2003](https://onlinelibrary.wiley.com/doi/full/10.1002/asi.10244))
- **Kuhlthau's ISP (1991).** Six stages (initiation, selection, exploration, formulation, collection, presentation) across affective/cognitive/physical realms. Its load-bearing finding is the **uncertainty principle**: information *increases* uncertainty early (exploration) before a *focus-formulation turning point* reduces it. Implication: early-stage searching is exploratory and broad; precise targeted retrieval only becomes possible *after* a focus forms. The high-uncertainty zone is where intervention helps most. ([Kuhlthau ISP](https://wp.comminfo.rutgers.edu/ckuhlthau/information-search-process/))

**Design takeaway:** an AI doing literature synthesis should not model the task as one verbatim query. It should model a *trajectory* — broad exploratory queries first (high uncertainty, terms unsettled), narrowing/refining after a focus emerges, plus a distinct citation-chasing mode.

## 2. Keyword search vs citation chasing — both needed, hybrid wins

This is the most directly actionable empirical block for tool strategy.

- Keyword/database search alone is **surprisingly weak on recall**: Scopus alone found only **13–35%** of relevant papers in software-engineering SLRs; Scopus + ACM together **23–60%**. Failure mode: terminology variance — relevant work using different words is invisible to a term query. ([Hybrid strategies, arXiv 2004.09741](https://arxiv.org/pdf/2004.09741))
- Citation chasing (snowballing) can dominate: one study found snowballing identified **83%** of papers vs **46%** for database search; its recall hinges entirely on a good **start set**. ([Badampudi/Wohlin](https://dl.acm.org/doi/10.1145/2745802.2745818))
- **Hybrid is the consensus winner:** one iteration of backward+forward snowballing *on top of* database search gives **90–100% recall**. Relying on a single method "can lead to a set of included papers which misrepresents the research field." ([arXiv 2004.09741](https://arxiv.org/pdf/2004.09741))
- Trade-off: forward snowballing has higher precision, database search slightly higher recall; balance per goal. ([Felizardo et al.](https://www-di.inf.puc-rio.br/~kalinowski/publications/FelizardoMKSV16.pdf))

**Design takeaway:** the librarian today only does keyword/semantic retrieval (one half of the hybrid). For synthesis tasks, a single semantic query is the 13–60%-recall mode. A *chaining* capability — following the breadcrumbs/citations in returned chunks to fetch neighbouring chunks/sources — is the missing high-recall lever and the obvious experimental variable.

## 3. How many searches, how many sources, where time goes

- **Queries per session are few but reformulation is routine.** Web/academic logs: mean ~**2.3 queries/session**; **46–52%** of users reformulate; **~29–37%** of sessions issue **3+ queries**. A 39M-query academic log studied failure modes (null queries/sessions). ([Hearst Ch.6](https://searchuserinterfaces.com/book/sui_ch6_reformulation.html); [academic search failures](https://www.sciencedirect.com/science/article/abs/pii/S0306457316304071))
- **Tool choice & start point:** in a PhD-student study, **82%** started with Google/Google Scholar, only 18% with library databases; 45% used both. ([QUT study](https://www.sciencedirect.com/science/article/abs/pii/S0099133311000711))
- **Effort is enormous and front-loaded on search+screen.** Systematic reviews: ~**18 months** calendar, **~1,000–1,140 person-hours** median (range 216–2,518). Median **63** full texts screened (max 4,385). Screening ~**100–200 records/hr/screener** (title/abstract) but far slower careful. Librarian search-strategy work alone ~**27 hrs**; "search strategy development and translation" is the single largest librarian time sink. ([PROSPERO analysis](https://pmc.ncbi.nlm.nih.gov/articles/PMC5337708/); [librarian time](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC5886502/))

**Design takeaway:** humans run *few queries but reformulate*, and the cost is in screening/triage, not in query count. An AI with cheap queries should invert this: many cheap retrievals + aggressive automated triage. Per-task query *budgets* (1 for a fact lookup vs many for synthesis) are the natural experimental knob.

## 4. Reading is layered triage, with explicit stopping rules

- **Keshav three-pass:** Pass 1 skim (5–10 min: title/abstract/intro/headings/conclusion -> keep or drop), Pass 2 (~1 hr: figures, grasp content), Pass 3 (4+ hr: reconstruct/critique). Most papers never get past Pass 1. ([Keshav](https://www.adarshsalagame.com/post/how-to-read-a-paper))
- **Satisficing:** readers set a relevance threshold and skip any unit below it; time concentrates at paragraph/page/doc starts. ([Scim, arXiv 2205.04561](https://arxiv.org/pdf/2205.04561))
- **Stopping rules are explicit:** (a) **saturation** — stop when new sources yield no new concepts; (b) **effort-bounded** — top-N results only; practical rule "stop when I have enough / content repeats." ([software-eng review guidance](https://arxiv.org/pdf/2004.09741))

**Design takeaway:** map the librarian's `k` and confidence label onto a three-pass discipline — cheap top-k skim to triage which chunks deserve quote-extraction, with a saturation stopping rule (stop issuing refinement queries when new queries return already-seen chunks).

## Actionable implications for task-conditioned experiments on the librarian

1. **Model synthesis as a trajectory, not a query.** Compare single-verbatim-query (Round-1 optimum) vs a *berrypicking loop* (broad query -> read -> reformulate -> repeat) for literature-synthesis tasks. Measure recall of a known relevant chunk set, not just answer quality.
2. **Add and test a chaining mode.** Follow breadcrumbs/citations in returned chunks to pull neighbouring chunks. Hypothesis from §2: semantic-only ≈ 13–60% recall; semantic + one chaining iteration -> 90–100%. This is the single highest-value experiment.
3. **Set per-task query budgets and stopping rules.** Fact/maths/code lookups: 1–2 queries, high-confidence-or-abstain (Round-1 contract). Synthesis/learning: multi-query with a **saturation** stop (halt when a new query returns >X% already-seen chunks). Measure cost vs marginal recall.
4. **Front-load broad, narrow after focus (Kuhlthau).** For exploratory synthesis, start with low-precision broad queries (large k), tighten k and terms once a focus forms. Test whether an early-broad/late-narrow schedule beats fixed k=20.
5. **Three-pass triage to control context.** Use cheap top-k retrieval as Pass-1 skim to *rank* chunks, only quote-extract (Round-1 quote-first) from those above a satisficing threshold — directly addresses the "never read something that blows context" constraint.
6. **Measure where the AI's effort goes.** Humans spend hours screening, not querying. Instrument the librarian loop to see whether AI cost is in retrieval or in chunk triage; optimise the dominant term per task type.
