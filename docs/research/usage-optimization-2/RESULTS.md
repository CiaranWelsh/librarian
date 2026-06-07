# Issue 031 — Final results: how to use the librarian (rounds 1+2 + Claude validation)

Pipeline: literature research (20 + 50 Opus agents) -> exploratory experiments (round 1:
13 arms x 43 questions; round 2: 5 task types x 4 arms, 772 calls) -> winners re-validated
with Claude as generator. gen=gpt-4o-mini (validation: Claude), judge=gpt-4o, temp 0,
bootstrap CIs. Full tables: turbo `/data/books/.staging/usage_report.txt` +
`usage2_report.txt` + per-cell JSON in `usage_results/` + `usage2_results/`.

## Round 1 — single-query mechanics (settled)

- Retrieval's measured value is TRUTH: hallucination 12% -> ~0%; absent-topic abstention
  0/5 -> 5/5 (perfect, all retrieval arms). Quality also rose (1.28 -> 1.65 point est.).
- Winner: quote-first generation, verbatim question, k=20 (k=8 = value point: 97% of the
  quality at 38% of the tokens).
- Confirmed literature predictions on this corpus: keyword rewriting and HyDE LOSE to
  verbatim questions for general queries; equal-budget fan-out adds nothing; gpt-4o-mini
  listwise reranking HURT (added hallucinations); insufficient context is worse than none.

## Round 2 — task-conditioned strategy

Oracle task-routing beats the fixed round-1 pipeline by ~25% (qual 1.10 -> 1.38). A cheap
prompt classifier routes at only 42% (the router is the bottleneck) — but in conversation
the assistant itself is a ~perfect router (it knows its task). Per-task winners:

| Task | Winner | Evidence |
|---|---|---|
| LOOKUP | single search, k=8, quote-first | all arms tie; k=20 wastes 2.6x tokens |
| MULTIHOP | decompose-then-retrieve | single query abstained 88% (starved); 0.25 -> 0.75 |
| SYNTHESIS | fan-out 3 x k=8 (RRF) | single k=20 pull was WORST (0.88) — dilution |
| MATHS | HyDE: search with the target result statement | 2.00 [2.00,2.00] — only perfect cell |
| LEARNING | retrieve k=8, then elaborate (own words + example) | quote-first is the worst teacher (1.12 vs 1.75) |
| ABSENT | any retrieval | closed-book hallucinated 100%; retrieval abstained 5/5 |

## Claude validation (clean sweep)

| Cell | Claude | mini |
|---|---|---|
| MULTIHOP single -> decompose | 0.38 -> 0.88 | 0.25 -> 0.75 |
| MATHS verbatim -> HyDE | 1.75 -> 2.00 | 1.75 -> 2.00 |
| LEARNING quote -> elaborate | 1.38 -> 1.50 | 1.12 -> 1.38 |

All three reversals hold with Claude as generator (same judge, same references). The
recipe is generator-robust and is now encoded in the `cw:librarian` skill.

## Caveats

n=8/cell, overlapping CIs: directions validated (three confirmed twice, across
generators), magnitudes indicative. Some auto-generated questions were front-matter junk —
identical across arms, so comparisons stand. Judge ceiling ~80% human agreement bounds all
absolute numbers (see usage-optimization/FINDINGS.md #13).

## The deployed recipe (canonical copy lives in the cw:librarian skill)

Verbatim questions, --limit 8 default; decompose for two-source questions; 3-query fan-out
for synthesis (never one giant pull); HyDE for maths/theory; retrieve-then-elaborate for
teaching; abstention contract always (the 0%-hallucination floor); cite source_ids.
