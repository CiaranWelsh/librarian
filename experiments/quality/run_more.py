# run_more.py - follow-up experiments to tighten the evidence before extracting a design.
# E9  validate the 13% low-value classification (false positives? volume?)
# E10 push SUBTLE garble toward the threshold + test on REAL mojibake docs + corrupt-but-balanced math
# (E11 physics prevalence is added after inspect_cache.py reveals the cache format)
import os
import sys
import random
from collections import defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import qsig
import run_experiments as R

random.seed(7)


def gscore(t):
    return (qsig.letterspace_density(t)
            + 1000 * qsig.replacement_rate(t)
            + min(qsig.latex_issues(t), 5))


def synth_math_corrupt(text):
    # corrupt characters INSIDE math, keeping $ delimiters balanced -> the subtle
    # "marker mangled a formula but it still parses" case.
    def f(m):
        s = m.group()
        return "".join(random.choice("xyzpq") if (ch.isalnum() and random.random() < 0.3) else ch
                       for ch in s)
    return qsig.RX_INLINE_MATH.sub(f, qsig.RX_DISPLAY_MATH.sub(f, text))


def e9():
    R.hdr("E9 - validate the 13% low-value classification")
    docs = qsig.all_docs()
    bykey = defaultdict(list)
    lv_chars = all_chars = 0
    for p in docs:
        t = qsig.load(p)
        all_chars += len(t)
        m = qsig.RX_LOWVALUE.search(p)
        if m:
            lv_chars += len(t)
            bykey[m.group(1).lower()].append(p)
    total = sum(len(v) for v in bykey.values())
    print(f"low-value docs: {total}   char-share of corpus: {100*lv_chars/all_chars:.1f}%")
    print("by matched keyword:")
    for k in sorted(bykey, key=lambda k: -len(bykey[k])):
        ex = os.path.relpath(bykey[k][0], qsig.EXTRACT_ROOT)
        print(f"  {k:14} {len(bykey[k]):4d}  eg: {ex}")
    rnd = random.Random(1)
    flat = [p for v in bykey.values() for p in v]
    print("-- content preview (first 90 chars, newlines stripped) - spot false positives --")
    for p in rnd.sample(flat, min(6, len(flat))):
        t = qsig.load(p).replace("\n", " ")[:90]
        print(f"   [{os.path.basename(p)[:34]:34}] {t}")


def e10():
    R.hdr("E10 - subtle garble near threshold + REAL mojibake docs + corrupt-but-balanced math")
    base = R.load_rel(R.GOOD_MATH[0])
    print(f"clean baseline gscore = {gscore(base):.3f}   (good/low-value max from E8 = 1.00)")
    print("synthetic intensity sweep:")
    for frac in (0.002, 0.005, 0.01, 0.02, 0.05, 0.15):
        ls = gscore(R.synth_letterspace(base, frac))
        mb = gscore(R.synth_mojibake(base, frac))
        print(f"   frac={frac:<6} letterspace={ls:8.3f}   mojibake={mb:8.3f}")
    mc = synth_math_corrupt(base)
    print(f"corrupt-but-balanced math (30% of math chars scrambled): gscore={gscore(mc):.3f}  "
          f"(stays low -> subtle math garble is INVISIBLE, like omission)")
    docs = qsig.all_docs()
    real = []
    for p in docs:
        t = qsig.load(p)
        rr = qsig.replacement_rate(t)
        if rr > 0:
            real.append((rr * 1000, gscore(t), p))
    real.sort(reverse=True)
    print(f"REAL docs with mojibake: {len(real)}  (top 8 with gscore):")
    for rr, gs, p in real[:8]:
        print(f"   ufffd/kc={rr:6.3f}  gscore={gs:7.3f}  {os.path.relpath(p, qsig.EXTRACT_ROOT)[:58]}")


def read_cache_text(coll, ref):
    import json
    p = f"/data/librarian-state/{coll}/cache/{ref[:2]}/{ref[2:]}"
    if not os.path.exists(p):
        return None
    try:
        j = json.loads(open(p, encoding="utf-8", errors="ignore").read())
    except Exception:
        return None
    if isinstance(j, dict) and isinstance(j.get("spans"), list):
        return "\n".join(s.get("text", "") for s in j["spans"])
    return None


def e11():
    R.hdr("E11 - garble prevalence on the REAL librarian cache: software vs particle-physics")
    import sqlite3
    for coll in ("software", "particle-physics"):
        c = sqlite3.connect(f"/data/librarian-state/{coll}/manifest.sqlite")
        rows = c.execute("select source_id,output_ref from manifest "
                         "where stage='extract' and status in ('Success','Cached') "
                         "and output_ref is not null").fetchall()
        read = mojibake = ls_hi = lowval = 0
        worst = []
        for sid, ref in rows:
            t = read_cache_text(coll, ref)
            if t is None:
                continue
            read += 1
            if qsig.replacement_rate(t) > 0:
                mojibake += 1
            d = qsig.letterspace_density(t)
            if d > 0.5:
                ls_hi += 1
            worst.append((d, sid))
            if qsig.is_low_value(sid):
                lowval += 1
        worst.sort(reverse=True)
        print(f"[{coll}] extract rows={len(rows)} read={read}")
        if read:
            print(f"   mojibake(any U+FFFD): {mojibake} ({100*mojibake/read:.1f}%)   "
                  f"letterspace>0.5/kc: {ls_hi} ({100*ls_hi/read:.1f}%)   "
                  f"low-value(src name): {lowval} ({100*lowval/read:.1f}%)")
            for d, sid in worst[:5]:
                print(f"      {d:6.2f}/kc  ...{sid[-52:]}")


if __name__ == "__main__":
    if "e11" in sys.argv:
        e11()
    else:
        e9()
        e10()
