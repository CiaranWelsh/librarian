#!/usr/bin/env python3
"""Chunk-level retrieval eval (issue 027 diagnostic).

run_eval.py matches at the SOURCE level, which rewards chunk COUNT: a book split into many
tiny chunks gets many chances to land *something* at rank 1 -- even a content-free heading
that just echoes the query (see D0). This metric instead asks whether a retrieved chunk is
actually ANSWER-BEARING: from a relevant source, containing the answer keyword(s), and
substantial (>= min_len chars, so headings/stubs don't qualify). A simple keyword+length
proxy for Chroma's token-level relevance.

  chunk-recall@k : fraction of questions with an answer-bearing chunk in top-k
  chunk-MRR      : mean reciprocal rank of the first answer-bearing chunk (the honest "is the
                   useful passage ranked high" number)
"""
import argparse, json, os, sys, urllib.request, urllib.error


def search(daemon, collection, query, k):
    body = json.dumps({"collection": collection, "query": query, "limit": k}).encode()
    req = urllib.request.Request(daemon + "/v1/search", data=body,
                                 headers={"Content-Type": "application/json"}, method="POST")
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r).get("hits", [])


def answer_bearing(hit, item, min_len):
    sid = hit.get("source_id", "").lower()
    text = hit.get("text", "")
    if not any(s.lower() in sid for s in item["relevant"]):
        return False
    if len(text) < min_len:
        return False
    t = text.lower()
    return all(kw.lower() in t for kw in item["keywords"])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("golden")
    ap.add_argument("--collection", required=True)
    ap.add_argument("--daemon", default=os.environ.get("LIBRARIAN_DAEMON", "http://100.127.138.48:6700"))
    ap.add_argument("--k", type=int, default=10)
    ap.add_argument("--min-len", type=int, default=200)
    args = ap.parse_args()

    golden = json.load(open(args.golden))
    rec, recip, rows = [], [], []
    for item in golden:
        try:
            hits = search(args.daemon, args.collection, item["q"], args.k)
        except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError) as e:
            print("ERROR (%s): %s" % (args.collection, e), file=sys.stderr)
            return
        rank = next((i + 1 for i, h in enumerate(hits) if answer_bearing(h, item, args.min_len)), None)
        rec.append(1 if rank else 0)
        recip.append(1.0 / rank if rank else 0.0)
        rows.append((rank, item["q"]))

    n = len(golden)
    print("=== chunk-level eval  collection=%s  k=%d  min_len=%d ===" % (args.collection, args.k, args.min_len))
    print("chunk-recall@%d:  %.0f%%" % (args.k, 100 * sum(rec) / n))
    print("chunk-MRR:        %.3f" % (sum(recip) / n))
    print("rank | question")
    for rank, q in rows:
        print("  %3s | %s" % (rank if rank else "-", q[:58]))


if __name__ == "__main__":
    main()
