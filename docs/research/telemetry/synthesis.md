# Telemetry synthesis → design input for librarian (issue 033)

Synthesis of the 50 telemetry/usage-data reports in this folder, mapped onto our system:
a stateless axum query daemon (ADR-0005) over qdrant + OpenAI `text-embedding-3-large`,
a thin CLI/API front (no web SERP), ≤25 keyed users over a Cloudflare tunnel, an existing
per-query Tier-0 confidence signal, Tier-1 LLM judge, Tier-2 golden-set health, generation
on the free Anthropic subscription, paid OpenAI embeddings kept minimal, copyrighted corpus.

The literature is overwhelmingly built around a **trace → span → score** model (Langfuse /
LangSmith / Phoenix / OTel GenAI) [01][02][03] and around **web-SERP click models** [19][23][24].
We adopt the *schema discipline* and the *feedback-loop discipline*, and explicitly drop the
click-model machinery, which assumes a ranked SERP and high traffic we do not have (see §4).

---

## 1. Log schema (per request)

One SQLite row per request is the right grain — the report consensus is "one query = one trace"
[01][02][03], and the existing issue-033 schema already matches this. We keep it flat (no spans):
we have exactly two stages (embed, retrieve) plus an optional judge, so a row is enough.

### TIER-1 — must-have, cheap, no extra LLM/embedding calls (log on 100% of traffic)

All of these are already computed or trivially available in the daemon; cost is one INSERT.

| Field | Source / note | Feasible here? |
|---|---|---|
| `ts`, `request_id` | wall clock + uuid | yes |
| `user`, `session_id` | bearer key (032); session = client-supplied or rolling id | yes — §4 |
| `channel`/`endpoint` | CLI vs MCP vs HTTP; `query` vs `extract` | yes |
| `collection`, `limit` (k) | request params | yes |
| `query` (verbatim) | the corpus-gap signal + future golden probe [29][30][32]; disclosed to keyed users | yes (already planned) |
| `query_len`, `query_char_len` | cheap pre-retrieval features [09][17] | yes |
| `status` (ok/empty/error), `error_type` | RED metrics [05][08] | yes |
| `latency_ms`, `embed_ms`, `retrieve_ms` | per-stage split — report consensus [08][42] | yes |
| `hits` (count returned) | zero/low-result detection [12][29] | yes |
| `top_score`, `margin` (top1−top2), `score_spread` (σ/range of top-k), `fragment_rate` | **already in Tier-0 confidence** — this is exactly the post-retrieval QPP signal (NQC/WIG-style) the IR literature uses [09][10][11][15][16][17] | yes — already computed |
| `score_vector` (full top-k scores) | enables gap/drop-off + recalibration offline [11][16]; store as JSON/blob | yes |
| `confidence_label` + `confidence_value` | Tier-0 output | yes — already computed |
| `embed_model`, `index_version`/corpus snapshot id, `daemon_version` | drift + deploy-correlation [18][35][42] | yes |
| `est_embed_cost` | tokens × price-table (OpenAI is the only paid call) [07][48] | yes |

Post-query coupling (cheap, no LLM): the `extract` call and any follow-up `query` in the same
session are first-class **implicit signals** for us (see §3). Log on the extract row:
`parent_request_id`, `chunk_ids_extracted`. This is the CLI/API analogue of a citation click [25][47].

### TIER-2 — sampled / opt-in (do NOT run on 100%; bounded cost)

| Field | Method | When to run |
|---|---|---|
| `judge_score` (faithfulness/answer-relevance, reference-free RAG-triad) [13][14][15][18][49] | Tier-1 LLM judge (Anthropic = free, but rate/latency cost) | sampled 5–20% [01][34][40], or triggered on `confidence_label=weak`/`likely_no_answer` |
| `judge_reason`, `failure_category` | LLM open-coding into a taxonomy [03][46] | same sample |
| `answerable_pred` (answerable vs gap) | LLM or threshold classifier [12][17][29] | triggered on low confidence |
| `golden_metrics` (hit-rate@k, MRR) | Tier-2 health vs golden set — **not per-request**; periodic batch | scheduled / pre-deploy gate [32][42] |

