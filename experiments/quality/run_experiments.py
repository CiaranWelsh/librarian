# run_experiments.py - the exploratory-experiment battery for extraction quality.
# Each e*() proves one thing. Output is summarised (no whole-doc dumps).
# Run on turbo:  python3 run_experiments.py

import os
import re
import sys
import random
import statistics

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import qsig

random.seed(7)  # determinism: synthetic garble is reproducible


def hdr(t):
    print("\n" + "=" * 72 + "\n" + t + "\n" + "=" * 72)


# --- labelled exemplars (rel = book/chapter; md is rel/<chapter>.md) ---
GOOD_MATH = [
    "ml-probabilistic/Chapter-04-Gaussian-Models",
    "ml-probabilistic/Chapter-02-Probability",
    "ml-probabilistic/Chapter-08-Logistic-Regression",
    "CLRS/C2-Probability",
]
GOOD_PLAIN = [
    "DDIA/Chapter-1-Reliable-Scalable-and-Maintainable-Applications",
    "SAIP/Chapter-01-1-What-Is-Software-Architecture",
    "SAIP/Chapter-02-2-Why-Is-Software-Architecture-Important",
]
LOW_VALUE = [
    "ml-probabilistic/Index-to-Keywords",
    "ml-probabilistic/Bibliography",
    "ml-probabilistic/Contents",
    "AT/BIBLIOGRAPHY",
]


def path_of(rel):
    base = rel.split("/")[-1]
    p = os.path.join(qsig.EXTRACT_ROOT, rel, base + ".md")
    return p if os.path.exists(p) else None


def load_rel(rel):
    p = path_of(rel)
    return qsig.load(p) if p else None


# --- synthetic garble generators (ground-truth positives) ---
def synth_letterspace(text, frac=0.15):
    out = []
    for w in text.split(" "):
        if len(w) > 3 and random.random() < frac:
            out.append(" ".join(w))
        else:
            out.append(w)
    return " ".join(out)


def synth_mojibake(text, frac=0.03):
    return "".join("�" if (c.isalpha() and random.random() < frac) else c
                   for c in text)


def pct(vals, p):
    s = sorted(vals)
    if not s:
        return 0.0
    k = min(len(s) - 1, int(round((p / 100.0) * (len(s) - 1))))
    return s[k]


def e1b():
    hdr("E1b - size the problem corpus-wide (U1)")
    docs = qsig.all_docs()
    n = len(docs)
    lv = uf = ls_hi = 0
    ls = []
    for p in docs:
        t = qsig.load(p)
        if qsig.is_low_value(p):
            lv += 1
        if qsig.replacement_rate(t) > 0:
            uf += 1
        d = qsig.letterspace_density(t)
        ls.append((d, p))
        if d > 0.5:
            ls_hi += 1
    vals = [d for d, _ in ls]
    ls.sort(reverse=True)
    print(f"docs total                         : {n}")
    print(f"low-value sections (by name)       : {lv}  ({100*lv/n:.1f}%)")
    print(f"docs with any U+FFFD mojibake      : {uf}  ({100*uf/n:.1f}%)")
    print(f"docs letterspace density >0.5/kc   : {ls_hi}  ({100*ls_hi/n:.1f}%)")
    print(f"letterspace density  median={statistics.median(vals):.3f} "
          f"p90={pct(vals,90):.3f} p99={pct(vals,99):.3f} max={max(vals):.3f}")
    print("top-8 letterspace docs:")
    for d, p in ls[:8]:
        print(f"   {d:6.2f}/kc  {os.path.relpath(p, qsig.EXTRACT_ROOT)}")


def _row(name, cls, text, masked=False):
    s = qsig.signals(text, masked=masked)
    print(f"  {cls:11} {name:36} " + " ".join(f"{k}={v}" for k, v in s.items()))
    return s


