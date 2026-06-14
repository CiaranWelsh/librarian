# `librarian add` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** One quality-gated command to add any supported resource to any collection, following the canonical corpus path automatically, with preview-by-default and easy rollback.

**Architecture:** New `add` subcommand orchestrates EXISTING pieces — it never reimplements ingest. Preview runs `Extractor::extract` + `Chunker::chunk` + `garble_signal`/`classify_section` directly (no embed, no write). Commit places the file canonically, calls the existing `run_ingest` path, then runs the health gate and rolls back via `BatchRunner::remove` if retrieval regresses.

**Tech Stack:** Rust, clap, the librarian-cli crate (binary), librarian-runner, librarian-domain quality module, the qdrant indexer. Build + test + deploy on turbo (`/data/librarian`, branch `feat/relpath-migration`).

**Spec:** `docs/superpowers/specs/2026-06-14-add-command-design.md`

---

## File structure
- Create `crates/cli/src/commands/add/mod.rs` — `cmd_add(args) -> Result<(), String>`; the orchestrator (preview vs commit vs undo).
- Create `crates/cli/src/commands/add/plan.rs` — pure `AddPlan` planner: `(path, collection, shelf, slug) -> AddPlan { kind, slug, raw_path, ingest_path, source_id, config_path }`. Inverse of `canonical_source_id`. Unit-tested.
- Create `crates/cli/src/commands/add/quality.rs` — Gate-2 helpers: `regression_threshold(history) -> Thresholds` (statistical) and `is_regression(before, after, thresholds) -> Option<Reason>`. Pure, unit-tested.
- Modify `crates/cli/src/main.rs` — add `Cmd::Add { … }` variant (after `Ingest`, ~line 74) + dispatch arm (~line 218).
- Modify `crates/cli/src/commands/mod.rs` — `pub mod add;`
- Reuse (no change): `docs.rs::canonical_source_id`, `ingest.rs::{run_ingest, select_chunker, the extractor dispatch}`, `commands/health.rs` (Gate-2 metric core), `commands/remove.rs` (undo/rollback), `librarian_domain::quality::{garble_signal, classify_section}`.

---

### Task 1: `add` subcommand skeleton + dispatch

**Files:** Modify `crates/cli/src/main.rs`; Create `crates/cli/src/commands/add/mod.rs`; Modify `crates/cli/src/commands/mod.rs`; Test: `crates/cli/tests/cli_integration.rs`.