### Infeasible / not-applicable to a CLI/API tool (flag explicitly)
- **Dwell time, scroll, hover, pogo-sticking** [19][26][27] — no rendered page, no view lifecycle. Drop.
- **Position / examination propensity, click models, IPW, counterfactual LTR** [19][23][24][28] — we return a small unranked-for-the-user result set the agent reads in full; there is no position bias to debias and far too little traffic. Drop (see §4).
- **Per-result CTR / impressions** [04][25] — no impression surface. The `extract` call is our only "click" proxy.
- **PII anonymisation pipeline (Presidio, k-anonymity, AOL-style)** [06] — keyed, ≤25 trusted users, copyrighted corpus we control. Disclosure + access control suffice; full anonymisation is over-engineering. Keep only: verbatim-logging disclosure + retention TTL.

---

## 2. Metrics catalog

Marked `online-cheap` (100% of traffic, no LLM), `sampled-LLM` (Tier-2 judge), `needs-labels`
(golden set / human).

**Latency & cost**
- p50/p95/p99 end-to-end and per-stage (embed vs retrieve) — `online-cheap` [08][42]. Watch p99/p50 tail ratio [08]; with ≤25 users and an external OpenAI hop, p95 is the SLO, p99 is noisy.
- Embedding cost / day, per user, **per successful answer** (the report-preferred unit) — `online-cheap` [07][48].
- Error/timeout rate (RED) — `online-cheap` [05][08].

**Retrieval quality**
- Zero-result rate; low-confidence/abstention rate — `online-cheap` [12][29]. (Targets like "<2-3% zero-result" [12][29] are ecommerce; ours will differ — set our own baseline, §4.)
- `top_score` / `margin` / `score_spread` distributions and drop-off gap — `online-cheap` [11][16]; these *are* post-retrieval QPP [09][15][17].
- hit-rate@k, MRR vs golden set — `needs-labels` (Tier-2 health, already have) [32][50].

**Answer quality**
- Faithfulness / groundedness, answer relevance, context relevance (RAG triad) — `sampled-LLM` [13][14][15][18][49].
- Citation/attribution validity (does the cited chunk entail the claim) — `sampled-LLM`, only if we start emitting per-claim citations [47][49].
- Hallucination / unsupported-claim rate — `sampled-LLM` [13].

**Feedback (engagement)**
- Re-query / reformulation rate within a session — `online-cheap`, our strongest behavioural signal [04][20][26].
- `extract`-after-`query` rate (our "satisfied click" proxy) — `online-cheap` [25][47].
- Explicit thumbs (if we add a `librarian feedback <request_id> up/down`) — `online-cheap` but sparse/biased [22][28].

**Corpus-gap**
- Per-(clustered)-topic demand-vs-coverage gap score = query frequency × low-confidence rate — `online-cheap` + light embedding clustering [29][30][31][39].
- Weak-query-by-frequency list (already in the issue-033 `stats` plan) — `online-cheap` [29][30].

**Adoption (≤25-user internal-tool framing)** [44]
- WAU/MAU stickiness, queries-per-user, queries-to-`extract` — `online-cheap`. WAU > DAU for a tool used in bursts [44].

---

## 3. Feedback loops (how we actually use the data)

1. **Corpus-gap acquisition list (the data flywheel core).** Aggregate `confidence_label ∈ {weak, likely_no_answer}` and zero/low-`top_score` queries, **cluster by embedding** into topics (not one-offs — the repeated lesson [29][30][31][39]), rank by *frequency × low-confidence* (demand-vs-supply [29][30][39]), route to acquisition. Before acquiring, run the **overlap test** [12][29][31]: does *any* chunk match intent? If yes → retrieval/vocabulary problem (re-chunk, raise k, lower threshold); if no → genuine content gap → ingest. This is `librarian stats`' weak-query view made actionable. Already in the issue-033 acceptance criteria; the literature just adds *cluster-first* and *overlap-test* discipline.

2. **Golden-set-from-logs.** Every Tier-2 judge failure and every user-flagged bad answer becomes a candidate golden-set entry — "every production failure becomes a golden-set entry" [03][32][40]. Stratify by frequency strata (head/torso/tail), since pure frequency sampling misses the tail [32]. Promote via a quick human pass (us). This grows the Tier-2 set from real demand, which beats synthetic benchmarks [32]. The recurring warning: offline golden faithfulness (~0.92) overstates live (~0.78) [49] — so the golden set must be *fed from logs*, not frozen.

