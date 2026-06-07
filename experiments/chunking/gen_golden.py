#!/usr/bin/env python3
"""Generate gold-standard chunking vectors from the real LangChain splitters, for the Rust
port's TDD (issue 027). The Rust adapter must reproduce `expected` byte-for-byte.

  rcts: RecursiveCharacterTextSplitter.split_text  (separators ["\\n\\n","\\n"," ",""],
        keep_separator=True, char length) -- the core algorithm.
  md:   sweep.chunk_rcts_md  -- MarkdownHeaderTextSplitter (strip_headers) + RCTS within each
        section + "Book > Chapter > h1 > h2 > h3" breadcrumb prefix.

Run on turbo (has langchain):  .venv/bin/python gen_golden.py
"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import sweep
from langchain_text_splitters import RecursiveCharacterTextSplitter

SEPS = ["\n\n", "\n", " ", ""]

rcts_cases = [
    {"name": "three_paras_pack", "text": "alpha\n\nbeta\n\ngamma", "chunk_size": 13, "chunk_overlap": 0},
    {"name": "overlap_words", "text": "one two three four five six seven eight", "chunk_size": 15, "chunk_overlap": 5},
    {"name": "recurse_to_space", "text": "The quick brown fox jumps over the lazy dog near the river bank at dawn", "chunk_size": 30, "chunk_overlap": 8},
    {"name": "long_word_charsplit", "text": "supercalifragilisticexpialidocious", "chunk_size": 10, "chunk_overlap": 2},
    {"name": "mixed_markdownish", "text": "# Heading\n\nFirst paragraph here is short.\n\nSecond paragraph that is quite a bit longer than the first one and will need splitting.", "chunk_size": 50, "chunk_overlap": 10},
    {"name": "newline_recursion", "text": "line one\nline two\nline three\nline four\nline five", "chunk_size": 20, "chunk_overlap": 5},
    {"name": "empty", "text": "", "chunk_size": 50, "chunk_overlap": 10},
    {"name": "single_under_budget", "text": "short text", "chunk_size": 100, "chunk_overlap": 10},
]
rcts_out = []
for c in rcts_cases:
    sp = RecursiveCharacterTextSplitter(chunk_size=c["chunk_size"], chunk_overlap=c["chunk_overlap"],
                                        separators=SEPS, keep_separator=True)
    rcts_out.append({**c, "expected": sp.split_text(c["text"])})

md_cases = [
    {"name": "headers_basic", "file": "testing_Effective-Software-Testing__Chapter-09-Integration.md",
     "text": "# Chapter 9\n\nIntro to integration testing here.\n\n## Database tests\n\nWe submit SQL to the database and check the results returned.\n\n### DAO\n\nThe DAO class wraps the queries.",
     "chunk_size": 80, "chunk_overlap": 10},
    {"name": "no_headers", "file": "misc_Some-Book__Plain.md",
     "text": "Just plain text with no markdown headers at all, several sentences long to force packing into a couple of chunks perhaps.",
     "chunk_size": 50, "chunk_overlap": 10},
    {"name": "code_fence", "file": "lang_Some-Book__Ch1.md",
     "text": "# Title\n\nIntro text here.\n\n```\n# not a header\nsome code line\n```\n\nAfter the code block.",
     "chunk_size": 120, "chunk_overlap": 10},
    {"name": "header_pop", "file": "arch_Book__Ch2.md",
     "text": "# A\n\nUnder section A here.\n\n## B\n\nUnder subsection B.\n\n# C\n\nUnder section C only.",
     "chunk_size": 120, "chunk_overlap": 10},
]
md_out = []
for c in md_cases:
    md_out.append({**c, "expected": sweep.chunk_rcts_md(c["text"], c["file"], c["chunk_size"], c["chunk_overlap"])})

json.dump({"rcts": rcts_out, "md": md_out}, open("golden_vectors.json", "w"), indent=2, ensure_ascii=False)
print("wrote golden_vectors.json")
for c in rcts_out:
    print("rcts %-22s -> %d chunks" % (c["name"], len(c["expected"])))
for c in md_out:
    print("md   %-22s -> %d chunks" % (c["name"], len(c["expected"])))
