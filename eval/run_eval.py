#!/usr/bin/env python3
"""Retrieval eval harness for librarian.

For each golden question, calls the query daemon's /v1/search and scores:
  - hit-rate@k        : fraction of questions where a relevant source appears in top-k
  - MRR               : mean reciprocal rank of the first relevant-source hit
  - fragment-rate@5   : mean fraction of top-5 hits that are <80 chars or heading-ish
                        (lower is better; this is the signal the chunker swap should move)

A hit is "relevant" if its source_id contains any of the question's `relevant` substrings.
Source-level matching is deliberately forgiving and survives re-chunking, so the same
golden set can A/B two collections (current vs a re-chunked one).

Usage:
  python3 eval/run_eval.py [golden.json] [--daemon URL] [--k N] [--collection NAME]
Env: LIBRARIAN_DAEMON overrides the default daemon URL.
"""
import argparse, json, os, sys, urllib.request, urllib.error


def search(daemon, collection, query, k):
    body = json.dumps({"collection": collection, "query": query, "limit": k}).encode()
    req = urllib.request.Request(daemon + "/v1/search", data=body,
                                 headers={"Content-Type": "application/json"}, method="POST")
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r).get("hits", [])


def is_fragment(text):
    t = text.strip()
    return len(t) < 80 or t.startswith("#")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("golden", nargs="?", default=os.path.join(os.path.dirname(__file__), "golden.json"))
    ap.add_argument("--daemon", default=os.environ.get("LIBRARIAN_DAEMON", "http://turbo:6700"))
    ap.add_argument("--k", type=int, default=10)
    ap.add_argument("--collection", default=None, help="override the per-item collection")
    args = ap.parse_args()

    golden = json.load(open(args.golden))
    rows, hit_at_k, recip, frag = [], [], [], []

    for item in golden:
        coll = args.collection or item.get("collection", "software")
        rel = [s.lower() for s in item["relevant"]]
        try:
            hits = search(args.daemon, coll, item["q"], args.k)
        except (urllib.error.URLError, TimeoutError) as e:
            print("ERROR querying daemon: %s" % e, file=sys.stderr)
            sys.exit(1)
        rank = next((i + 1 for i, h in enumerate(hits)
                     if any(s in h["source_id"].lower() for s in rel)), None)
        hit_at_k.append(1 if rank else 0)
        recip.append(1.0 / rank if rank else 0.0)
        top5 = hits[:5]
        f = sum(is_fragment(h["text"]) for h in top5) / max(len(top5), 1)
        frag.append(f)
        rows.append((bool(rank), rank, f, item["q"]))

    n = len(golden)
    print("=== librarian retrieval eval  (n=%d, k=%d, daemon=%s) ===" % (n, args.k, args.daemon))
    print("hit-rate@%d:         %.0f%%" % (args.k, 100 * sum(hit_at_k) / n))
    print("MRR:                 %.3f" % (sum(recip) / n))
    print("fragment-rate@5:     %.0f%%   (lower is better)" % (100 * sum(frag) / n))
    print("\nhit | rank | frag@5 | question")
    for ok, rank, f, q in rows:
        print("  %s | %3s | %4.0f%% | %s" % ("Y" if ok else "n", rank if rank else "-", 100 * f, q[:58]))


if __name__ == "__main__":
    main()
