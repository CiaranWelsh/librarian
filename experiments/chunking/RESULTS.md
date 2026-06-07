# Chunking experiment series (issue 027) — results log

Prototype the best retrieval configuration with the **real LangChain splitters** before
porting the winner to Rust. Harness: `sweep.py` (chunk → embed `text-embedding-3-large`
3072-d → qdrant `eval_<name>`). Scored three ways, on the live daemon on turbo:

- `run_eval.py` (`golden_sample.json`, 15 covered Qs) — **source-level** hit/MRR + fragment-rate@5.
- `chunk_eval.py` (`golden_answers.json`) — **chunk-level**: is a retrieved chunk *answer-bearing*
  (right source + answer keyword + ≥ min_len chars)?
- `judge_eval.py` (`golden_answers.json`) — **LLM judge** (gpt-4o-mini): does the retrieved
  passage actually answer the question? Reads meaning → immune to length/keyword/source quirks.

Sample: 333 markdown chapters from 11 golden-referenced books. Compare configs against each
other (same sample), not against the full-corpus 89% baseline.

## Results

| config | chunks | median chars | src-MRR | frag@5 | chunk-recall@10 | chunk-MRR | judge answer@1 | judge hit@1 | judge hit@3 |
|---|---|---|---|---|---|---|---|---|---|
| **E0 blankline** (current) | 73,469 | 140 | 0.900 | 17% | 87% | 0.633 | 1.40 | 40% | 53% |
| rcts 800 | 31,630 | 645 | 0.861 | 15% | 100% | 0.611 | — | — | — |
| rcts 1200 | 19,916 | 1013 | 0.833 | 24% | 100% | 0.669 | — | — | — |
| rcts 1600 | 14,401 | 1403 | 0.856 | 24% | 93% | 0.676 | 1.47 | 47% | 87% |
| rcts 2000 | ~12k | ~1.6k | 1.000 | 24% | 93% | 0.810 | 1.73 | 73% | 87% |
| bc 1600 (breadcrumbs) | 17,252 | 1332 | 0.947 | 0% | 93% | 0.813 | 1.73 | 73% | 80% |
| **bc 2000 (breadcrumbs)** | 14,749 | 1525 | 0.942 | **0%** | 93% | **0.842** | **1.80** | **80%** | **93%** |

## The surprise and its resolution

1. **Surprise:** recursive chunking *lost* to blankline on source-MRR (0.90 vs 0.83–0.86).
2. **Diagnosis (D0):** blankline's rank-1 for "dependency inversion principle" was the literal
   heading `# DIP: The Dependency-Inversion Principle` — a content-free stub that lexically
   matches the query. **Source-level MRR rewards chunk *count* (heading lottery), not quality.**
3. **Reframe:** built a chunk-level answer-bearing metric, then an LLM judge that reads meaning.
4. **Resolution:** under both honest metrics the premise holds — blankline answers at rank-1 only
   **40%** of the time; **recursive + breadcrumbs ≈ 80%** (hit@3 53% → 93%). The "win" for tiny
   chunks was a metric artifact. Robust to the `min_len` floor (recursive leads even at floor 0).

## Conclusion — config to port (issue 027)

**Recursive splitter (RecursiveCharacterTextSplitter algorithm) + markdown-header breadcrumbs,
~2000 chars (~512 tokens), ~10% overlap.** Robust, field-standard, corroborated by four
independent metrics including the meaning-reading judge.

- **Robust claims:** recursive ≫ blankline; breadcrumbs eliminate fragments (frag@5 → 0%) and
  lift answer quality.
- **Below the noise floor** (15 Qs ≈ 7%/question): exact size 1600 vs 2000, and overlap 0/10/20%.
  Pick the field standard (~512 tok, 10% overlap); don't over-tune on this golden set.
- **Not adopted:** semantic chunking, propositions, late chunking (per FINDINGS.md).

## Limitations / next if more rigor wanted

- 15-question golden set is small; a larger set would let us fine-tune size/overlap meaningfully.
- Metric proxies (keyword/length) validated *post hoc* by the LLM judge agreeing with them.
- Orphaned `eval_A–D` collections (prior ad-hoc run) left in qdrant; clean up later.
