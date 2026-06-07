# E1 — confidence separability & recalibration (results)

Experiment: `experiments/telemetry/tele_exp1.py`, run 2026-06-07 against the live daemon.
Validates the synthesis's priority experiment: do the cheap per-query signals we already
compute separate **answerable** (in-corpus) from **unanswerable** (off-domain/nonsense)
queries, and what are the corrected thresholds?

**Design.** Ground truth by construction — no LLM judge. Answerable = 91 golden + pilot
questions (answers are in the corpus), queried against their home collection. Unanswerable =
55 off-domain + nonsense queries (cooking, sport, gardening, gibberish — absent from a
physics+SWE corpus), each queried against both collections (110 samples). Each query hits the
daemon; we capture `top_score, margin, score_spread, fragment_rate, value, label`. Only cost
is ~200 query embeddings (a cent). Same embedder as production (`text-embedding-3-large`).

## Findings

**Per-signal AUROC (answerable vs unanswerable):**

| Signal | AUROC |
|---|---|
| `top_score` | **1.000** |
| `value` | **1.000** |
| `score_spread` | 0.817 |
| `neg_fragment_rate` | 0.693 |
| `margin` | 0.656 |

`top_score` and `value` separate the two classes perfectly: answerable top_score median
**0.736** [0.552, 0.821] vs unanswerable **0.245** [0.107, 0.485] — no overlap.

**The calibration bug, quantified.** Current `ConfidenceThresholds.no_answer_below = 0.25` is
far too low for this embedder: unanswerable queries score up to **0.485**. Youden-optimal
`top_score` threshold = **0.552** (TPR 1.00, FPR 0.00); for `value`, 0.517. Current-label
behaviour:

- answerable (91): 62 `weak`, 29 `strong`, 0 `likely_no_answer` — never wrongly abstains, but
  68% labelled `weak` when they are answerable (the "everything is weak" symptom).
- unanswerable (110): 58 `likely_no_answer`, **50 `weak`, 2 `strong`** — only 53% correctly
  flagged; 52 leak through as weak/strong (2 with false `strong` confidence).

ECE of `value` as P(answerable) = **0.237** — poorly calibrated as a probability.

## What this decides

1. **What to log:** `top_score` and `value` are the high-information fields (AUROC 1.0);
   `score_spread` is a useful secondary; `margin`/`fragment_rate` are weak alone. Log the full
   `score_vector` so thresholds can be re-fit offline.
2. **How to use it (the fix):** raise `no_answer_below` from **0.25 → ~0.50**. At ~0.50–0.55
   the answerable/unanswerable split is clean on this set. Our Tier-0 signal was already the
   right signal — only mis-thresholded.

## Caveats (honest)

- AUROC 1.0 is partly because the answerable set is golden/generated and shares corpus
  vocabulary (these queries are *easy*). Real user phrasings will score lower, narrowing the
  gap. **Recommendation:** adopt a conservative threshold (~0.45–0.50), and **re-fit on real
  logged scores** once Line A brings traffic (the calibration is the flywheel's job, not a
  one-time constant). Confirms the design: log the score_vector, recalibrate from logs.
- A handful of off-domain queries could weakly touch the corpus (e.g. trading-adjacent); the
  large margin makes the result robust to this label noise.

## Follow-ups (not yet run)
- E2 QPP-vs-golden correlation (limited by golden ceiling).
- E3 judge-vs-threshold agreement → Tier-2 sampling rate (needs the LLM judge).
- E4 gap-detection precision: drop a topic from the index, confirm the weak-query + overlap
  test flags it as a content gap. (Validates the acquisition loop end-to-end.)
