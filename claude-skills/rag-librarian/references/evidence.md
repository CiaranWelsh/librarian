# Where the recipe comes from

The strategy table in SKILL.md is not folklore. It came out of two rounds of controlled
experiments run against this library in June 2026 (in the repository under
`docs/research/usage-optimization/FINDINGS.md` and
`docs/research/usage-optimization-2/RESULTS.md`), preceded by a literature sweep of
about 70 research agents over the 2023 to 2026 RAG literature. Read this file when you
want to know how much weight a rule carries, or when a situation does not fit the table.

## How it was measured

Round 1 ran 13 strategy arms over 43 questions, covering single-query mechanics only.
Round 2 ran 5 task types by 4 arms (772 calls). The generator was gpt-4o-mini and the
judge gpt-4o at temperature 0, with bootstrap confidence intervals. The three round-2
reversals were then re-run with Claude as the generator, same judge, same references,
and all three held. Outcomes were scored as Correct, Hallucinate or Abstain, with a
closed-book control arm so that net help versus harm could be measured.

## What was found

**Retrieval's value is truth, not polish.** Closed-book answering hallucinated on 12% of
in-corpus questions and on 100% of questions about topics absent from the corpus. Every
retrieval arm took the first to about 0% and abstained on all five absent-topic
questions. Answer quality also rose (1.28 to 1.65 on the judge's scale), but the
hallucination floor is the headline.

**Verbatim questions are the baseline to beat, and nothing beat them for general
queries.** Keyword rewriting and HyDE both lost to the verbatim question in round 1.
The literature predicted this for strong in-domain embedders, where rewriting is
neutral to negative (FiQA nDCG@10 minus 9%, arXiv:2603.13301).

**`--limit 8` is the value point.** k=20 scored highest in round 1 but k=8 delivered 97%
of that quality at 38% of the tokens. For lookups every arm tied and k=20 simply cost
2.6 times the tokens. The literature's inverted U (RAGGED, arXiv:2403.09040) shows up
here as context dilution once near-miss passages crowd the window.

**Two-source questions need decomposition.** A single query over a question whose
answer spans two sources abstained 88% of the time because neither half retrieved
cleanly. Decomposing into one search per sub-question moved the score from 0.25 to 0.75
with gpt-4o-mini and from 0.38 to 0.88 with Claude.

**Synthesis wants three angles, not one big pull.** Three searches at k=8 merged with
reciprocal rank fusion won. A single k=20 pull was the worst arm (0.88), diluted by
near-miss passages.

**Maths and theory want HyDE.** Searching with a one-sentence statement of the expected
result, rather than with the question, was the only perfect cell (2.00 with a
[2.00, 2.00] interval, both generators). This is the one place HyDE wins, because the
result statement shares vocabulary with the proof or derivation while the question does
not.

**Teaching wants elaboration.** Quote-first generation, the best pattern for lookups,
was the worst teacher (1.12 against 1.75). Retrieving at k=8 and then explaining in your
own words with an example, citing as you go, won for both generators.

## How much to trust the numbers

n=8 per cell with overlapping confidence intervals. The directions are validated (three
of them confirmed twice, across generators). The magnitudes are indicative only. Some
auto-generated questions were front-matter junk, identical across arms, so comparisons
stand but absolute scores are depressed. The judge agrees with humans about 80% of the
time, which bounds every absolute number above.

## The confidence line

`confidence: LABEL (value)  [top, margin, fragments]` is computed by the server from the
top-k cosine scores it already has, with no LLM call. `top` is the best hit's cosine
score, `margin` the gap to the second hit and `fragments` the share of the top five hits
that are bare headings or stubs. The value is top closeness discounted by the fragment
rate.

For dense retrieval, score-based quality prediction correlates with true answer quality
at only r of roughly 0.2 to 0.4 (Faggioli et al., arXiv:2302.09947; see
`docs/research/rag-quality/FINDINGS.md`). That is why the label is a triage signal and
not a grade. In particular `WEAK` fires on a tiny margin even when the top hit is
excellent, which happens routinely when a book is indexed twice or a topic spans
adjacent chunks. `LIKELY_NO_ANSWER` is tied to an absolute score floor that this
embedder rarely crosses, so a low score together with visibly off-topic text is the
dependable out-of-corpus signal.

## Things the experiments did not settle

Reranking with gpt-4o-mini hurt (it added hallucinations), so no rerank step is in the
recipe, but a stronger reranker was not tried. Exact identifiers such as register names
or trait names retrieve poorly with dense vectors alone. Hybrid sparse plus dense
retrieval is the known fix and is an index-side change, not a usage one. If an
identifier lookup fails, search for the surrounding concept instead.