- [ ] **Step 1 — failing test:** in `cli_integration.rs`, assert `librarian add --help` exits 0 and mentions `--to`, and `librarian add some.pdf` (no `--to`) exits non-zero.
```rust
#[test]
fn add_requires_to_flag() {
    Command::cargo_bin("librarian").unwrap().args(["add", "x.pdf"]).assert().failure();
    Command::cargo_bin("librarian").unwrap().args(["add", "--help"]).assert().success().stdout(contains("--to"));
}
```
- [ ] **Step 2 — run, expect FAIL** (`add` subcommand doesn't exist): `cargo test -p librarian-cli --test cli_integration add_requires_to_flag`.
- [ ] **Step 3 — implement:** add the clap variant + dispatch + module:
```rust
// main.rs, in enum Cmd:
/// Add a resource to a collection (quality-gated; preview by default).
Add {
    path: Option<PathBuf>,               // None only when --undo is used
    #[arg(long)] to: String,             // collection
    #[arg(long)] commit: bool,
    #[arg(long)] shelf: Option<String>,
    #[arg(long)] slug: Option<String>,
    #[arg(long)] r#move: bool,
    #[arg(long)] force: bool,
    #[arg(long)] judge: bool,
    #[arg(long)] undo: Option<String>,   // source_id to remove
},
// dispatch arm:
Cmd::Add { path, to, commit, shelf, slug, r#move, force, judge, undo } =>
    commands::add::cmd_add(commands::add::AddArgs { path, to, commit, shelf, slug, move_: r#move, force, judge, undo }),
```
`commands/add/mod.rs`: define `pub struct AddArgs {…}` and `pub fn cmd_add(a: AddArgs) -> Result<(), String> { todo via subsequent tasks }` returning `Ok(())` for now after printing a stub.
- [ ] **Step 4 — run, expect PASS.**
- [ ] **Step 5 — commit:** `[L-064: feat] add: subcommand skeleton + dispatch`.

### Task 2: Path planner (pure)

**Files:** Create `crates/cli/src/commands/add/plan.rs`; Test: inline `#[cfg(test)]` in that file.

- [ ] **Step 1 — failing tests** (cover each type + slug rules):
```rust
// software/pdf: raw under pdf/, ingest the derived markdown, source_id relative
#[test] fn plan_pdf() {
    let p = AddPlan::derive(Path::new("/x/Programming Rust.pdf"), "software", None, None).unwrap();
    assert_eq!(p.kind, Kind::Pdf);
    assert_eq!(p.raw_path, PathBuf::from("/data/corpus/software/pdf/programming-rust.pdf"));
    assert_eq!(p.ingest_path, PathBuf::from("/data/corpus/software/markdown/programming-rust")); // dir
    assert_eq!(p.config_path, PathBuf::from("/home/asi/.librarian/software/pdf.toml")); // or HOME-resolved
}
#[test] fn plan_epub_in_place() {
    let p = AddPlan::derive(Path::new("/x/Async Rust.epub"), "software", None, None).unwrap();
    assert_eq!(p.kind, Kind::Ebook);
    assert_eq!(p.ingest_path, PathBuf::from("/data/corpus/software/ebook/async-rust.epub"));
}
#[test] fn slug_override_and_shelf_prefix() {
    let p = AddPlan::derive(Path::new("/x/foo.md"), "software", Some("architecture"), Some("my-book")).unwrap();
    assert_eq!(p.ingest_path, PathBuf::from("/data/corpus/software/markdown/architecture-my-book"));
}
#[test] fn unknown_extension_errors() {
    assert!(AddPlan::derive(Path::new("/x/photo.png"), "software", None, None).is_err()); // image: v1 unsupported
}
```
- [ ] **Step 2 — run, expect FAIL.**
- [ ] **Step 3 — implement** `Kind` (Pdf/Ebook/Html/Code/Markdown), `slug()` (lowercase, non-alnum→`-`, collapse — same rule as the migration), `AddPlan::derive` building the canonical paths under `/data/corpus/<collection>/<type>/<slug>…` and the config path `~/.librarian/<collection>/<type>.toml` (text.toml for markdown/pdf-derived). `corpus_root` read from the resolved config (default `/data/corpus`).
- [ ] **Step 4 — run, expect PASS.**
- [ ] **Step 5 — commit:** `[L-065: feat] add: canonical path planner`.

### Task 3: Preview (Gate 1 + fragment stats, no write)

**Files:** Modify `crates/cli/src/commands/add/mod.rs`; Test: inline + a fixture.

- [ ] **Step 1 — failing test:** a clean markdown fixture previews with chunk count + 0% fragments + garble OK; a garbled fixture (`\u{FFFD}` heavy) reports flagged and would-abort. Test the pure helper `preview_quality(extracted, chunks) -> PreviewVerdict { chunks, fragment_rate, garble: GarbleSignal, gate1_pass }`.
```rust
#[test] fn preview_flags_garble() {
    let v = preview_quality_text("text \u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD} more", &GarbleConfig{flag_above:1.0});
    assert!(!v.gate1_pass);
}
```
- [ ] **Step 2 — run, expect FAIL.**
- [ ] **Step 3 — implement:** in `cmd_add`, when `!commit`: build extractor (reuse ingest.rs dispatch on `plan.kind`) + chunker, call `extract` then `chunk`, run `garble_signal`+`classify_section`, compute fragment-rate (reuse the health.rs fragment heuristic: chunk text < 80 chars or bare heading), print the PREVIEW verdict + planned paths. Abort (non-zero) if Gate 1 fails unless `--force`. NO embedder/indexer constructed.
- [ ] **Step 4 — run, expect PASS.**
- [ ] **Step 5 — commit:** `[L-066: feat] add: preview with Gate-1 + fragment stats`.

### Task 4: Gate-2 statistical regression (pure)

**Files:** Create `crates/cli/src/commands/add/quality.rs`; Test: inline.

- [ ] **Step 1 — failing tests:** given a health JSONL history (vec of past runs) + an after-run, `is_regression` flags a hit-rate/MRR drop or fragment-rate rise beyond `mean - k*stdev` (k configurable, default 2).
```rust
#[test] fn regression_when_hitrate_drops_beyond_sigma() {
    let hist = vec![run(1.0,0.78,0.0), run(1.0,0.77,0.0), run(0.99,0.78,0.0)];
    let after = run(0.80, 0.78, 0.0); // hit-rate craters
    assert!(is_regression(&hist, &after, 2.0).is_some());
}
#[test] fn no_regression_within_noise() {
    let hist = vec![run(1.0,0.78,0.0), run(0.99,0.77,0.0)];
    assert!(is_regression(&hist, &run(0.99,0.78,0.0), 2.0).is_none());
}
```
- [ ] **Step 2 — run, expect FAIL.**
- [ ] **Step 3 — implement** `mean`/`stdev` over history per metric; regression if `after.hit_rate < mean_hit - k*sd` or `after.mrr < mean_mrr - k*sd` or `after.fragment_rate > mean_frag + k*sd`. Return `Option<Reason>`.
- [ ] **Step 4 — run, expect PASS.**
- [ ] **Step 5 — commit:** `[L-067: feat] add: statistical Gate-2 regression detector`.

### Task 5: Commit path — place + ingest + Gate 2 + auto-rollback

**Files:** Modify `crates/cli/src/commands/add/mod.rs`; Test: `crates/cli/tests/add_e2e.rs` (gated on qdrant, like v1_e2e — self-skip if absent; use a temp corpus_root + stub embedder config).

- [ ] **Step 1 — failing test** (integration, self-skipping): `add --commit` a clean fixture into a temp corpus_root+collection → asserts `ok\t` + chunks>0 + the source_id is the canonical relative path; a fixture engineered to regress (reuse a tiny golden set) → asserts rollback (source absent afterwards).
- [ ] **Step 2 — run, expect FAIL.**
- [ ] **Step 3 — implement** the `--commit` branch: re-run preview gates; `copy` (or `move` if `--move`) the raw file to `plan.raw_path`; for pdf run the existing Marker step (the pdf extractor already does this in `run_ingest`); call `run_ingest(cfg, plan.ingest_path)` (reuse); read health golden+history (reuse health.rs), run health, call `is_regression`; if regression and not `--force`, `BatchRunner::remove(source_id)` + restore + report `⚠ REGRESSED — rolled back`; else append the health run to history and report `✓`. **QR-7:** if `--judge` (or chunk count > a configurable threshold), additionally run the existing `judge` logic (commands/judge.rs) on a probe and include its score in the verdict; off by default (OpenAI cost).
- [ ] **Step 4 — run, expect PASS** (or SKIP if no qdrant — verify locally on turbo).
- [ ] **Step 5 — commit:** `[L-068: feat] add: commit path with health gate + auto-rollback`.

### Task 6: Idempotency + `--undo` + `--force`

**Files:** Modify `crates/cli/src/commands/add/mod.rs`; Test: inline + add_e2e.

- [ ] **Step 1 — failing tests:** committing an already-present source_id skips unless `--force`; `add --undo <source_id> --to <col>` removes the points (reuse `cmd_remove`) and deletes the corpus files for that resource.
- [ ] **Step 2 — run, expect FAIL.**
- [ ] **Step 3 — implement:** query qdrant for the planned source_id (indexer count_by_source) → skip if present unless `--force`; `--undo` → `cmd_remove(config, source_id)` + `std::fs::remove` the resource's raw + markdown paths.
- [ ] **Step 4 — run, expect PASS.**
- [ ] **Step 5 — commit:** `[L-069: feat] add: idempotency + undo`.

### Task 7: Build, deploy, smoke-test on turbo

- [ ] **Step 1:** `cargo test -p librarian-cli --bins` (unit) + `cargo test -p librarian-cli --no-run` (compile integration) — all green.
- [ ] **Step 2:** `cargo install --path crates/cli` on turbo → updates `~/.cargo/bin/librarian`.
- [ ] **Step 3 — smoke (preview, no write):** `librarian add /some/sample.pdf --to software` → prints PREVIEW + canonical paths + chunk/frag stats, writes nothing. Verify `software` count unchanged.
- [ ] **Step 4 — smoke (commit + undo):** `add --commit` a throwaway fixture into `software`, verify it lands canonically + health held, then `add --undo <source_id>` and verify it's gone (count back to baseline).
- [ ] **Step 5 — commit + STOP for user sign-off before merging `feat/relpath-migration` → main** (per never-deploy-half-done).

---

## Notes
- Build/commit on turbo (`/data/librarian`, `feat/relpath-migration`); per source-of-truth rule. Editing happens on the Mac checkout then sync, or directly on turbo — executor's choice, but the branch is the constant.
- No co-author on commits; `[L-NN: feat]` tags.
- Marker must be working (`MARKER_BIN` was repointed to `/data/miniconda3/bin/marker_single`) for pdf commit smoke tests.
- Image ingestion is explicitly out of v1 (Task 2 errors on unknown extensions).
