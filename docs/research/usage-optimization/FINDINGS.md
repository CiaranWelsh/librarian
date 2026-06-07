# RAG Usage Optimization — research synthesis (20-agent web sweep, 2026-06-07)

How should an assistant USE the librarian for maximally informative, true context?
Synthesis of ~19 parallel research agents over the 2023-2026 literature + vendor guidance.
Full agent reports in the session transcript; key numbers and design implications below.

## What the evidence says (numbers are from the cited studies)

1. **Query rewriting is not a free win.** On strong embedders over in-domain corpora it is
   neutral-to-negative (FiQA nDCG@10 −9%, p<.001; helps only when it moves vocabulary
   *toward* corpus terms; gating is near-impossible, AUC 0.59) (arXiv:2603.13301,
   query2doc 2303.07678). → verbatim questions are the baseline to beat.
2. **HyDE helps weak/zero-shot retrievers; on strong in-domain embedders it matches or
   hurts**, esp. on niche jargon the generator doesn't know (2212.10496, ReDE-RF
   2410.21242: +5.3 nDCG over HyDE by judging real docs instead).
3. **Fan-out raises raw recall but end-to-end gains evaporate at equal context budget**
   behind truncation/rerank (Hit@10 0.510→0.448-0.478, n.s.; RRF k=60 is the standard
   merge) (2603.02153, Cormack 2009). Use N=3 sub-queries when used at all.
4. **k has an inverted-U.** Sweet spot 3-8 with mid-size chunks; recall keeps rising with k
   but answers peak then fall (distractor dilution; "context cliff" ~2.5k tokens; random
   noise can even help while *near-miss* passages hurt) (RAGGED 2403.09040, Power of Noise
   2401.14887, 2601.14123).
5. **Ordering: genuine literature tension.** Attention-bias work says best-at-edges (+15pp
   recoverable); the most realistic study (EMNLP 2025 2505.15561) finds ordering ≈ shuffle
   once real distractors are present — distractor *filtering* outranks ordering.
6. **Reranking is the highest-ROI production lever** when recall@pool ≫ precision@k
   (Anthropic: retrieval failures 5.7%→1.9% stacked; listwise LLM rerank ≈ +0.04 nDCG at
   ~1 call). Pattern: retrieve 30-50 → rerank → keep 5-10.
7. **Small-to-big wins at fixed budget**: fewer, expanded hits beat more flat chunks
   (RAGChecker 2408.08067); window ≈ ±3 sentences; over-expansion drops groundedness.
8. **Iterative retrieval must be gated and targeted**: retry only on a weak-result signal,
   reformulate using terms surfaced by round 1, *drop* weak round-1 chunks (CRAG 2401.15884,
   Adaptive-RAG 2403.14403: ~multi-hop accuracy at half the steps; single-hop hurt 9× slower).
9. **Insufficient context is worse than none**: error 10.2%→66.1% (Gemma) when weak context
   is added; LIKELY-NO-ANSWER should abstain, not degrade (Google sufficient-context
   2411.06037). Models adopt wrong context >60% of the time (ClashEval 2404.10198).
10. **Anthropic official guidance**: context first, question LAST (+30% on multi-doc);
    XML document tags; quote-extraction-first; contextual retrieval at ingest (−35-67%
    retrieval failures); top-20 retrieval; <200k-token corpora → skip RAG entirely.
11. **Grounded-answer prompting**: quote-then-answer; inline per-claim citations correlate
    strongly with low hallucination (r=−0.72, 2512.12117); explicit abstention contract
    ("Not found in the provided context") is the largest trust gain (TRUST-ALIGN 2409.11242);
    chain-of-note helps precisely under noisy retrieval (+7.9 EM, 2311.09210).
12. **Formatting**: numbered IDs + source metadata are load-bearing; XML-vs-markdown is
    ~null on frontier models (Finetune-RAG: flat 98.2% vs XML 97.0%); token-efficiency wins.
13. **Eval methodology**: 15 questions cannot rank ~10 arms — use ~50+, bootstrap CIs,
    declare ties on overlap; never let the generator judge itself (self-preference +10-25%);
    separate retrieval-axis from generation-axis metrics (MT-Bench 2306.05685, RAGAS, ARES).
14. **Closed-book control is mandatory** to measure *net help vs harm*; score
    Correct/Hallucinate/Abstain as first-class outcomes; hard distractors are the realistic
    threat model (Power of Noise; Stanford legal RAG: 17-33% hallucination *with* retrieval).
15. **Hybrid BM25+dense** is the fix for exact identifiers (registers, trait names) — an
    INDEX-side change (qdrant sparse vectors / miniCOIL), out of scope for usage experiments
    but the top librarian-feature candidate (BEIR 2104.08663; qdrant miniCOIL).
16. **Long-context vs RAG**: whole-chapter reads win for self-contained narrative questions
    under ~32k effective tokens; route by confidence (Self-Route −65% cost at LC accuracy).

## Design consequences for the experiment harness (v2)

- Add **closed-book control** (A0) — net-help measurement.
- Baseline = **verbatim query**; rewriting/HyDE as challengers expected to lose.
- **k-sweep {3,5,8,12,20}**; track context chars; expect peak 3-8.
- **Listwise LLM rerank arm** (30 → k*) — high prior.
- **Fan-out with RRF** (k=60) at equal budget — low prior.
- **Deep-read ±1500 chars, merged overlaps** — high prior per RAGChecker.
- **Quote-first generation arm** — high prior for faithfulness.
- **Gated retry**: only on weak Tier-0 label; targeted reformulation; drop weak chunks.
- All arms: passages first / question LAST, numbered IDs + book names, inline-citation
  requirement, explicit abstention contract.
- **~45 questions** (15 golden + generated single-hop/multi-hop + absent-topic controls),
  bootstrap CIs, cross-model judging (generator ≠ judge), retrieval vs generation axes
  reported separately, Correct/Hallucinate/Abstain outcomes.
