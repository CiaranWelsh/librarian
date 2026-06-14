# ADR-0007: Relative source_id scheme + canonical corpus layout

**Status:** Accepted — migrated 2026-06-14 (code enforcement pending, see Consequences)

## Context
`source_id` was an **absolute filesystem path** (`SourceId(path.display())` in `cli/src/docs.rs`),
and the point UUID is `uuidv5("{source_id}#{chunk_index}")` — so identity was tied to one
machine's filesystem layout *and* to each file's location. The corpus had also accreted across
three roots (`/data/books` PoC, `/data/books-curated`, `/data/corpus`) with flat
`<shelf>_<Book>__<Chapter>.md` names. Result: the live `software` index referenced **four** roots
(~66% books-curated, ~21% corpus, ~13% books, ~0.6% `/tmp`), with cross-root duplicate content
polluting retrieval, and nothing portable across machines.

## Decision
1. **`source_id` is a path relative to a single corpus root (`/data/corpus`)**, shaped
   `<corpus>/<type>/<resource>[/<chapter>].<ext>`. UUID derivation is unchanged; only the seed
   becomes relative — so the index is portable.
2. **Canonical layout** (deployed as `/data/corpus/LAYOUT.md`, mirrored here as `docs/corpus-layout.md`):
   - `<corpus>/markdown/<resource>/<chapter>.md` (chaptered) or `<corpus>/markdown/<resource>.md` (single-unit paper)
   - `<corpus>/{pdf,ebook,html,code}/<resource>…` — the five extractor types
   - **book vs paper = `content_type` payload metadata, never a directory level** (it's an indexed, queryable field)
   - **extraction effort matches source fidelity:** PDF → Marker → markdown; html/epub/code/native-md ingest in place
3. `LAYOUT.md` is THE convention — do not re-derive it from prose.

## Migration (2026-06-14)
- Rewrote every live point's id + payload to a relative `source_id`, **reusing vectors (no re-embed)**:
  software 141,711 pts, particle-physics 13,104 pts; 0 collisions; 900 `/tmp` orphans dropped.
- Reorganised files into the new layout; consolidated indexed raw PDFs into `<corpus>/pdf/`;
  un-indexed PDFs → `/data/pdf-archive-do-not-use/`; redundant copies deleted.
- Retired the `library` PoC collection (snapshot on NAS) + the books-mcp prototype
  (`/data/books`, `~/Documents/books`, mcp-server, MCP registration, old ingest skills).
- Relocated qdrant out of the deleted prototype dir to `/data/qdrant` (docker, `restart: unless-stopped`).
- Single restore point: NAS `backups/librarian/2026-06-14-clean-state/` + `RESTORE.md`.

## Consequences
- **Portable, single-root, de-duplicated.** Cross-root duplicates and machine-tied paths are gone.
- **Enforcement is PENDING (the divergence firewall).** `cli/src/docs.rs` still sets
  `source_id = path.display()` (absolute). Until a `canonical_source_id(path, corpus_root)` change
  lands and `ingest` calls it, a future `librarian ingest` will produce non-conforming absolute
  source_ids and re-open the drift this ADR closed. This ADR records the *convention*; the code
  change is the required follow-up.
- **Marker is broken** post-migration (`~/.librarian/env` `MARKER_BIN` points at the deleted
  mcp-server venv) — reinstall before any PDF ingest.
