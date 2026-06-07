#!/usr/bin/env python3
"""E1 - confidence separability & recalibration (telemetry, issue 033).

Question: do the cheap per-query signals we already compute (top_score, margin, score_spread,
fragment_rate, value) actually separate ANSWERABLE (in-corpus) from UNANSWERABLE (off-domain /
nonsense) queries? And what are the corrected thresholds?

Ground truth by construction: answerable = golden + pilot questions (answers ARE in corpus);
unanswerable = off-domain + nonsense (answers are NOT in this physics+SWE corpus). No LLM judge
needed. Only cost is the daemon's query embeddings (~200 queries, a cent).

Outputs: per-signal AUROC (which fields to log), Youden-J threshold for top_score + value
(corrected no_answer_below), current-label confusion, and ECE of `value` as P(answerable).
"""
import json, urllib.request
from concurrent.futures import ThreadPoolExecutor

DAEMON = "http://100.127.138.48:6700"
GP = "/home/asi/.librarian/golden_pp.json"
GS = "/home/asi/.librarian/golden_software.json"
EP = "/data/books/.staging/ctx_exp/eval_pairs.json"
OUT = "/data/books/.staging/ctx_exp/tele_e1"

OFFDOMAIN = [
    "best sourdough bread recipe for beginners",
    "how to train a labrador retriever puppy",
    "who won the 2022 FIFA world cup final",
    "symptoms and treatment of seasonal allergies",
    "how to prune tomato plants in summer",
    "best budget travel destinations in southeast asia",
    "how to safely change a flat car tire",
    "what caused the fall of the roman empire",
    "calories in a banana versus an apple",
    "how to knit a scarf for beginners",
    "how does the en passant rule work in chess",
    "how to apply for a US passport online",
    "best stretching exercises for lower back pain",
    "how to make cold brew coffee at home",
    "how to grow fresh basil indoors",
    "how long to boil an egg for soft yolk",
    "best dog breeds for small apartments",
    "how to remove red wine stains from carpet",
    "how to meditate to relieve stress",
    "difference between a latte and a cappuccino",
    "how to start a backyard vegetable garden",
    "best scenic hiking trails in the swiss alps",
    "how to tie a bow tie step by step",
    "what causes hiccups and how to stop them",
    "best running shoes for flat feet",
    "homemade pizza dough recipe without yeast",
    "how to grow an avocado tree from a seed",
    "health benefits of drinking green tea daily",
    "how to clean and season a cast iron skillet",
    "guitar chords to play wonderwall",
    "how to fold a fitted bed sheet neatly",
    "how to whiten teeth naturally at home",
    "recipe for soft chocolate chip cookies",
    "how to potty train a toddler quickly",
    "how often should you water succulents",
    "best way to descale a coffee machine",
    "how to get rid of fruit flies in the kitchen",
    "what to feed a hummingbird in winter",
    "how to fix a leaky bathroom faucet",
    "easy origami crane folding instructions",
]
NONSENSE = [
    "zxqwy plrmf gxhtk vbnml",
    "the the the and and of of of",
    "asdfjkl qwerty zxcvbn poiuyt",
    "florble wibwib snerk gronkle plif",
    "blorptang quvix mneso fradl",
    "xkcd zorp glarn femb tuvy",
    "aaaa bbbb cccc dddd eeee",
    "wug blicket dax fep zib",
    "qwop snarf glorx mibble",
    "vorp jindle krunk splee",
    "thneed glorf wuzzle binth",
    "kazoo flibber nooble grant",
    "yzptlk mxyzptlk gvtsk",
    "moop doop loop snoop groop",
    "trz wkx jhg lmn opq rst",
]


def search(col, q, k=10):
    req = urllib.request.Request(DAEMON + "/v1/search",
                                 data=json.dumps({"collection": col, "query": q, "limit": k}).encode(),
                                 headers={"content-type": "application/json"})
    return json.loads(urllib.request.urlopen(req, timeout=60).read())


def conf_of(q, col):
    c = search(col, q)["confidence"]
    return {"top_score": c["top_score"], "margin": c["margin"], "score_spread": c["score_spread"],
            "fragment_rate": c["fragment_rate"], "value": c["value"], "conf_label": c["label"]}


def build():
    jobs = []
    for x in json.load(open(GP)):
        jobs.append((x["q"], "particle-physics", 1))
    for x in json.load(open(GS)):
        jobs.append((x["q"], "software", 1))
    for x in json.load(open(EP)):
        jobs.append((x["q"], "particle-physics", 1))
    for q in OFFDOMAIN + NONSENSE:
        for col in ("particle-physics", "software"):
            jobs.append((q, col, 0))

    def work(j):
        q, col, y = j
        try:
            return {"q": q, "collection": col, "label": y, **conf_of(q, col)}
        except Exception as e:
            return {"q": q, "collection": col, "label": y, "error": str(e)[:80]}

    with ThreadPoolExecutor(max_workers=12) as ex:
        rows = [r for r in ex.map(work, jobs) if "top_score" in r]
    return rows


