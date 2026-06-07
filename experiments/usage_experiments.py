#!/usr/bin/env python3
"""Issue 031 v2: research-informed exploratory experiments on HOW to use the librarian.
Design per docs/research/usage-optimization/FINDINGS.md (20-agent literature sweep):
- closed-book control (net help vs harm), verbatim-query baseline
- k-sweep -> rerank -> fan-out(RRF) -> deep-read -> quote-first -> gated retry
- passages first / question LAST, numbered IDs + sources, inline-citation requirement,
  explicit abstention contract (insufficient context is worse than none)
- generator (gpt-4o-mini) != judge (gpt-4o) for answer-axis metrics; ctx-precision via mini
- ~45 questions (golden + corpus-generated + absent-topic controls), bootstrap CIs
Runs unattended on turbo. Results: /data/books/.staging/usage_results/, usage_report.txt"""
import json, math, os, random, re, time, urllib.request

DAEMON = "http://100.127.138.48:6700"
KEY = os.environ["OPENAI_API_KEY"]
GEN_MODEL = "gpt-4o-mini"
JUDGE_MODEL = "gpt-4o"          # cross-model judging (self-preference mitigation)
CHEAP_JUDGE = "gpt-4o-mini"     # high-volume ctx-precision only
COL = "software"
SW = "/data/corpus/markdown/software"
STAGING = "/data/books/.staging"
RESULTS = f"{STAGING}/usage_results"
REPORT = f"{STAGING}/usage_report.txt"
QFILE = f"{STAGING}/usage_questions.json"
os.makedirs(RESULTS, exist_ok=True)
random.seed(31)

def post(url, body, headers=None, tries=4):
    data = json.dumps(body).encode()
    for i in range(tries):
        try:
            req = urllib.request.Request(url, data=data,
                headers={"Content-Type": "application/json", **(headers or {})})
            with urllib.request.urlopen(req, timeout=180) as r:
                return json.loads(r.read())
        except Exception:
            if i == tries - 1: raise
            time.sleep(8 * (i + 1))

def search(q, k):
    return post(f"{DAEMON}/v1/search", {"collection": COL, "query": q, "limit": k})

def chat(prompt, model=GEN_MODEL, system=None, max_tokens=400):
    msgs = ([{"role": "system", "content": system}] if system else []) + \
           [{"role": "user", "content": prompt}]
    r = post("https://api.openai.com/v1/chat/completions",
             {"model": model, "temperature": 0, "max_tokens": max_tokens, "messages": msgs},
             headers={"Authorization": f"Bearer {KEY}"})
    return r["choices"][0]["message"]["content"].strip()

def digit(reply):
    m = re.search(r"[012]", reply)
    return int(m.group()) if m else 0

def book_of(sid):
    stem = os.path.basename(sid).rsplit(".", 1)[0]
    return stem.rsplit("__", 1)[0] if "__" in stem else stem

# ---------------- question set (~45, cached) ----------------
ABSENT = [  # topics certainly not in this textbook corpus -> correct behaviour is abstention
    "What does the Kubernetes 1.31 release change about Job success policies?",
    "How does Swift 6 strict concurrency checking treat global actors?",
    "What did the CrowdStrike 2024 outage postmortem identify as the root cause?",
    "How do you configure Bun's built-in S3 client credentials?",
    "What are the pricing tiers of Datadog's LLM observability product?",
]

