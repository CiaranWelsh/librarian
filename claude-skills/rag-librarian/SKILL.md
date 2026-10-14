---
name: rag-librarian
description: >-
  Search the shared reference library (software-engineering and CS books, trading
  books and particle-physics / detector papers) with the `librarian` CLI. Use it
  whenever you need to ground a claim, cite a book or paper, or look up a
  definition, API, threshold, algorithm or method, and whenever the user asks
  "what does X say about Y" or wants references, even if they never say "search
  the library". Prefer it over answering reference questions from memory.
---

# RAG Librarian

The library is a vector index over real books and papers, served read-only at
`https://rag-librarian.com` and queried with the `librarian` CLI. Search it before you
assert anything a book would settle. A cited passage beats a confident paraphrase from
memory, and in the measured experiments retrieval took hallucination on in-corpus
questions from 12% to about 0% (see `references/evidence.md`).

If `librarian` is not installed, or `LIBRARIAN_DAEMON` and `LIBRARIAN_KEY` are unset,
read `references/setup.md` and walk the user through it first.

## Locate, read, report

**1. Locate.** Ask the library a question in plain language.

```
librarian query <collection> "<your question>" --limit 8
```

Collection names and sizes come from `librarian list`. Today they are `software`
(software engineering, CS, networking, GPU and parallel computing, ML, Rust, Qt,
TypeScript and detector manuals such as Timepix4), `particle-physics` (detector and
particle-physics papers, including Timepix, TPX3Cam, time-of-arrival and ion-trapping
work) and `trading` (quantitative and algorithmic trading, financial ML and systematic
investing).

Phrase the query as the question you want answered, not as a keyword list. Chunks are
embedded prose, so a question lands on the paragraph that explains the thing while
keywords land on the section heading that merely names it. Keyword rewrites lost to
verbatim questions on every metric measured. `--limit 8` is the value point (97% of the
quality of 20 hits at 38% of the tokens). Raising it widens breadth across sources,
which is not what you want when you need depth.

**2. Read.** Each hit is `[score] <source_id>#<chunk_index>` followed by the chunk.
Chunks are small, often a heading plus one paragraph, so a hit is a pointer rather than
a passage. Skim the hits to pick the best source, then read around it:

```
librarian extract <collection> "<source_id>#<index>" --context 5
```

Paste the hit token exactly as printed. `--context N` returns N chunks either side, in
order, which reconstructs the original passage. Read several hits before choosing, since
the top score is similarity, not usefulness. If every hit is a bare heading, the query
was too keyword-shaped. Rephrase it as a question.

**3. Report.** Cite the `source_id` for every claim you take from the library, so the
user can trace it. Pass on the confidence (below). If the passages do not contain the
answer, say the library does not cover it and stop. That abstention is the measured
0%-hallucination floor, and dressing thin evidence up as an answer is the failure the
library exists to prevent.

`references/example.md` walks through one real run of all three steps.

## Shape the search to the task

The right strategy depends on what you are doing. These were the measured winners (two
experimental rounds, re-validated with Claude as the generator; details and caveats in
`references/evidence.md`).

| Task | Do this | Why |
|---|---|---|
| Fact, API, definition | One search, verbatim question, `--limit 8`, quote first | Every arm tied; more hits only cost tokens |
| Answer spans two sources | Decompose into one search per sub-question, then combine | A single query starved and abstained on 88% of two-hop questions |
| Synthesis, survey, trade-offs | Three searches from different angles, `--limit 8` each, merge in your head | One `--limit 20` pull was the worst arm, through context dilution |
| Maths, theory, derivations | Write the one-sentence result you expect and search with that sentence | The only perfect cell; the question itself retrieves poorly |
| Explaining or teaching | Retrieve for grounding, then explain in your own words with an example, citing | Stitched quotes were the worst teaching pattern |

## Pass the confidence on

Every query ends with a line like

```
confidence: WEAK (0.68)  [top 0.677, margin 0.003, fragments 0%]
```

- `STRONG` means a close, distinguishable, substantial top hit. Cite normally.
- `WEAK` is the common case and is conservative. It fires when the top hits are
  near-tied or fragmentary, even when the answer is plainly there. Treat it as "verify
  by reading", not as "wrong".
- `LIKELY_NO_ANSWER` suggests the topic is outside the corpus. Low scores together with
  visibly off-topic text are the real signal; do not wait for the label.

Score-based confidence correlates only weakly with answer quality, so the label is a
prior and your reading of the text is the verdict. The user cannot see the retrieval, so
tell them how well grounded the answer is. "The library supports this solidly", "the
library only weakly supports this, treat it as a lead" or "the library does not appear
to cover this" are the three honest outcomes.

## Errors

- `401 unauthorized`. `LIBRARIAN_KEY` is missing, mistyped or revoked. Check the env var,
  then ask the operator.
- `404 not_found`. Wrong collection name. Run `librarian list`.
- `429 rate_limited`. Per-key limit of 60 requests a minute. Wait for `Retry-After` and
  batch your thinking, not your requests.
- Connection errors. The service or your network is down. Say so plainly rather than
  silently answering from memory.

Add `--json` to any command when you want to parse the output rather than read it.