3. **Confidence-threshold recalibration (our known bug).** Nonsense scores top ~0.38 and almost everything labels "weak" — the textbook symptom of an **uncalibrated, embedder-specific threshold** [10][16]. Fix exactly as the literature prescribes: collect the logged `top_score`/`margin`/`score_vector` distribution, pair it with golden answerable/unanswerable labels, fit the operating point on a **held-out** split [16] using **F1-knee / Youden's J** or a risk-coverage curve [10][16], validate with **ECE / reliability diagram** [10]. Then monitor ECE drift and recalibrate when corpus/query mix shifts [10][16][35]. This is *the* design pivot: our Tier-0 signal is already the right signal, only mis-thresholded.

4. **Drift detection.** Track (a) query-distribution drift via embedding centroid distance + KS test [35][43], (b) retrieval-health drift via rolling mean/σ of `top_score` vs baseline (alert on sustained >2σ drop) [35][43], (c) corpus-coverage drift via new low-precision query clusters [35]. Use **robust z-score (median+MAD)** on residuals to avoid alert noise at our low volume [43]. Re-run golden hit-rate@k after any re-embed / corpus change as a self-heal gate [35].

5. **Acquisition prioritisation.** Rank the gap list by frequency × impact; close the loop on a cadence (the reports stress cadence + a human owner [29][30][40]): for ≤25 users, monthly is the right rhythm, not weekly. Measure **gap-closure velocity** [30] — are gaps closing faster than they appear — to know the flywheel is turning.

---

## 4. Applies / doesn't apply to a CLI/API, ≤25-user tool

**Transfers directly:**
- Flat per-request log with stage-split latency, token/cost, confidence/score fields [01][02][07][08].
- Post-retrieval QPP / score-distribution signals as a confidence proxy [09][10][11][16][17] — these need *no* clicks or traffic, perfect for low-volume.
- Threshold calibration from logged scores [10][16]; drift detection [35][43].
- Reference-free LLM-as-judge on a sample [14][15][34]; golden-set-from-logs [32][40].
- Corpus-gap mining from zero/low-confidence queries [12][29][30][31][39].
- Reformulation / re-query as a dissatisfaction signal [20][26] (works in any conversational/CLI loop, not just SERP).
- Cost guardrails: cost-per-successful-answer, daily cap, anomaly on 7-day baseline [07][48]. (We only pay OpenAI embeddings, so the runaway-agent-loop horror stories [48] are bounded — but a per-user/day embedding-cost view is still worth it.)
- Internal-tool adoption funnel: WAU/MAU on value-actions [44].

**Does NOT apply (web-SERP or high-traffic assumptions):**
- **Click models, position/examination bias, IPW/counterfactual LTR** [19][23][24][28][36] — no ranked presentation surface, no position to debias, and the agent reads all returned hits. Wholesale drop.
- **A/B testing & interleaving** [33][34][41] — designed for traffic volumes we will never have (each needs 100s–1000s of impressions; interleaving needs a per-result origin tag on a SERP). For retrieval/embedding changes we evaluate **offline against the golden set** instead. Interleaving's *premise* (offline-gate before any online exposure) we keep; its mechanism we drop.
- **Dwell/scroll/pogo-stick satisfaction proxies** [19][25][26][27] — no view lifecycle.
- **Sampling infrastructure (reservoir/tail sampling, head vs tail)** [05] — at our volume we log 100% of Tier-1 cheaply; sampling applies *only* to the Tier-2 LLM judge, and even there a simple fixed-rate + "always judge low-confidence" rule beats reservoir machinery.
- **Heavy PII/anonymisation** [06] — keyed trusted users; disclosure + TTL only.
- **Per-cohort personalisation / self-tuning rank** [36][38][50] — too few users; signals too sparse [25][50]. Don't personalise ranking.

---

## 5. Top recommendations for issue 033 (ranked by value/effort)

