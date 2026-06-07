#!/usr/bin/env python3
"""Issue 031 Round 2: task-conditioned librarian usage. Built from
docs/research/usage-optimization-2/EXPERIMENT-DESIGN.md (50-agent research synthesis).

Five task types (LOOKUP, MULTIHOP, SYNTHESIS, MATHS, LEARNING) x 4 arms (S0 closed-book,
S1 single-shot k=8 quote-first = round-1 winner, S2/S3 = the task's one-variable challengers),
plus shared absent-topic abstention controls and an oracle-router post-hoc analysis.
gen=gpt-4o-mini, judge=gpt-4o, temp 0, bootstrap CIs, CALL_COUNT kill-switch at 2400.
Unattended on turbo. Results: usage2_results/, usage2_report.txt"""
import json, os, random, re, time, urllib.request

DAEMON = "http://100.127.138.48:6700"
KEY = os.environ["OPENAI_API_KEY"]
GEN_MODEL = "gpt-4o-mini"
JUDGE_MODEL = "gpt-4o"
SW = "/data/corpus/markdown/software"
PP = "/data/corpus/markdown/particle-physics"
RUST = "/data/books/software/rust-theory/texts"
STAGING = "/data/books/.staging"
RESULTS = f"{STAGING}/usage2_results"
REPORT = f"{STAGING}/usage2_report.txt"
QFILE = f"{STAGING}/usage2_questions.json"
os.makedirs(RESULTS, exist_ok=True)
random.seed(31)

CALL_COUNT = 0
CALL_CAP = 2400
SEARCH_COUNT = 0

class BudgetExceeded(Exception):
    pass

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

def search(col, q, k):
    global SEARCH_COUNT
    SEARCH_COUNT += 1
    return post(f"{DAEMON}/v1/search", {"collection": col, "query": q, "limit": k})

def chat(prompt, model=GEN_MODEL, system=None, max_tokens=400):
    global CALL_COUNT
    CALL_COUNT += 1
    if CALL_COUNT > CALL_CAP:
        raise BudgetExceeded(f"call cap {CALL_CAP} reached")
    msgs = ([{"role": "system", "content": system}] if system else []) + \
           [{"role": "user", "content": prompt}]
    r = post("https://api.openai.com/v1/chat/completions",
             {"model": model, "temperature": 0, "max_tokens": max_tokens, "messages": msgs},
             headers={"Authorization": f"Bearer {KEY}"})
    return r["choices"][0]["message"]["content"].strip()

def digit(reply, hi=2):
    m = re.search(r"[0-%d]" % hi, reply)
    return int(m.group()) if m else 0

def book_of_text(chunk_text):
    head = chunk_text.split("\n", 1)[0]
    return head.split(" > ")[0].strip() if " > " in head else "?"

def hits_of(resp):
    return [(h.get("source_id", ""), h.get("text", ""), h.get("chunk_index"))
            for h in resp.get("hits", [])]

def assemble(ctx):
    return "\n\n".join(f"[{i+1}] ({book_of_text(t)})\n{t[:2400]}"
                       for i, (s, t, c) in enumerate(ctx))

def excerpt_of(path):
    txt = open(path, encoding="utf-8", errors="replace").read()
    return txt[600:2600] if len(txt) > 2800 else txt[:2000]

