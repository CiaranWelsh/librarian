# Claude Guide — librarian

Hexagonal RAG framework (Rust). Collections `software` + `particle-physics` are
**production** (signed off 2026-06-07). Source of truth for content is the corpus
markdown on turbo (`/data/corpus/markdown/`), backed up to the NAS; the qdrant index is
derived state.

## The one rule that overrides convenience

**No addition or re-ingest to a production collection without running the quality gate
in `docs/quality-standard.md`** — extraction scan (garble + gzip), manifest
failures/flags, `librarian health` against the baselines, Tier-1 judge for large
additions. That document defines every metric, how to read it (including the
legitimately-degenerate genres: hardware pin tables, register manuals, proof
appendices), and the frozen baseline values. Update its baseline table when a gated
addition is accepted.

## Conventions

- No `Box<dyn>` — traits and generics (enum dispatch at composition roots).
- TDD; few, highly relevant, non-overlapping tests; `cargo fmt` before finishing.
- Issues in `docs/issues/NNN-name.md` — never committed. Commits tagged `[L-NNN: type]`,
  no co-author, and only with explicit consent.
- Marker is decoupled from ingest (issue 029/030): extraction is an offline step into
  the corpus; the pdf extractor's `[ingest.marker]` knobs exist for constrained GPUs
  (batch sizes are CLI flags — marker ignores env vars).

## Ops (turbo)

- Daemon: `librarian-serve` on 100.127.138.48:6700 (bare process; `~/.librarian/serve.toml`).
- Per-collection configs: `~/.librarian/<collection>/*.toml`; state in
  `/data/librarian-state/<collection>/` (cache is disposable, manifest is not).
- NAS backup: `bash /data/books/.staging/backup_corpus_to_nas.sh` (see the CLAUDE.md
  on the NAS share for its rules); golden sets + health history in `~/.librarian/`.