1. **Persist the Tier-0 confidence + full `score_vector` on every request, not just the label.** Highest value, near-zero effort — it's already computed and currently discarded. Unblocks recs 2, 4, 7. [09][11][16]
2. **Ship the flat SQLite Tier-1 row** (schema in §1) via request-log middleware, 100% of traffic, one INSERT, no sampling. TDD. Matches the existing issue-033 plan; this synthesis just fixes the field list (add `score_vector`, `score_spread`, `query_len`, `index_version`, `est_embed_cost`, `session_id`). [02][05][07]
3. **Recalibrate `ConfidenceThresholds` from logged scores + golden labels** using F1-knee/Youden + a held-out split + ECE check. This is the known mis-calibration bug and the single most concrete win. Can be done *now* against synthesised answerable/unanswerable sets before real traffic exists (see §6). [10][16]
4. **`librarian stats` with: per-user/day counts, latency p50/p95, label distribution, top queries, weak-query-by-frequency, per-user embedding cost.** Already planned; add a **clustered** weak-query view (embed + group) so the acquisition list is topics, not one-offs. [29][30][39][44]
5. **Weak-query → acquisition workflow with the overlap test** to split content-gap from retrieval-gap before acquiring. Cheap, prevents wasted ingestion. [12][29][31]
6. **Log `extract`-after-`query` and same-session re-query as implicit signals.** Free behavioural feedback that *does* transfer to CLI; our only real engagement proxy. [20][25][26][47]
7. **Tier-2 judge: run the existing LLM judge on low-confidence + a 10% random sample, write `judge_score`/`failure_category` back to the row.** Bounded cost (Anthropic free), feeds golden-set-from-logs. [03][14][32][34][40][46]
8. **Drift watch on `top_score` rolling mean/σ and query-embedding centroid**, robust-z alert. Low effort once rec 1–2 land; catches silent retrieval degradation (HTTP-200 wrong answers) [16][35][43].
9. **Golden-set-from-logs loop:** promote judge/user failures into the Tier-2 golden set, stratified by frequency. Closes the flywheel. [32][40][49]
10. **Retention TTL + verbatim-logging disclosure** (in the keyed-user terms). The only privacy work warranted at our scale. [06]

Defer / reject: A/B + interleaving [33], click-model/IPW LTR [23][24], dwell-based proxies [19], cohort personalisation [38], reservoir/tail sampling [05], Presidio anonymisation [06].

---

## 6. Experiments we can run NOW (no real users)

We have golden sets, the live Tier-0 confidence signals, the Tier-1 LLM judge, and can synthesise
answerable-vs-unanswerable query sets — enough to validate "what to log and how to use it" before
any of the ≤25 users exists.

- **E1 — Confidence recalibration & separability (the priority experiment).** Build two query sets: *answerable* (paraphrases/questions derived from golden-set passages) and *unanswerable* (off-topic + adversarial nonsense). Run all through the daemon, log `top_score`, `margin`, `score_spread`, `fragment_rate`. Plot the score distributions of the two classes; compute AUROC of each signal vs the answerable label, fit the F1-knee/Youden threshold on a held-out split, report ECE before/after [10][16][17]. **Decides: which logged fields actually separate good from bad, and the corrected thresholds.** This both fixes the known bug and proves the log schema's core fields earn their place.
- **E2 — QPP correlation.** Correlate `top_score`/`margin`/`score_spread` against golden hit-rate@k / MRR (Pearson/Kendall) [09][16]. Confirms the cheap online signal tracks real retrieval quality, justifying logging it instead of always invoking the judge.
- **E3 — Judge-vs-threshold agreement & sampling rate.** Run the Tier-1 judge on the E1 sets; measure judge-vs-golden agreement (κ) [32], and how much of the answerable/unanswerable distinction the cheap Tier-0 signal already captures. **Decides the Tier-2 sampling policy** (how low can the random rate go if we always judge low-confidence?).
- **E4 — Gap-detection precision.** Inject known holes (drop a topic's docs from the index), fire topic queries, check the weak-query/low-confidence list + overlap test correctly flags the hole as a *content* gap vs a retrieval gap [29][31]. Validates the acquisition loop end-to-end.

---

### Executive summary
The literature gives us a schema discipline (one flat per-request row: identity, stage-split
latency, cost, and the **already-computed** confidence/score fields) and a feedback-loop
discipline (corpus-gap mining, golden-set-from-logs, threshold calibration, drift watch) — while
most of its machinery (click models, IPW LTR, A/B/interleaving, dwell proxies, heavy
anonymisation) assumes a web SERP and high traffic we do not have and should drop. Top moves:
persist the full confidence/score vector we currently throw away; ship the flat SQLite Tier-1
log on 100% of traffic; **recalibrate the mis-set thresholds from logged scores + golden labels**;
add a clustered weak-query view to `librarian stats` driving an overlap-tested acquisition list;
sample the LLM judge only on low-confidence + 10% random.

**The one experiment to run now: E1 — the answerable-vs-unanswerable confidence-separability /
recalibration study.** It uses only assets we already have, directly fixes the known
mis-calibration ("everything labels weak"), and empirically proves which fields are worth
logging and how to threshold them — validating both halves of "what to log and how to use it"
before a single real user arrives.
