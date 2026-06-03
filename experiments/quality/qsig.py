# qsig.py - candidate extraction-quality signals + corpus loader.
# Shared by the exploratory experiments. Signals are deliberately cheap and
# attempt to be domain-agnostic. The point is to discover, empirically, which
# (if any) of these separate garbled extraction from correctly-extracted
# (incl. math-dense) text, without false-rejecting the math.

import os
import re
import glob
from collections import Counter

EXTRACT_ROOT = "/data/books/extracted"

# --- span masking: remove math/code/tables so we can measure PROSE separately ---
RX_DISPLAY_MATH = re.compile(r"\$\$.*?\$\$", re.S)
RX_INLINE_MATH = re.compile(r"\$[^$\n]+\$")
RX_FENCE = re.compile(r"```.*?```", re.S)
RX_TABLE_ROW = re.compile(r"^\s*\|.*\|\s*$", re.M)


def mask_math_code(text):
    t = RX_FENCE.sub(" ", text)
    t = RX_DISPLAY_MATH.sub(" ", t)
    t = RX_INLINE_MATH.sub(" ", t)
    t = RX_TABLE_ROW.sub(" ", t)
    return t


# --- signals ---
RX_LETTERSPACE = re.compile(r"(?:\b[A-Za-z] ){3,}[A-Za-z]\b")
WORD = re.compile(r"[A-Za-z]{2,}")
TOKEN = re.compile(r"\S+")


def alpha_ratio(text):
    if not text:
        return 0.0
    return sum(c.isalpha() for c in text) / len(text)


def symbol_word_ratio(text):
    words = max(1, len(TOKEN.findall(text)))
    syms = sum(1 for c in text
               if not c.isalnum() and not c.isspace() and c not in ".,;:!?'\"()-")
    return syms / words


def replacement_rate(text):
    if not text:
        return 0.0
    return text.count("�") / len(text)


def letterspace_density(text):
    # letter-spaced runs per 1000 chars
    return 1000.0 * len(RX_LETTERSPACE.findall(text)) / max(1, len(text))


def mean_word_len(text):
    ws = WORD.findall(text)
    if not ws:
        return 0.0
    return sum(len(w) for w in ws) / len(ws)


_DICT = None


def load_dict():
    global _DICT
    if _DICT is None:
        _DICT = set()
        for p in ("/usr/share/dict/words", "/usr/share/dict/american-english"):
            if os.path.exists(p):
                with open(p, encoding="utf-8", errors="ignore") as f:
                    _DICT = set(w.strip().lower() for w in f)
                break
    return _DICT


def dict_hit_rate(text):
    d = load_dict()
    if not d:
        return None
    ws = [w.lower() for w in WORD.findall(text)]
    if not ws:
        return 0.0
    return sum(1 for w in ws if w in d) / len(ws)


def repetition(text):
    lines = [ln.strip() for ln in text.splitlines() if ln.strip()]
    if not lines:
        return 0.0
    c = Counter(lines)
    return sum(n for n in c.values() if n > 1) / len(lines)


def latex_issues(text):
    # 0 == clean. Higher == more malformed math markup.
    dd = text.count("$$")
    single = text.count("$") - 2 * dd
    issues = (dd % 2) + (abs(single) % 2)
    ob, cb = text.count("{"), text.count("}")
    if ob + cb:
        issues += abs(ob - cb) / (ob + cb)
    return issues


def signals(text, masked=False):
    t = mask_math_code(text) if masked else text
    s = {
        "alpha": round(alpha_ratio(t), 3),
        "sym/word": round(symbol_word_ratio(t), 3),
        "ufffd/kc": round(1000 * replacement_rate(text), 3),
        "lspace/kc": round(letterspace_density(text), 3),
        "meanwl": round(mean_word_len(t), 2),
        "rep": round(repetition(text), 3),
        "latex_iss": round(latex_issues(text), 2),
    }
    dh = dict_hit_rate(t)
    if dh is not None:
        s["dict"] = round(dh, 3)
    return s


RX_LOWVALUE = re.compile(
    r"(?i)(index|bibliography|references|contents|cover|title-page|"
    r"copyright|notation|glossary|acknowledg|about-the|half-title|list-of-)")


def is_low_value(path):
    return bool(RX_LOWVALUE.search(path))


def load(path):
    with open(path, encoding="utf-8", errors="ignore") as f:
        return f.read()


def all_docs():
    return glob.glob(os.path.join(EXTRACT_ROOT, "**", "*.md"), recursive=True)
