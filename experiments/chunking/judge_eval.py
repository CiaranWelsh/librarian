#!/usr/bin/env python3
"""LLM-judge retrieval eval (issue 027 — the unbiased arbiter).

Every heuristic metric we tried has a bias: source-level rewards chunk count (heading
lottery, D0); the chunk-level keyword+length proxy mildly favors big chunks (larger net /
length floor). This asks a judge model directly whether the retrieved passage *answers* the
question. It reads meaning, so it is immune to chunk length, breadcrumb gaming, keyword
nets, and source-substring quirks. RAGAS-style context relevance.

  answer@1 : mean judge score (0/1/2) of the rank-1 chunk
  hit@1    : fraction of questions whose rank-1 chunk DIRECTLY answers (score 2)
  hit@3    : fraction with a direct answer (score 2) anywhere in top-3
"""
import argparse, json, os, sys, urllib.request, urllib.error
from openai import OpenAI

JUDGE_MODEL = "gpt-4o-mini"
PROMPT = """You are grading a retrieval system. Given a question and a retrieved passage, judge whether the passage contains information that answers the question.

Question: {q}

Passage:
{p}

Scoring:
0 = irrelevant, or just a heading/fragment with no real content
1 = related/partial — touches the topic but does not actually answer it
2 = directly answers the question with substantive content

Reply with ONLY the single digit 0, 1, or 2."""


def search(daemon, collection, query, k):
    body = json.dumps({"collection": collection, "query": query, "limit": k}).encode()
    req = urllib.request.Request(daemon + "/v1/search", data=body,
                                 headers={"Content-Type": "application/json"}, method="POST")
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r).get("hits", [])


def judge(client, q, passage):
    r = client.chat.completions.create(
        model=JUDGE_MODEL, temperature=0, max_tokens=1,
        messages=[{"role": "user", "content": PROMPT.format(q=q, p=passage[:4000])}])
    s = (r.choices[0].message.content or "").strip()
    return int(s[0]) if s and s[0] in "012" else 0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("golden")
    ap.add_argument("--collection", required=True)
    ap.add_argument("--daemon", default=os.environ.get("LIBRARIAN_DAEMON", "http://100.127.138.48:6700"))
    ap.add_argument("--k", type=int, default=3)
    args = ap.parse_args()

    client = OpenAI()
    golden = json.load(open(args.golden))
    top1, hit1, hit3, rows = [], [], [], []
    for item in golden:
        hits = search(args.daemon, args.collection, item["q"], args.k)
        scores = [judge(client, item["q"], h.get("text", "")) for h in hits]
        s1 = scores[0] if scores else 0
        top1.append(s1)
        hit1.append(1 if s1 == 2 else 0)
        hit3.append(1 if any(s == 2 for s in scores) else 0)
        rows.append((scores, item["q"]))

    n = len(golden)
    print("=== LLM-judge  collection=%s  k=%d  judge=%s ===" % (args.collection, args.k, JUDGE_MODEL))
    print("answer@1 (mean 0-2): %.2f" % (sum(top1) / n))
    print("hit@1  (direct ans): %.0f%%" % (100 * sum(hit1) / n))
    print("hit@3  (direct ans): %.0f%%" % (100 * sum(hit3) / n))
    for scores, q in rows:
        print("  %s | %s" % (scores, q[:54]))


if __name__ == "__main__":
    main()
