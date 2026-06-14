# /data/corpus — CANONICAL LAYOUT  ⚠️ READ BEFORE INGESTING ANYTHING

> **THIS IS THE ONE TRUE CONVENTION. DO NOT RE-DERIVE IT FROM MEMORY.**
> A `source_id` is the **primary key** of every chunk in Qdrant (the point UUID is
> `uuidv5("{source_id}#{chunk_index}")`). If you invent a different path scheme, you
> create a *parallel, diverging* corpus that silently duplicates content and pollutes
> retrieval. That exact mess is what the 2026-06-13 migration cleaned up. **Don't recreate it.**
>
> The authority is **code, not this prose**: `canonical_source_id(corpus, path)` in the
> librarian (crate `cli`, `docs.rs`). `librarian ingest` calls it, so the tool always
> produces conforming ids. `librarian audit` flags any id that doesn't match. If you are
> hand-constructing a source_id, **stop** — run the tool instead. This doc explains *what*
> the function does so you can verify it, not so you can reimplement it.

## The rule in one line

```
source_id = <corpus>/<type>/<resource-slug>[/<chapter-slug>].<ext>      (relative to /data/corpus)
```

Relative — never absolute — so the corpus is portable across machines.

## Structure

```
/data/corpus/                                    ROOT (source_ids are relative to here)
  <corpus>/                                      = the Qdrant collection: software | particle-physics
    markdown/                                    INGESTED text (the only thing the text extractor embeds)
      <resource>/<chapter>.md                      chaptered work (book): one dir, one file per chapter
      <resource>.md                                single-unit work (paper): FLAT, no dir
    code/        <resource>/<files>              INGESTED code (code extractor)
    pdf/         <resource>.pdf                  RAW original — Marker input; not embedded directly*
    ebook/       <resource>.epub|.mobi|.azw3|.azw RAW; ingested in place by the ebook extractor
    html/        <resource>.html|.htm            RAW; ingested in place by the html extractor
```

\*Except a handful of legacy PDFs ingested raw before this convention (see Migration note).

## Type axis = the librarian's 5 extractors (and nothing else)

| `type/` dir | extractor | formats | embedded? |
|---|---|---|---|
| `markdown/` | text | `.md` | YES — canonical ingested form |
| `code/` | code | source files | YES — ingested in place |
| `pdf/` | (pdf→Marker) | `.pdf` | NO — extracted to `markdown/` first |
| `ebook/` | ebook | `.epub .mobi .azw3 .azw` | in place |
| `html/` | html | `.html .htm` | in place |

## Extraction policy — match effort to source fidelity

- **PDF** is a presentation format → run **Marker** (offline) to recover structure →
  output lands in `markdown/<resource>/…`. The raw `.pdf` stays in `pdf/` as regenerable input.
- **HTML / EPUB** are already semantic markup → ingest **in place** with their extractor.
  Do NOT round-trip them through Marker (adds cost, loses structure).
- **code / native .md** → ingest in place.

## Slug rule

`slug = lowercase; '__','_' and any non-alphanumeric run → '-'; collapse repeats; trim '-'`.
The shelf/topic prefix is **kept** in the slug (e.g. `networking-bonaventure`,
`fpga-design-recipes-for-fpgas`) — it is collision-safe and groups related works lexically.

## book vs paper is METADATA, never a directory

`content_type ∈ {book, paper, code}` is a **payload field, indexed in Qdrant** — filter on it
at query time. Do NOT create `pdf/books/` vs `pdf/papers/` dirs: that duplicates a fact that
already lives (and is queryable) in the payload, and the dir/payload can then disagree.

## Migration note (2026-06-13)

Collapsed three historical roots (`/data/books`, `/data/books-curated`, `/tmp/diag-ingest`)
+ absolute paths → this single relative scheme by rewriting Qdrant point ids + payloads
(vectors reused, no re-embed). Backup: `nasdog:/share/CACHEDEV1_DATA/backups/librarian/2026-06-13-pre-relpath-migration/`.
17 legacy PDFs remain `pdf/`-typed (ingested raw, pre-Marker); flagged for optional future
re-extraction to `markdown/`.
