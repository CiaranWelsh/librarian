# inspect_cache.py - figure out the librarian extract-stage cache format so E11
# can read the real librarian extraction (incl. physics, which has no .md tree).
import sqlite3
import os
import json

MS = "/data/librarian-state/software/manifest.sqlite"
CACHE = "/data/librarian-state/software/cache"

c = sqlite3.connect(MS)
print("manifest columns:",
      [r[1] for r in c.execute("PRAGMA table_info(manifest)").fetchall()])
rows = c.execute(
    "select source_id,stage,status,output_ref from manifest "
    "where stage='extract' and output_ref is not null limit 5").fetchall()
for sid, stage, status, ref in rows:
    print(f"  {status:8} ref={ref!r}  src=...{sid[-46:]}")

ref = rows[0][3]
print("\nref repr:", repr(ref), "len:", len(ref) if ref else None)

# content-addressed layout seen in recon: cache/<k[:2]>/<k[2:]>
cands = []
if ref:
    cands.append(os.path.join(CACHE, ref[:2], ref[2:]))
    cands.append(os.path.join(CACHE, ref))
for cand in cands:
    print("try:", cand, "exists:", os.path.exists(cand))
    if os.path.exists(cand):
        raw = open(cand, "rb").read()
        print("  size:", len(raw))
        try:
            j = json.loads(raw)
            print("  json type:", type(j).__name__)
            if isinstance(j, dict):
                print("  keys:", list(j.keys())[:12])
                # common shape: {"spans":[{"text":...}]}
                sp = j.get("spans")
                if isinstance(sp, list) and sp:
                    print("  span0 keys:", list(sp[0].keys()))
                    print("  n spans:", len(sp), "first text:", repr(sp[0].get("text", ""))[:80])
            elif isinstance(j, list):
                print("  len:", len(j), "elem0:", type(j[0]).__name__)
                if isinstance(j[0], dict):
                    print("  elem0 keys:", list(j[0].keys()))
        except Exception as e:
            print("  not json:", e, "head:", raw[:100])
        break