def build_questions():
    if os.path.exists(QFILE):
        return json.load(open(QFILE))
    qs = []
    for g in json.load(open(f"{STAGING}/golden_answers.json")):
        qs.append({"q": g["q"], "type": "golden", "keywords": g.get("keywords", []), "ref": ""})
    files = sorted(f for f in os.listdir(SW) if f.endswith(".md"))
    picks = files[::max(1, len(files)//25)][:25]
    excerpts = []
    for fn in picks:
        txt = open(os.path.join(SW, fn), encoding="utf-8", errors="replace").read()
        excerpts.append((fn, txt[600:2600]))
    for fn, ex in excerpts[:20]:   # single-hop generated, reference-anchored
        try:
            q = chat("Write ONE specific technical question that the passage below answers. "
                     "Reply with only the question.\n\n" + ex, max_tokens=60).strip('" ')
            if len(q) > 15:
                qs.append({"q": q, "type": "single", "keywords": [], "ref": ex[:1200], "src": fn})
        except Exception as e:
            print("qgen fail", fn, e, flush=True)
    for (f1, e1), (f2, e2) in zip(excerpts[20:23], excerpts[10:13]):  # multi-hop
        try:
            q = chat("Excerpts from two different books follow. Write ONE comparison question "
                     "that needs information from BOTH. Reply with only the question.\n\n"
                     f"BOOK A:\n{e1[:900]}\n\nBOOK B:\n{e2[:900]}", max_tokens=70).strip('" ')
            if len(q) > 15:
                qs.append({"q": q, "type": "multihop", "keywords": [],
                           "ref": e1[:700] + "\n---\n" + e2[:700], "src": f"{f1}|{f2}"})
        except Exception as e:
            print("mh qgen fail", e, flush=True)
    for q in ABSENT:
        qs.append({"q": q, "type": "absent", "keywords": [], "ref": ""})
    json.dump(qs, open(QFILE, "w"), indent=1)
    return qs

QS = build_questions()
print(f"questions: {len(QS)} ({sum(1 for x in QS if x['type']=='absent')} absent controls)",
      flush=True)

# ---------------- context builders ----------------
def hits_of(resp):
    return [(h.get("source_id", ""), h.get("text", ""), h.get("chunk_index"))
            for h in resp.get("hits", [])]

def ctx_flat(q, k):
    return hits_of(search(q, k))

def reform_keywords(q):
    return chat("Rewrite as a terse keyword search query (5-10 words, no question words). "
                "Only the query.\n\n" + q, max_tokens=30)

def reform_hyde(q):
    return chat("Write ONE textbook-style sentence that would answer this question. "
                "Only the sentence.\n\n" + q, max_tokens=60)

def ctx_form(q, k, form):
    q2 = q if form == "verbatim" else (reform_keywords(q) if form == "keywords" else reform_hyde(q))
    return ctx_flat(q2, k)

def ctx_fanout(q, k):  # RRF (Cormack k=60), N=3 sub-queries, equal final budget
    subs = [s.strip("-• ").strip() for s in
            chat("Decompose into 3 diverse terse search queries, one per line.\n\n" + q,
                 max_tokens=80).splitlines() if s.strip()][:3] or [q]
    scores, payload = {}, {}
    for s in subs:
        for rank, h in enumerate(hits_of(search(s, k))):
            key = (h[0], h[2])
            scores[key] = scores.get(key, 0) + 1.0 / (60 + rank)
            payload[key] = h
    best = sorted(scores, key=scores.get, reverse=True)[:k]
    return [payload[key] for key in best]

def ctx_rerank(q, k, pool=30):  # listwise LLM rerank: retrieve wide, keep k
    cands = ctx_flat(q, pool)
    if len(cands) <= k: return cands
    listing = "\n".join(f"[{i+1}] {t[:300]}" for i, (s, t, c) in enumerate(cands))
    reply = chat(f"QUESTION: {q}\n\nRank the {len(cands)} passages below by how well they "
                 f"answer the question. Reply with the best {k} passage numbers, "
                 f"comma-separated, best first.\n\n{listing}", max_tokens=60)
    order = [int(n) - 1 for n in re.findall(r"\d+", reply) if 0 < int(n) <= len(cands)]
    seen, out = set(), []
    for i in order:
        if i not in seen:
            seen.add(i); out.append(cands[i])
    return (out + [c for j, c in enumerate(cands) if j not in seen])[:k]

def expand_from_file(sid, chunk_text, pad=1500):
    try:
        body = chunk_text.split("\n\n", 1)[-1]
        probe = body[200:360] if len(body) > 380 else body[:160]
        if not probe or not os.path.exists(sid): return chunk_text
        full = open(sid, encoding="utf-8", errors="replace").read()
        i = full.find(probe)
        return chunk_text if i < 0 else full[max(0, i - pad): i + len(probe) + pad]
    except Exception:
        return chunk_text

def ctx_deepread(q, form):
    return [(s, expand_from_file(s, t), c) for s, t, c in ctx_form(q, 3, form)]

def ctx_confretry(q, k, form):  # gated, targeted retry; weak chunks not stacked blindly
    resp = search(q if form == "verbatim" else reform_keywords(q), k)
    base = hits_of(resp)
    label = str(resp.get("confidence", {}).get("label", "")).lower()
    if "strong" in label:
        return base
    extra = hits_of(search(reform_hyde(q), k))
    seen = {(s, c) for s, t, c in base[:max(2, k//2)]}
    out = base[:max(2, k//2)]                      # keep only the head of the weak result
    for h in extra:
        if (h[0], h[2]) not in seen and len(out) < k:
            seen.add((h[0], h[2])); out.append(h)
    return out

# ---------------- generation + judging ----------------
ABSTAIN = "Not found in the provided context"
GEN_SYS = ("You answer technical questions using ONLY the provided passages. "
           "Cite the supporting passage number like [2] after each claim. "
           f'If the passages do not contain the answer, reply exactly: "{ABSTAIN}" '
           "followed by one sentence on what is missing. Never use prior knowledge.")
GEN_SYS_CLOSED = ("You answer technical questions from your own knowledge, concisely. "
                  "If you are not confident, say so explicitly.")
QUOTE_SYS = ("You answer technical questions using ONLY the provided passages. "
             "FIRST output a <quotes> block with the verbatim passage sentences (with [n] ids) "
             "that bear on the question; THEN answer using only those quotes, citing [n]. "
             f'If the passages do not contain the answer, reply exactly: "{ABSTAIN}".')

P_CTX = ("Score how well this CONTEXT chunk helps answer the QUESTION: 0 irrelevant/fragment, "
         "1 related but not answering, 2 directly answers. Only the digit.\n\n"
         "QUESTION:\n{q}\n\nCONTEXT CHUNK:\n{c}")
P_FAITH = ("Is every factual claim in ANSWER supported by CONTEXT? 2 fully, 1 mostly with "
           "minor unsupported detail, 0 significant unsupported claims. Only the digit.\n\n"
           "CONTEXT:\n{c}\n\nANSWER:\n{a}")
P_QUAL_REF = ("REFERENCE is ground truth. Score ANSWER for the QUESTION: 2 consistent with "
              "the reference and answers it, 1 partially, 0 wrong or contradicts. Only the "
              "digit.\n\nQUESTION:\n{q}\n\nREFERENCE:\n{r}\n\nANSWER:\n{a}")
P_QUAL = ("Score ANSWER for QUESTION: 2 correct and substantive, 1 partial, 0 wrong/evasive."
          " Key points that should appear: {kw}. Only the digit.\n\n"
          "QUESTION:\n{q}\n\nANSWER:\n{a}")

def assemble(ctx):
    return "\n\n".join(f"[{i+1}] ({book_of(s)})\n{t[:2400]}" for i, (s, t, c) in enumerate(ctx))

def run_arm(name, builder, gen_sys=GEN_SYS):
    rows = []
    for g in QS:
        q = g["q"]
        try:
            ctx = builder(q) if builder else []
        except Exception as e:
            print(f"  [{name}] ctx fail {q[:36]!r}: {e}", flush=True); continue
        passages = assemble(ctx)
        prompt = (f"{passages}\n\nQUESTION: {q}" if ctx else f"QUESTION: {q}")
        try:
            answer = chat(prompt, system=(gen_sys if ctx else GEN_SYS_CLOSED), max_tokens=450)
        except Exception as e:
            print(f"  [{name}] gen fail: {e}", flush=True); continue
        abst = ABSTAIN.lower() in answer.lower()[:160]
        if g["type"] == "absent":
            qual = 2 if abst else 0
            halluc = 0 if abst else 1
            faith = None
        elif abst:
            qual, halluc, faith = 0, 0, None
        else:
            try:
                if g.get("ref"):
                    qual = digit(chat(P_QUAL_REF.format(q=q, r=g["ref"], a=answer),
                                      model=JUDGE_MODEL, max_tokens=2))
                else:
                    qual = digit(chat(P_QUAL.format(q=q, kw=", ".join(g.get("keywords", [])) or "n/a",
                                                    a=answer), model=JUDGE_MODEL, max_tokens=2))
                faith = (digit(chat(P_FAITH.format(c=passages[:11000], a=answer),
                                    model=JUDGE_MODEL, max_tokens=2)) if ctx else None)
            except Exception as e:
                print(f"  [{name}] judge fail: {e}", flush=True); continue
            halluc = 1 if (ctx and faith == 0) else 0
        cprec = []
        if ctx:
            try:
                cprec = [digit(chat(P_CTX.format(q=q, c=t[:3000]), model=CHEAP_JUDGE,
                                    max_tokens=2)) for s, t, c in ctx[:8]]
            except Exception:
                pass
        sents = max(1, len(re.findall(r"[.!?]\s", answer)))
        cites = len(re.findall(r"\[\d+\]", answer))
        rows.append({"q": q, "type": g["type"], "qual": qual, "faith": faith,
                     "halluc": halluc, "abstain": int(abst),
                     "ctx_prec": (sum(cprec)/len(cprec)) if cprec else None,
                     "cite_per_sent": min(1.0, cites/sents),
                     "ctx_chars": sum(len(t) for s, t, c in ctx)})
        print(f"  [{name}] {q[:36]!r} qual={qual} faith={faith} abst={int(abst)}", flush=True)
    n = max(len(rows), 1)
    faiths = [r["faith"] for r in rows if r["faith"] is not None]
    precs = [r["ctx_prec"] for r in rows if r["ctx_prec"] is not None]
    quals = [r["qual"] for r in rows]
    boots = sorted(sum(random.choices(quals, k=len(quals)))/len(quals) for _ in range(800)) \
            if quals else [0]
    agg = {"arm": name, "n": len(rows),
           "qual": sum(quals)/n, "qual_lo": boots[int(0.025*len(boots))],
           "qual_hi": boots[int(0.975*len(boots))-1],
           "faith": sum(faiths)/max(len(faiths), 1),
           "halluc_rate": sum(r["halluc"] for r in rows)/n,
           "abstain_rate": sum(r["abstain"] for r in rows)/n,
           "absent_ok": sum(r["qual"] for r in rows if r["type"] == "absent")/max(
               2*sum(1 for r in rows if r["type"] == "absent"), 1),
           "ctx_prec": sum(precs)/max(len(precs), 1),
           "cite": sum(r["cite_per_sent"] for r in rows)/n,
           "ctx_chars": sum(r["ctx_chars"] for r in rows)/n}
    json.dump({"agg": agg, "rows": rows}, open(f"{RESULTS}/{name}.json", "w"), indent=1)
    print(f"== {name}: qual={agg['qual']:.2f} [{agg['qual_lo']:.2f},{agg['qual_hi']:.2f}] "
          f"faith={agg['faith']:.2f} halluc={agg['halluc_rate']:.2f} "
          f"chars={agg['ctx_chars']:.0f}", flush=True)
    return agg

ARMS = []
ARMS.append(run_arm("A0_closedbook", None))                         # net-help control
for k in (3, 5, 8, 12, 20):                                         # A1 k-sweep
    ARMS.append(run_arm(f"A1_k{k}", lambda q, k=k: ctx_flat(q, k)))
sweep = [a for a in ARMS if a["arm"].startswith("A1_")]
best = sorted(sweep, key=lambda a: (a["qual"], a["faith"], -a["ctx_chars"]), reverse=True)[0]
K = int(best["arm"].split("k")[1]); print(f"## k* = {K}", flush=True)

forms = {"verbatim": best}
for form in ("keywords", "hyde"):                                   # A2 query form
    forms[form] = run_arm(f"A2_{form}_k{K}", lambda q, f=form: ctx_form(q, K, f))
    ARMS.append(forms[form])
FORM = max(forms, key=lambda f: (forms[f]["qual"], forms[f]["faith"]))
print(f"## form* = {FORM}", flush=True)

ARMS.append(run_arm(f"A3_rerank30_k{K}", lambda q: ctx_rerank(q, K)))
ARMS.append(run_arm(f"A4_fanout_k{K}", lambda q: ctx_fanout(q, K)))
ARMS.append(run_arm(f"A5_deepread_{FORM}", lambda q: ctx_deepread(q, FORM)))
ARMS.append(run_arm(f"A6_quotefirst_{FORM}_k{K}",
                    lambda q: ctx_form(q, K, FORM), gen_sys=QUOTE_SYS))
ARMS.append(run_arm(f"A7_confretry_{FORM}_k{K}", lambda q: ctx_confretry(q, K, FORM)))

ranked = sorted(ARMS, key=lambda a: (a["qual"], a["faith"], a["ctx_prec"]), reverse=True)
with open(REPORT, "w") as f:
    f.write("USAGE OPTIMIZATION REPORT  " + time.strftime("%Y-%m-%d %H:%M") + "\n")
    f.write(f"({len(QS)} questions incl. absent-topic controls; gen={GEN_MODEL}, "
            f"judge={JUDGE_MODEL}; CIs = bootstrap over questions; overlapping CIs = tie)\n\n")
    f.write(f"{'arm':26s} {'qual':>5s} {'95% CI':>12s} {'faith':>5s} {'halluc':>6s} "
            f"{'abst':>5s} {'absOK':>5s} {'prec':>5s} {'cite':>5s} {'chars':>7s}\n")
    for a in ranked:
        f.write(f"{a['arm']:26s} {a['qual']:5.2f} [{a['qual_lo']:4.2f},{a['qual_hi']:4.2f}] "
                f"{a['faith']:5.2f} {a['halluc_rate']:6.2f} {a['abstain_rate']:5.2f} "
                f"{a['absent_ok']:5.2f} {a['ctx_prec']:5.2f} {a['cite']:5.2f} "
                f"{a['ctx_chars']:7.0f}\n")
    f.write(f"\nk* = {K}; form* = {FORM}; WINNER (point estimate): {ranked[0]['arm']}\n")
    f.write("Read with the closed-book control: arms must beat A0 to justify retrieval at "
            "all, and absent-topic abstention (absOK) is a first-class outcome.\n"
            "Next: re-validate the top-2 arms with Claude as generator; then encode the "
            "recipe in the cw:librarian skill (issue 031).\n")
print("REPORT WRITTEN " + REPORT, flush=True)
