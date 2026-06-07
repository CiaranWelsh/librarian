#!/usr/bin/env python3
"""Chunking experiment harness (issue 027). Prototype the best retrieval config with the
*real* LangChain splitters before porting the winner to Rust.

One config per run:  chunk a fixed sample corpus -> embed (text-embedding-3-large, 3072-d)
-> (re)create qdrant collection eval_<name> with a daemon-compatible payload -> upsert.
Score afterwards with  eval/run_eval.py --collection eval_<name>  (queries the live daemon).

Payload mirrors crates/adapter-indexer-qdrant (named vector "text"; payload keys
source_id/chunk_index/text/content_type) so the daemon reads the experiment collections
exactly like production. Run ON turbo (corpus, qdrant, OpenAI key all local there):

    set -a; . ~/.librarian/env; set +a
    python sweep.py --name e0_blankline --method blankline
    python sweep.py --name e1_rcts1200 --method rcts   --size 1200 --overlap 120
    python sweep.py --name e4_bc        --method rcts_md --size 1600 --overlap 160
"""
import argparse, glob, os, sys, time, uuid
from statistics import median

from openai import OpenAI
from qdrant_client import QdrantClient, models
from langchain_text_splitters import RecursiveCharacterTextSplitter, MarkdownHeaderTextSplitter

CORPUS = "/data/books-curated/ingest-text"
QDRANT_URL = "http://localhost:6333"
EMBED_MODEL = "text-embedding-3-large"
DIM = 3072

# Sample = golden-referenced books that exist as markdown (so eval has relevant targets).
# Substring-matched against filenames; every chapter of each book is included.
SAMPLE_BOOKS = [
    "Effective-Software-Testing", "Building-Microservices", "Microservices-Patterns",
    "Programming-Rust", "KubePatterns2", "Introduction-to-Algorithms",
    "GPU-Computing-Gems", "SWEBOK", "React-Application", "Forouzan",
    "Agile-Software-Development",
]

HEADERS = [("#", "h1"), ("##", "h2"), ("###", "h3")]


def collect_files():
    out = []
    for f in sorted(glob.glob(os.path.join(CORPUS, "*.md"))):
        base = os.path.basename(f)
        if any(b.lower() in base.lower() for b in SAMPLE_BOOKS):
            out.append(f)
    return out


def book_breadcrumb(filename):
    """Human breadcrumb base from the `<cat>_<Book>__<Chapter>.md` filename."""
    stem = os.path.basename(filename)[:-3]
    parts = stem.split("__")
    book = parts[0].split("_", 1)[-1]
    chapter = parts[1] if len(parts) > 1 else ""
    return f"{book} > {chapter}".strip(" >")


# --- chunkers: each returns a list[str] of chunk texts for one document ---

def chunk_blankline(text, _file, _size, _overlap):
    # Mirrors the Rust BlankLineChunker: one chunk per blank-line block, no size target.
    return [b.strip() for b in text.split("\n\n") if b.strip()]


def chunk_rcts(text, _file, size, overlap):
    splitter = RecursiveCharacterTextSplitter(
        chunk_size=size, chunk_overlap=overlap,
        separators=["\n\n", "\n", " ", ""], keep_separator=True,
    )
    return splitter.split_text(text)


def chunk_rcts_md(text, file, size, overlap):
    # Two-stage: split on markdown headers (breadcrumb), then recursive-pack within each
    # section, prepending the Book > Chapter > h1 > h2 > h3 breadcrumb to each chunk.
    md = MarkdownHeaderTextSplitter(headers_to_split_on=HEADERS, strip_headers=True)
    rcts = RecursiveCharacterTextSplitter(
        chunk_size=size, chunk_overlap=overlap,
        separators=["\n\n", "\n", " ", ""], keep_separator=True,
    )
    base = book_breadcrumb(file)
    out = []
    sections = md.split_text(text) or []
    if not sections:  # no headers in this file -> fall back to plain recursive
        return [f"{base}\n\n{c}" for c in rcts.split_text(text)]
    for sec in sections:
        crumb = " > ".join([base] + list(sec.metadata.values()))
        for piece in rcts.split_text(sec.page_content):
            out.append(f"{crumb}\n\n{piece}")
    return out


CHUNKERS = {"blankline": chunk_blankline, "rcts": chunk_rcts, "rcts_md": chunk_rcts_md}


def embed(client, texts, batch=128):
    vecs = []
    for i in range(0, len(texts), batch):
        chunk = texts[i:i + batch]
        resp = client.embeddings.create(model=EMBED_MODEL, input=chunk, dimensions=DIM)
        vecs.extend(d.embedding for d in resp.data)
        print(f"  embedded {min(i + batch, len(texts))}/{len(texts)}", end="\r", flush=True)
    print()
    return vecs


def build_collection(qc, name, records, vectors):
    if qc.collection_exists(name):
        qc.delete_collection(name)
    qc.create_collection(
        name,
        vectors_config={"text": models.VectorParams(size=DIM, distance=models.Distance.COSINE)},
    )
    points = []
    for (sid, idx, text), vec in zip(records, vectors):
        pid = str(uuid.uuid5(uuid.NAMESPACE_URL, f"{sid}#{idx}"))
        points.append(models.PointStruct(
            id=pid, vector={"text": vec},
            payload={"source_id": sid, "chunk_index": idx, "text": text, "content_type": "book"},
        ))
    for i in range(0, len(points), 256):
        qc.upsert(name, points=points[i:i + 256], wait=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--name", required=True, help="collection suffix -> eval_<name>")
    ap.add_argument("--method", required=True, choices=list(CHUNKERS))
    ap.add_argument("--size", type=int, default=1600, help="chunk_size in characters")
    ap.add_argument("--overlap", type=int, default=160, help="chunk_overlap in characters")
    args = ap.parse_args()

    files = collect_files()
    chunk = CHUNKERS[args.method]
    records, lengths = [], []
    for f in files:
        sid = os.path.basename(f)
        text = open(f, encoding="utf-8", errors="ignore").read()
        for idx, ctext in enumerate(chunk(text, f, args.size, args.overlap)):
            records.append((sid, idx, ctext))
            lengths.append(len(ctext))

    print(f"config={args.name} method={args.method} size={args.size} overlap={args.overlap}")
    print(f"files={len(files)}  chunks={len(records)}  "
          f"chars: median={int(median(lengths))} min={min(lengths)} max={max(lengths)}")

    client = OpenAI()
    qc = QdrantClient(url=QDRANT_URL)
    t = time.time()
    vectors = embed(client, [r[2] for r in records])
    build_collection(qc, f"eval_{args.name}", records, vectors)
    print(f"built eval_{args.name} ({len(records)} points) in {time.time() - t:.0f}s")


if __name__ == "__main__":
    main()