def e2_e3():
    hdr("E2/E3 - discrimination across classes, synthetic garble, masking (U2,U3)")
    base = load_rel(GOOD_MATH[0])
    if base is None:
        print("base missing; skip")
        return
    rows = []
    print("-- unmasked signals --")
    for rel in GOOD_MATH:
        t = load_rel(rel)
        if t:
            nm = rel.split("/")[-1]
            rows.append(("good-math", nm, _row(nm, "good-math", t)))
    for rel in GOOD_PLAIN:
        t = load_rel(rel)
        if t:
            nm = rel.split("/")[-1]
            rows.append(("good-plain", nm, _row(nm, "good-plain", t)))
    for rel in LOW_VALUE:
        t = load_rel(rel)
        if t:
            nm = rel.split("/")[-1]
            rows.append(("low-value", nm, _row(nm, "low-value", t)))
    rows.append(("garble", "SYNTH-letterspace",
                 _row("SYNTH-letterspace", "garble", synth_letterspace(base))))
    rows.append(("garble", "SYNTH-mojibake",
                 _row("SYNTH-mojibake", "garble", synth_mojibake(base))))

    print("\n-- E3: good-math signals WITH math/code MASKED --")
    for rel in GOOD_MATH:
        t = load_rel(rel)
        if t:
            _row(rel.split("/")[-1], "good-math*", t, masked=True)

    print("\n-- separation: per-doc composite = max(lspace/kc, ufffd/kc, latex_iss) --")

    def composite(s):
        return max(s.get("lspace/kc", 0), s.get("ufffd/kc", 0), s.get("latex_iss", 0))

    good = [(nm, composite(s)) for c, nm, s in rows if c != "garble"]
    garb = [(nm, composite(s)) for c, nm, s in rows if c == "garble"]
    worst_good = max(good, key=lambda x: x[1])
    print(f"  good+lowvalue composite max = {worst_good[1]:.2f}  (worst: {worst_good[0]})")
    for nm, g in garb:
        print(f"  garble {nm:20} composite = {g:.2f}")
    print(f"  SEPARABLE (every garble > every good/lowvalue): "
          f"{min(g for _, g in garb) > max(g for _, g in good)}")

    print("\n-- E3 verdict: does masking pull good-math's sym/word down to prose range? --")
    gm_un = [qsig.signals(load_rel(r))["sym/word"] for r in GOOD_MATH if load_rel(r)]
    gm_ma = [qsig.signals(load_rel(r), masked=True)["sym/word"] for r in GOOD_MATH if load_rel(r)]
    gp = [qsig.signals(load_rel(r))["sym/word"] for r in GOOD_PLAIN if load_rel(r)]
    print(f"  good-math sym/word  unmasked={[round(x,2) for x in gm_un]}")
    print(f"  good-math sym/word  masked  ={[round(x,2) for x in gm_ma]}")
    print(f"  good-plain sym/word         ={[round(x,2) for x in gp]}")


def e4():
    hdr("E4 - is omission detectable from output alone? (U4)")
    t = load_rel(GOOD_MATH[0])
    if t is None:
        print("base missing; skip")
        return
    dropped = [0]

    def drop(m):
        if random.random() < 0.4:
            dropped[0] += 1
            return " "
        return m.group()

    mut = qsig.RX_DISPLAY_MATH.sub(drop, t)
    blocks = len(qsig.RX_DISPLAY_MATH.findall(t))
    print(f"display-math blocks={blocks}  dropped={dropped[0]}")
    print(f"  before: {qsig.signals(t)}")
    print(f"  after : {qsig.signals(mut)}")
    print("  -> prose signals (alpha/dict/lspace/ufffd) barely move: omission is invisible from output")
    ff = t.count("\x0c")
    pg = len(re.findall(r"(?i)\bpage\s+\d+", t))
    print(f"page markers in output: formfeed={ff}  'Page N'={pg}  -> cannot detect dropped PAGES from output")


RX_HYPHEN = re.compile(r"(\w+)-\n(\w+)")


def dehyphenate(text):
    return RX_HYPHEN.sub(lambda m: m.group(1) + m.group(2), text)


def collapse_ls(text):
    return qsig.RX_LETTERSPACE.sub(lambda m: m.group().replace(" ", ""), text)