def auroc(scores, labels):
    """Mann-Whitney AUROC with tie-averaged ranks."""
    pairs = sorted(zip(scores, labels))
    ranks = [0.0] * len(pairs)
    i = 0
    while i < len(pairs):
        j = i
        while j + 1 < len(pairs) and pairs[j + 1][0] == pairs[i][0]:
            j += 1
        avg = (i + j) / 2.0 + 1.0  # 1-based average rank
        for k in range(i, j + 1):
            ranks[k] = avg
        i = j + 1
    npos = sum(labels)
    nneg = len(labels) - npos
    if npos == 0 or nneg == 0:
        return float("nan")
    sum_pos = sum(r for r, (_, l) in zip(ranks, pairs) if l == 1)
    return (sum_pos - npos * (npos + 1) / 2.0) / (npos * nneg)


def youden(scores, labels):
    """Best threshold t: predict answerable if score>=t. Maximize TPR-FPR."""
    npos = sum(labels); nneg = len(labels) - npos
    best = (-1, None, None, None)
    for t in sorted(set(scores)):
        tp = sum(1 for s, l in zip(scores, labels) if s >= t and l == 1)
        fp = sum(1 for s, l in zip(scores, labels) if s >= t and l == 0)
        tpr = tp / npos; fpr = fp / nneg
        j = tpr - fpr
        if j > best[0]:
            best = (j, t, tpr, fpr)
    return best  # (J, threshold, tpr, fpr)


def ece(values, labels, bins=10):
    N = len(values); e = 0.0
    for b in range(bins):
        lo, hi = b / bins, (b + 1) / bins
        idx = [i for i, v in enumerate(values) if (v >= lo and (v < hi or (b == bins - 1 and v <= hi)))]
        if not idx:
            continue
        acc = sum(labels[i] for i in idx) / len(idx)
        conf = sum(values[i] for i in idx) / len(idx)
        e += len(idx) / N * abs(acc - conf)
    return e


def main():
    import os
    os.makedirs(OUT, exist_ok=True)
    rows = build()
    json.dump(rows, open(f"{OUT}/rows.json", "w"), indent=1)
    pos = [r for r in rows if r["label"] == 1]
    neg = [r for r in rows if r["label"] == 0]
    labels = [r["label"] for r in rows]

    L = []
    L.append("E1 - confidence separability & recalibration (telemetry / issue 033)")
    L.append(f"answerable(in-corpus)={len(pos)}  unanswerable(off-domain+nonsense)={len(neg)}")
    L.append("")
    L.append("Per-signal AUROC (separating answerable vs unanswerable; 0.5=useless, 1.0=perfect):")
    signals = {"top_score": lambda r: r["top_score"], "value": lambda r: r["value"],
               "margin": lambda r: r["margin"], "score_spread": lambda r: r["score_spread"],
               "neg_fragment_rate": lambda r: -r["fragment_rate"]}
    aucs = {}
    for name, f in signals.items():
        a = auroc([f(r) for r in rows], labels)
        aucs[name] = a
        L.append(f"  {name:18s} AUROC={a:.3f}")
    L.append("")
    for cls, rs in (("answerable", pos), ("unanswerable", neg)):
        ts = sorted(r["top_score"] for r in rs)
        vs = sorted(r["value"] for r in rs)
        L.append(f"{cls:12s} top_score median={ts[len(ts)//2]:.3f} "
                 f"[{ts[0]:.3f}, {ts[-1]:.3f}]   value median={vs[len(vs)//2]:.3f}")
    L.append("")
    J, t, tpr, fpr = youden([r["top_score"] for r in rows], labels)
    L.append(f"Youden-optimal top_score threshold = {t:.3f}  (TPR={tpr:.2f}, FPR={fpr:.2f}, J={J:.2f})")
    L.append(f"  current ConfidenceThresholds.no_answer_below = 0.25  ->  recommended ~ {t:.2f}")
    Jv, tv, tprv, fprv = youden([r["value"] for r in rows], labels)
    L.append(f"Youden-optimal value threshold     = {tv:.3f}  (TPR={tprv:.2f}, FPR={fprv:.2f})")
    L.append("")
    # current-label behaviour
    from collections import Counter
    L.append("Current label behaviour (how the live thresholds classify each class):")
    for cls, rs in (("answerable", pos), ("unanswerable", neg)):
        c = Counter(r["conf_label"] for r in rs)
        L.append(f"  {cls:12s}: {dict(c)}")
    L.append("")
    e = ece([r["value"] for r in rows], labels)
    L.append(f"ECE of `value` as P(answerable) = {e:.3f}  (0=perfectly calibrated)")
    L.append("")
    L.append("Read: highest-AUROC signals are the ones worth logging + thresholding; the gap")
    L.append("between current 0.25 and the recommended top_score threshold is the mis-calibration.")
    txt = "\n".join(L)
    open(f"{OUT}/summary.txt", "w").write(txt + "\n")
    print(txt)


if __name__ == "__main__":
    main()