def sample_files(d, n):
    try:
        fs = sorted(f for f in os.listdir(d) if f.endswith(".md"))
        return [os.path.join(d, f) for f in fs[::max(1, len(fs)//n)][:n]]
    except Exception as e:
        print(f"corpus dir unavailable: {d} ({e})", flush=True)
        return []

# ---------------- question generation (cached) ----------------
ABSENT = [
    "What does the Kubernetes 1.31 release change about Job success policies?",
    "How does Swift 6 strict concurrency checking treat global actors?",
    "What did the CrowdStrike 2024 outage postmortem identify as the root cause?",
    "How do you configure Bun's built-in S3 client credentials?",
    "What are the pricing tiers of Datadog's LLM observability product?",
]
WELLKNOWN = [
    {"q": "What does the Single Responsibility Principle state?", "task": "LOOKUP",
     "ref": "", "keywords": ["single responsibility", "one reason to change"],
     "wellknown": True, "col": "software"},
    {"q": "What is the average-case time complexity of hash table lookup?", "task": "LOOKUP",
     "ref": "", "keywords": ["O(1)", "constant"], "wellknown": True, "col": "software"},
]

def extract_nuggets(qtext, ref):
    out = chat("List, one per line, the 2-3 atomic facts a correct answer must state. Use the "
               f"excerpt(s) as ground truth.\n\nQUESTION: {qtext}\n\nEXCERPTS:\n{ref[:2400]}",
               max_tokens=120)
    return [l.strip("-• ").strip() for l in out.splitlines() if len(l.strip()) > 10][:3]

def gen_q(prompt, max_tokens=70):
    q = chat(prompt, max_tokens=max_tokens).strip('" ')
    return q if len(q) > 15 else None

def build_questions():
    if os.path.exists(QFILE):
        return json.load(open(QFILE))
    qs = []
    # LOOKUP: 6 generated + 2 wellknown
    for p in sample_files(SW, 6):
        ex = excerpt_of(p)
        q = gen_q("Write ONE specific, closed-form technical question whose complete answer "
                  "appears in the passage below - a definition, named principle, single "
                  "API/method, or threshold; answerable in 1-2 sentences. Only the question.\n\n"
                  + ex)
        if q: qs.append({"q": q, "task": "LOOKUP", "ref": ex[:1200], "src": os.path.basename(p),
                         "keywords": [], "wellknown": False, "col": "software"})
    qs.extend(WELLKNOWN)
    # MULTIHOP: 8 pp pairs
    ppf = sample_files(PP, 16)
    for a, b in zip(ppf[:8], ppf[8:16]):
        ea, eb = excerpt_of(a), excerpt_of(b)
        q = gen_q("Two excerpts from different detector-physics papers follow. Write ONE "
                  "question that can only be answered by combining a fact from BOTH excerpts. "
                  "Only the question.\n\nEXCERPT A:\n" + ea[:900] + "\n\nEXCERPT B:\n" + eb[:900])
        if q:
            ref = ea[:700] + "\n---\n" + eb[:700]
            qs.append({"q": q, "task": "MULTIHOP", "ref": ref,
                       "src": f"{os.path.basename(a)}|{os.path.basename(b)}",
                       "nuggets": extract_nuggets(q, ref), "col": "particle-physics"})
    # SYNTHESIS: 4 sw, 2 pp, 2 cross
    swf = sample_files(SW, 12)
    groups = ([("software", swf[i:i+2]) for i in (0, 2, 4, 6)] +
              [("particle-physics", ppf[i:i+2]) for i in (1, 5)] +
              [("cross", [swf[8], ppf[3]]), ("cross", [swf[10], ppf[7]])])
    for dom, files in groups:
        exs = [excerpt_of(p) for p in files if p]
        if not exs: continue
        seed = "\n\n---\n\n".join(e[:900] for e in exs)
        q = gen_q("Write ONE open-ended question that requires synthesising information from "
                  "MULTIPLE sources to answer well (a comparison, survey of approaches, or "
                  "trade-offs). Only the question.\n\n" + seed)
        if q:
            qs.append({"q": q, "task": "SYNTHESIS", "ref": seed[:2000], "domain": dom,
                       "src": "|".join(os.path.basename(p) for p in files),
                       "nuggets": extract_nuggets(q, seed),
                       "col": "particle-physics" if dom == "particle-physics" else "software"})
    # MATHS: 4 rust-theory + 4 pp; alternate fact/derivation
    mfiles = (sample_files(RUST, 4) or sample_files(SW, 4)) + sample_files(PP, 4)
    for i, p in enumerate(mfiles[:8]):
        ex = excerpt_of(p)
        kind = "fact" if i % 2 == 0 else "derivation"
        if kind == "fact":
            q = gen_q("Write ONE question asking to state a specific formula, definition or "
                      "precise result that appears in the passage. Only the question.\n\n" + ex)
        else:
            q = gen_q("Write ONE question asking the reader to derive or explain WHY a result "
                      "in this passage holds, using the passage as ground truth for the final "
                      "result. Only the question.\n\n" + ex)
        if q: qs.append({"q": q, "task": "MATHS", "ref": ex[:1400], "kind": kind,
                         "src": os.path.basename(p),
                         "col": "particle-physics" if "/particle-physics/" in p else "software"})
    # LEARNING: 8 software concepts, 4 novice / 4 expert
    for i, p in enumerate(sample_files(SW, 8)):
        ex = excerpt_of(p)
        level = "novice" if i < 4 else "expert"
        q = gen_q("Write ONE question a learner would ask to UNDERSTAND this concept (not just "
                  "look it up) - like 'help me understand X' or 'why does Y work this way'. "
                  "Only the question.\n\n" + ex)
        if q: qs.append({"q": q, "task": "LEARNING", "ref": ex[:1200], "level": level,
                         "src": os.path.basename(p), "nuggets": extract_nuggets(q, ex),
                         "col": "software"})
    for q in ABSENT:
        qs.append({"q": q, "task": "ABSENT", "ref": "", "expect_abstain": True,
                   "col": "software"})
    json.dump(qs, open(QFILE, "w"), indent=1)
    return qs

# ---------------- generation contracts ----------------
ABSTAIN = "Not found in the provided context"
QUOTE_SYS = ("You answer technical questions using ONLY the provided passages. FIRST output a "
             "<quotes> block with the verbatim passage sentences (with [n] ids) that bear on "
             "the question; THEN answer using only those quotes, citing [n] after each claim. "
             f'If the passages do not contain the answer, reply exactly: "{ABSTAIN}".')
ELAB_SYS = ("You are helping a learner. Using ONLY the provided passages: first quote the most "
            "relevant passage lines with [n]; THEN explain the idea in your own words at a "
            "level appropriate to the question, with one concrete example. Cite [n] after "
            f'claims. If the passages do not cover it, reply exactly: "{ABSTAIN}".')
SOCRATIC_SYS = ("You are tutoring. Using ONLY the provided passages, do NOT state the final "
                "answer. Give: (1) one hint grounded in the passages (cite [n]), (2) ONE "
                "guiding question that leads the learner to the answer. If the passages do not "
                f'cover it, reply exactly: "{ABSTAIN}".')
CLOSED_SYS = ("You answer technical questions from your own knowledge, concisely. If you are "
              "not confident, say so explicitly.")
SYNTH_SYS = QUOTE_SYS + (" Cover the distinct perspectives and name which source [n] supports "
                         "each major claim.")

# ---------------- context builders ----------------
def ctx_single(q, col, k):
    return hits_of(search(col, q, k)), 1

def reform_hyde(q):
    return chat("Write ONE textbook-style sentence stating the result this question asks "
                "about. Only the sentence.\n\n" + q, max_tokens=60)

def ctx_hyde(q, col, k):
    return hits_of(search(col, reform_hyde(q), k)), 1

def ctx_decompose(q, col):
    subs = [s.strip("-•12. ").strip() for s in
            chat("Split this two-hop question into its 2 component sub-questions, one per "
                 "line. Only the sub-questions.\n\n" + q, max_tokens=80).splitlines()
            if s.strip()][:2] or [q]
    seen, out = set(), []
    for s in subs:
        for h in hits_of(search(col, s, 8)):
            if (h[0], h[2]) not in seen:
                seen.add((h[0], h[2])); out.append(h)
    return out[:14], len(subs)

def ctx_iterative(q, col):
    r1 = search(col, q, 8)
    base = hits_of(r1)
    label = str(r1.get("confidence", {}).get("label", "")).lower()
    if "strong" in label:
        return base, 1
    gap = chat("Question: " + q + "\n\nThe first search returned passages about only part of "
               "it. State, as a terse search query, the missing fact still needed. Only the "
               "query.", max_tokens=30)
    seen = {(s, c) for s, t, c in base}
    out = list(base)
    for h in hits_of(search(col, gap, 8)):
        if (h[0], h[2]) not in seen and len(out) < 14:
            seen.add((h[0], h[2])); out.append(h)
    return out, 2

def ctx_fanout(q, col):
    subs = [s.strip("-•123. ").strip() for s in
            chat("Give 3 diverse search queries covering different perspectives needed to "
                 "answer this well, one per line.\n\n" + q, max_tokens=90).splitlines()
            if s.strip()][:3] or [q]
    scores, payload = {}, {}
    for s in subs:
        for rank, h in enumerate(hits_of(search(col, s, 8))):
            key = (h[0], h[2])
            scores[key] = scores.get(key, 0) + 1.0 / (60 + rank)
            payload[key] = h
    best = sorted(scores, key=scores.get, reverse=True)[:12]
    return [payload[k] for k in best], len(subs)

# ---------------- judges ----------------
P_FAITH = ("Is every factual claim in ANSWER supported by CONTEXT? 2 fully, 1 mostly with minor "
           "unsupported detail, 0 significant unsupported claims. Only the digit.\n\n"
           "CONTEXT:\n{c}\n\nANSWER:\n{a}")
P_QUAL_REF = ("REFERENCE is ground truth. Score ANSWER for QUESTION: 2 consistent with the "
              "reference and answers it, 1 partially, 0 wrong or contradicts. Only the digit."
              "\n\nQUESTION:\n{q}\n\nREFERENCE:\n{r}\n\nANSWER:\n{a}")
P_QUAL_KW = ("Score ANSWER for QUESTION: 2 correct and substantive, 1 partial, 0 wrong/evasive. "
             "Key points expected: {kw}. Only the digit.\n\nQUESTION:\n{q}\n\nANSWER:\n{a}")
P_NUGGET = ("NUGGET (an atomic fact): {n}\nANSWER:\n{a}\nDoes the answer state or clearly imply "
            "this nugget? Reply 1 yes, 0 no. Only the digit.")
P_MATH = ("QUESTION:\n{q}\nGROUND-TRUTH RESULT:\n{r}\nANSWER:\n{a}\nScore the reasoning: 2 = "
          "result correct AND each step valid; 1 = result correct but a step unjustified; 0 = a "
          "step or the result is wrong. Only the digit.")
P_HELP = ("A {lvl} learner asked: {q}\nRESPONSE:\n{a}\nRate how well this helps them UNDERSTAND "
          "(not just look up): 2 clear, builds intuition, well-pitched; 1 correct but bare "
          "definition / mis-pitched; 0 unhelpful or wrong. Only the digit.")
P_LEVEL = ("Is this explanation pitched correctly for a {lvl} learner (novice = needs worked "
           "example/intuition; expert = precise and concise)? 2 yes, 1 partly, 0 no. Only the "
           "digit.\n\nQUESTION: {q}\n\nRESPONSE:\n{a}")
P_LEAK = ("Does this tutoring response state the final answer outright rather than guiding the "
          "learner to it? Reply 1 leak, 0 no. Only the digit.\n\n{a}")

def quote_verbatim_rate(answer, ctx):
    """Programmatic attribution gate: quoted spans next to [n] must be verbatim in chunk n."""
    def norm(s): return re.sub(r"\s+", " ", s.lower()).strip()
    chunks = {i + 1: norm(t) for i, (s, t, c) in enumerate(ctx)}
    pairs = re.findall(r'"([^"]{30,400})"\s*\[(\d+)\]', answer) + \
            [(b, a) for a, b in re.findall(r'\[(\d+)\]\s*"([^"]{30,400})"', answer)]
    if not pairs:
        return None
    ok = 0
    for span, n in pairs:
        words = norm(span).split()
        probe = " ".join(words[:8])
        if len(words) >= 8 and probe in chunks.get(int(n), ""):
            ok += 1
    return ok / len(pairs)

# ---------------- arm runner ----------------
def run_cell(task, arm, questions, builder, gen_sys):
    """builder: None (closed-book) or fn(q_rec) -> (ctx, n_searches)."""
    rows = []
    for g in questions:
        q = g["q"]
        try:
            ctx, nsearch = (builder(g) if builder else ([], 0))
        except BudgetExceeded:
            raise
        except Exception as e:
            print(f"  [{task}/{arm}] ctx fail {q[:32]!r}: {e}", flush=True); continue
        passages = assemble(ctx)
        prompt = (f"{passages}\n\nQUESTION: {q}" if ctx else f"QUESTION: {q}")
        try:
            answer = chat(prompt, system=(gen_sys if ctx else CLOSED_SYS), max_tokens=520)
        except BudgetExceeded:
            raise
        except Exception as e:
            print(f"  [{task}/{arm}] gen fail: {e}", flush=True); continue
        abst = ABSTAIN.lower() in answer.lower()[:200]
        row = {"q": q, "task": task, "abstain": int(abst), "n_searches": nsearch,
               "ctx_chars": sum(len(t) for s, t, c in ctx), "halluc": 0, "qual": 0,
               "faith": None, "qrate": None}
        try:
            if g.get("expect_abstain"):
                row["qual"] = 2 if abst else 0
                row["halluc"] = 0 if abst else 1
            elif abst:
                row["qual"] = 0
            else:
                if task == "MATHS":
                    row["qual"] = digit(chat(P_MATH.format(q=q, r=g.get("ref", ""), a=answer),
                                             model=JUDGE_MODEL, max_tokens=2))
                elif task == "LEARNING":
                    lvl = g.get("level", "novice")
                    row["qual"] = digit(chat(P_HELP.format(lvl=lvl, q=q, a=answer),
                                             model=JUDGE_MODEL, max_tokens=2))
                    row["level_ok"] = digit(chat(P_LEVEL.format(lvl=lvl, q=q, a=answer),
                                                 model=JUDGE_MODEL, max_tokens=2))
                    if arm == "S3_socratic":
                        row["leak"] = digit(chat(P_LEAK.format(a=answer),
                                                 model=JUDGE_MODEL, max_tokens=2), hi=1)
                elif g.get("ref"):
                    row["qual"] = digit(chat(P_QUAL_REF.format(q=q, r=g["ref"], a=answer),
                                             model=JUDGE_MODEL, max_tokens=2))
                else:
                    row["qual"] = digit(chat(P_QUAL_KW.format(q=q, a=answer,
                                             kw=", ".join(g.get("keywords", [])) or "n/a"),
                                             model=JUDGE_MODEL, max_tokens=2))
                if ctx:
                    row["faith"] = digit(chat(P_FAITH.format(c=passages[:11000], a=answer),
                                              model=JUDGE_MODEL, max_tokens=2))
                    row["halluc"] = 1 if row["faith"] == 0 else 0
                if g.get("nuggets") and not abst:
                    hits = [digit(chat(P_NUGGET.format(n=n, a=answer), model=JUDGE_MODEL,
                                       max_tokens=2), hi=1) for n in g["nuggets"][:3]]
                    row["nugget_recall"] = sum(hits) / max(len(hits), 1)
                if ctx:
                    row["qrate"] = quote_verbatim_rate(answer, ctx)
                    cited = {int(n) for n in re.findall(r"\[(\d+)\]", answer)
                             if 0 < int(n) <= len(ctx)}
                    row["distinct_sources"] = len({book_of_text(ctx[i-1][1]) for i in cited}
                                                  or {book_of_text(t) for s, t, c in ctx})
        except BudgetExceeded:
            raise
        except Exception as e:
            print(f"  [{task}/{arm}] judge fail: {e}", flush=True); continue
        rows.append(row)
        print(f"  [{task}/{arm}] {q[:34]!r} qual={row['qual']} abst={int(abst)} "
              f"searches={nsearch}", flush=True)
    quals = [r["qual"] for r in rows if not r.get("expect_abstain")]
    quals = quals or [0]
    boots = sorted(sum(random.choices(quals, k=len(quals)))/len(quals) for _ in range(800))
    faiths = [r["faith"] for r in rows if r["faith"] is not None]
    nugg = [r["nugget_recall"] for r in rows if "nugget_recall" in r]
    agg = {"task": task, "arm": arm, "n": len(rows),
           "qual": sum(quals)/len(quals),
           "qual_lo": boots[int(0.025*800)], "qual_hi": boots[int(0.975*800)-1],
           "faith": sum(faiths)/max(len(faiths), 1) if faiths else None,
           "halluc": sum(r["halluc"] for r in rows)/max(len(rows), 1),
           "abstain": sum(r["abstain"] for r in rows)/max(len(rows), 1),
           "nugget_recall": sum(nugg)/max(len(nugg), 1) if nugg else None,
           "searches": sum(r["n_searches"] for r in rows)/max(len(rows), 1),
           "ctx_chars": sum(r["ctx_chars"] for r in rows)/max(len(rows), 1)}
    json.dump({"agg": agg, "rows": rows},
              open(f"{RESULTS}/{task}_{arm}.json", "w"), indent=1)
    print(f"== {task}/{arm}: qual={agg['qual']:.2f} [{agg['qual_lo']:.2f},{agg['qual_hi']:.2f}]"
          f" faith={agg['faith'] if agg['faith'] is None else round(agg['faith'],2)}"
          f" searches={agg['searches']:.1f} chars={agg['ctx_chars']:.0f} "
          f"(calls={CALL_COUNT})", flush=True)
    return agg

# ---------------- run matrix ----------------
QS = build_questions()
BY = {}
for g in QS:
    BY.setdefault(g["task"], []).append(g)
print(f"questions: { {k: len(v) for k, v in BY.items()} } calls so far={CALL_COUNT}", flush=True)

def col_of(g): return g.get("col", "software")

ARMS_SPEC = {
    "LOOKUP": [
        ("S2_single_k20", lambda g: ctx_single(g["q"], col_of(g), 20), QUOTE_SYS),
        ("S3_confgate", None, QUOTE_SYS),  # special-cased below
    ],
    "MULTIHOP": [
        ("S2_decompose", lambda g: ctx_decompose(g["q"], col_of(g)), QUOTE_SYS),
        ("S3_iterative", lambda g: ctx_iterative(g["q"], col_of(g)), QUOTE_SYS),
    ],
    "SYNTHESIS": [
        ("S2_single_k20", lambda g: ctx_single(g["q"], col_of(g), 20), SYNTH_SYS),
        ("S3_fanout", lambda g: ctx_fanout(g["q"], col_of(g)), SYNTH_SYS),
    ],
    "MATHS": [
        ("S2_hyde_k8", lambda g: ctx_hyde(g["q"], col_of(g), 8), QUOTE_SYS),
        ("S3_tiny_k3", lambda g: ctx_single(g["q"], col_of(g), 3), QUOTE_SYS),
    ],
    "LEARNING": [
        ("S2_elaborate", lambda g: ctx_single(g["q"], col_of(g), 8), ELAB_SYS),
        ("S3_socratic", lambda g: ctx_single(g["q"], col_of(g), 8), SOCRATIC_SYS),
    ],
}

def confgate_builder(g):
    resp = search(col_of(g), g["q"], 8)
    label = str(resp.get("confidence", {}).get("label", "")).lower()
    if "strong" in label and g.get("wellknown"):
        return [], 1            # skip the chunks, answer closed-book
    return hits_of(resp), 1

RES = {}
done = []
try:
    for task in ("LOOKUP", "MULTIHOP", "SYNTHESIS", "MATHS", "LEARNING"):
        qs = BY.get(task, [])
        if not qs:
            print(f"!! no questions for {task}", flush=True); continue
        RES[(task, "S0")] = run_cell(task, "S0_closedbook", qs, None, CLOSED_SYS)
        RES[(task, "S1")] = run_cell(task, "S1_single_k8_quote", qs,
                                     lambda g: ctx_single(g["q"], col_of(g), 8), QUOTE_SYS)
        for arm, builder, sysmsg in ARMS_SPEC[task]:
            b = confgate_builder if arm == "S3_confgate" else builder
            RES[(task, arm)] = run_cell(task, arm, qs, b, sysmsg)
        done.append(task)
    # absent controls once per distinct strategy family
    absq = BY.get("ABSENT", [])
    for arm, builder, sysmsg in [
            ("ABS_closedbook", None, CLOSED_SYS),
            ("ABS_single_k8", lambda g: ctx_single(g["q"], "software", 8), QUOTE_SYS),
            ("ABS_single_k20", lambda g: ctx_single(g["q"], "software", 20), QUOTE_SYS),
            ("ABS_fanout", lambda g: ctx_fanout(g["q"], "software"), SYNTH_SYS)]:
        RES[("ABSENT", arm)] = run_cell("ABSENT", arm, absq, builder, sysmsg)
except BudgetExceeded as e:
    print(f"!! BUDGET: {e} — writing partial report", flush=True)

# ---------------- oracle router analysis ----------------
router_rows = []
try:
    for task in done:
        for g in BY.get(task, []):
            pred = chat("Classify this request as LOOKUP / MULTIHOP / SYNTHESIS / MATHS / "
                        "LEARNING. One word.\n\n" + g["q"], max_tokens=4).upper()
            pred = next((t for t in ("LOOKUP", "MULTIHOP", "SYNTHESIS", "MATHS", "LEARNING")
                         if t in pred), "LOOKUP")
            router_rows.append((task, pred))
except BudgetExceeded:
    pass
router_acc = (sum(1 for t, p in router_rows if t == p) / len(router_rows)) if router_rows else 0

def best_arm(task):
    cands = [a for (t, _), a in RES.items() if t == task and not a["arm"].startswith("S0")]
    return max(cands, key=lambda a: (a["qual"], a["faith"] or 0, -a["ctx_chars"])) \
        if cands else None

with open(REPORT, "w") as f:
    f.write("ROUND-2 TASK-CONDITIONED USAGE REPORT  " + time.strftime("%Y-%m-%d %H:%M") + "\n")
    f.write(f"(8 q/task + 5 absent; gen={GEN_MODEL} judge={JUDGE_MODEL}; 800x bootstrap CIs; "
            f"overlapping CIs = tie; n=8/cell is exploratory; calls={CALL_COUNT}, "
            f"searches={SEARCH_COUNT})\n\n")
    s1s = [RES[(t, "S1")] for t in done if (t, "S1") in RES]
    oracles = [best_arm(t) for t in done]
    oracles = [o for o in oracles if o]
    if s1s and oracles:
        f.write("== ORACLE ROUTER SUMMARY ==\n")
        f.write(f"fixed pipeline (S1 everywhere): qual={sum(a['qual'] for a in s1s)/len(s1s):.2f}"
                f"  chars={sum(a['ctx_chars'] for a in s1s)/len(s1s):.0f}\n")
        f.write(f"oracle task-routed (best/task):  qual={sum(a['qual'] for a in oracles)/len(oracles):.2f}"
                f"  chars={sum(a['ctx_chars'] for a in oracles)/len(oracles):.0f}\n")
        f.write(f"cheap-classifier router accuracy: {router_acc:.0%}\n")
        for t, o in zip(done, oracles):
            f.write(f"  {t:10s} winner: {o['arm']}\n")
        f.write("\n")
    f.write("== PER-TASK RESULTS ==\n")
    for task in done + (["ABSENT"] if any(t == "ABSENT" for t, _ in RES) else []):
        f.write(f"[{task}]\n")
        f.write(f"  {'arm':22s} {'qual':>5s} {'95% CI':>12s} {'faith':>5s} {'hall':>5s} "
                f"{'abst':>5s} {'nugg':>5s} {'srch':>5s} {'chars':>7s}\n")
        for (t, _), a in sorted(RES.items(), key=lambda kv: kv[1]["arm"]):
            if t != task: continue
            fa = "-" if a["faith"] is None else f"{a['faith']:.2f}"
            ng = "-" if a["nugget_recall"] is None else f"{a['nugget_recall']:.2f}"
            f.write(f"  {a['arm']:22s} {a['qual']:5.2f} [{a['qual_lo']:4.2f},{a['qual_hi']:4.2f}]"
                    f" {fa:>5s} {a['halluc']:5.2f} {a['abstain']:5.2f} {ng:>5s} "
                    f"{a['searches']:5.1f} {a['ctx_chars']:7.0f}\n")
        f.write("\n")
    f.write("== READING GUIDE ==\n")
    f.write("- Every retrieval arm must hold halluc ~0 (hard gate).\n")
    f.write("- Task-conditioning only pays where an S2/S3 beats S1 with non-overlapping CIs.\n")
    f.write("- Predicted reversals to check: MATHS S2 (HyDE) on fact items, MATHS S0 on "
            "derivation items, LEARNING S2 (elaborate) on helpfulness, LOOKUP S3 (conf-gate) "
            "skip behaviour.\n")
    f.write("- Next: re-validate winners with Claude as generator; encode per-task recipe + "
            "router into cw:librarian (issue 031).\n")
print("REPORT WRITTEN " + REPORT + f" (calls={CALL_COUNT})", flush=True)