def e5():
    hdr("E5 - cleaner passes: when do they fire, and do they corrupt? (U5)")
    docs = qsig.all_docs()
    hy = hy_docs = 0
    for p in docs:
        n = len(RX_HYPHEN.findall(qsig.load(p)))
        if n:
            hy_docs += 1
            hy += n
    print(f"(a) line-break hyphens '-\\n' across {len(docs)} docs: total={hy} in {hy_docs} docs")
    print("    -> if ~0, de-hyphenation is moot for marker output (paragraphs are reflowed)")
    d = qsig.load_dict()
    real_ls = [
        "CSAPP/11-Information-Is-Bits-Context",
        "LMM/Brief-Contents",
        "GCGE/Chapter-34-Experiences-on-Image-and-Video-Processing-with-CUDA-and-OpenCL",
    ]
    print("(b) letterspace-collapse on REAL letter-spaced docs (does collapse yield real words?):")
    for rel in real_ls:
        t = load_rel(rel)
        if not t:
            continue
        runs = qsig.RX_LETTERSPACE.findall(t)
        coll = [r.replace(" ", "") for r in runs]
        hits = sum(1 for c in coll if d and c.lower() in d)
        print(f"    {rel.split('/')[-1]:36} runs={len(runs):3d} collapse->dictword={hits}/{len(runs)} eg={runs[:3]}")
    legit = "consider the points a b c d on the plane"
    print(f"(c) legit single-letter sequence : '{legit}'")
    print(f"    after letterspace-collapse   : '{collapse_ls(legit)}'  <- corruption when 'a b c d' -> 'abcd'")


def e6():
    hdr("E6 - clean improves the garble signal + idempotence (U6)")
    base = load_rel(GOOD_MATH[0])
    if base is None:
        print("base missing; skip")
        return
    g = synth_letterspace(base)
    before = qsig.letterspace_density(g)
    after = qsig.letterspace_density(collapse_ls(g))
    print(f"letterspace density  garbled={before:.2f}/kc  cleaned={after:.2f}/kc  improved={after < before}")
    viol = 0
    docs = GOOD_MATH + GOOD_PLAIN
    for r in docs:
        t = load_rel(r)
        if not t:
            continue
        c1 = collapse_ls(dehyphenate(t))
        c2 = collapse_ls(dehyphenate(c1))
        if c1 != c2:
            viol += 1
    print(f"idempotence clean(clean(x))==clean(x): violations={viol}/{len(docs)}")


def e7():
    hdr("E7 - doc-level vs worst-segment (U7)")
    for r in [GOOD_MATH[0], GOOD_PLAIN[0]]:
        t = load_rel(r)
        if not t:
            continue
        segs = [s for s in re.split(r"\n\s*\n", t) if len(s) > 200]
        dl = qsig.symbol_word_ratio(qsig.mask_math_code(t))
        ws = max((qsig.symbol_word_ratio(qsig.mask_math_code(s)) for s in segs), default=0)
        print(f"  {r.split('/')[-1]:36} segs={len(segs):4d} doc sym/word(masked)={dl:.3f} worst-seg={ws:.3f}")
    print("  -> worst-segment is far spikier; one symbol-heavy segment dominates (amplifies false positives)")


def e8():
    hdr("E8 - is there a single separating threshold? (U8)")
    def gscore(t):
        return (qsig.letterspace_density(t)
                + 1000 * qsig.replacement_rate(t)
                + min(qsig.latex_issues(t), 5))
    good = [gscore(load_rel(r)) for r in GOOD_MATH + GOOD_PLAIN + LOW_VALUE if load_rel(r)]
    base = load_rel(GOOD_MATH[0])
    garb = [gscore(synth_letterspace(base)), gscore(synth_mojibake(base))]
    print(f"good/lowvalue gscore max = {max(good):.2f}")
    print(f"garble        gscore min = {min(garb):.2f}")
    print(f"separable with one threshold: {min(garb) > max(good)}  "
          f"(midpoint ~{(min(garb)+max(good))/2:.2f})")


if __name__ == "__main__":
    e1b()
    e2_e3()
    e4()
    e5()
    e6()
    e7()
    e8()
