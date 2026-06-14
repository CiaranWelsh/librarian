//! `librarian add --commit` write side (L-069): place the resource under the
//! corpus, then ingest it by reusing `cmd_ingest`. Placement copies by default
//! and moves with `--move`; the PDF flow is decoupled (issue 029) — the raw
//! `.pdf` is archived and the already-extracted Marker markdown is written to a
//! durable `.md`, which is then ingested through the TEXT extractor rather than
//! re-running Marker.
//!
//! Gate 2 (L-070) runs after a successful ingest: it measures retrieval health
//! against the collection's golden probe set, compares it to the recorded
//! history via `quality::is_regression`, and on a regression rolls the addition
//! back (removes the just-ingested points and deletes the placed files) unless
//! `--force`. The gate fails OPEN: if there is no golden set or the daemon is
//! unreachable it warns loudly and lets the commit stand, because the content is
//! already ingested and blocking on infra availability is too harsh (Gate 1
//! already guarded against garbled extractions before anything was written).

use std::path::{Path, PathBuf};

use adapter_indexer_qdrant::QdrantIndexer;
use librarian_domain::{ExtractedText, SourceId};

use crate::commands::http::Daemon;
use crate::commands::ingest::cmd_ingest;
use crate::commands::{health, remove};
use crate::config::Config;

use super::plan::{AddPlan, Kind};
use super::quality;

/// `k` for the Gate 2 health measurement, matching `cmd_health`'s default.
const GATE2_K: u64 = 10;

/// Place the planned resource and ingest it, then run Gate 2. `src` is the
/// original file (still at its source location); `text` is the extraction already
/// produced by the preview step (reused for the PDF markdown body, never re-run).
/// The collection comes from `cfg`; `config_root` locates the golden probe set and
/// health history; `force` keeps a regressing addition instead of rolling it back.
///
/// Before any placement it runs the idempotency pre-check: if the resource is
/// already in the collection it prints a `skip:` line and returns without writing,
/// unless `force` is set (in which case it re-ingests, overwriting in place).
///
/// Note: `source_id_prefix` is a *prefix*. For in-place kinds it equals the real
/// source id; for the dir-based kinds (Pdf, Markdown) the ingested point ids
/// extend it with the markdown filename (`<prefix>/<slug>.md`). Both the pre-check
/// and rollback go through `expected_source_ids`, which covers both shapes.
pub(crate) fn run(
    plan: &AddPlan,
    cfg: &Config,
    src: &Path,
    move_: bool,
    text: &ExtractedText,
    config_root: &Path,
    force: bool,
) -> Result<(), String> {
    // Idempotency pre-check: if the resource is already in the collection, skip
    // the whole place/ingest/gate cycle unless --force. With --force we fall
    // through and re-ingest; deterministic point ids mean that overwrites the
    // existing points in place rather than duplicating them.
    if !force {
        if let Some(n) = already_present(plan, cfg) {
            println!(
                "skip: {} already present in {} ({} points); re-run with --force to re-add",
                plan.source_id_prefix, cfg.collection, n
            );
            return Ok(());
        }
    }

    match plan.kind {
        Kind::Pdf => {
            // Archive the raw pdf, write the extracted markdown to a durable .md,
            // then ingest that markdown dir via text.toml (decoupled, issue 029).
            // plan.config_path is pdf.toml (used only to preview the extraction);
            // the markdown is ingested through the sibling text.toml.
            copy_or_move(src, &plan.raw_path, move_)?;
            let md_path = plan.ingest_path.join(format!("{}.md", plan.slug));
            write_markdown(&md_path, text)?;
            cmd_ingest(&ingest_config_path(plan), &plan.ingest_path)?;
        }
        Kind::Markdown => {
            // ingest_path is the dir <corpus>/<col>/markdown/<slug>; place the
            // source as <slug>.md inside it, then ingest the dir via text.toml.
            let md_path = plan.ingest_path.join(format!("{}.md", plan.slug));
            copy_or_move(src, &md_path, move_)?;
            cmd_ingest(&ingest_config_path(plan), &plan.ingest_path)?;
        }
        Kind::Ebook | Kind::Html | Kind::Code => {
            // In-place: ingest_path == raw_path is the file itself.
            copy_or_move(src, &plan.ingest_path, move_)?;
            cmd_ingest(&ingest_config_path(plan), &plan.ingest_path)?;
        }
    }

    gate2(plan, &cfg.collection, config_root, force)
}

/// Authoritative "already present?" check against qdrant: open the collection and
/// count points carrying any of this add's expected source ids. Returns the point
/// count if present, `None` if absent or if the check could not run.
///
/// Fails OPEN: the pre-check is a UX guard and an optimization, not a correctness
/// gate. A brand-new collection legitimately has nothing, and `open` cannot be
/// reached when qdrant is down — neither should block a commit, so an open error
/// is logged to stderr and treated as "not present, proceed".
fn already_present(plan: &AddPlan, cfg: &Config) -> Option<u64> {
    let indexer =
        match QdrantIndexer::open(&cfg.qdrant.url, &cfg.collection, cfg.embedder.dimension()) {
            Ok(i) => i,
            Err(e) => {
                eprintln!(
                    "note: idempotency pre-check skipped (qdrant open failed: {e}); proceeding"
                );
                return None;
            }
        };
    for id in expected_source_ids(plan) {
        match indexer.count_by_source(&SourceId(id)) {
            Ok(n) if n > 0 => return Some(n),
            Ok(_) => {}
            Err(e) => {
                eprintln!("note: idempotency pre-check skipped (count failed: {e}); proceeding");
                return None;
            }
        }
    }
    None
}

/// Post-ingest retrieval-health gate. Measures the now-larger collection against
/// its golden probe set, compares to the recorded history, and rolls back on a
/// regression unless `--force`. Returns `Ok` (commit stands) on pass, on a kept
/// `--force` regression, and on a fail-open skip; returns `Err` only after a
/// successful rollback. On the happy path / kept-force path it records the new
/// measurement in the history so future commits compare against it.
fn gate2(plan: &AddPlan, collection: &str, config_root: &Path, force: bool) -> Result<(), String> {
    let cc = crate::client_config::ClientConfig::load();
    let d = Daemon::new(&cc.resolve_daemon(None), cc.resolve_timeout(None));

    let golden_path = config_root.join(format!("golden_{collection}.json"));
    let history_path = config_root.join(format!("health_{collection}.jsonl"));

    if !golden_path.exists() {
        eprintln!(
            "WARNING: gate 2 skipped: no golden probe set at {}; \
             run `librarian health` to verify retrieval",
            golden_path.display()
        );
        return committed(plan);
    }

    let golden = health::load_golden(&golden_path)?;
    let after = match health::measure(&d, collection, &golden, GATE2_K) {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "WARNING: gate 2 skipped: health measurement failed ({e}); \
                 run `librarian health` to verify retrieval"
            );
            return committed(plan);
        }
    };

    let history = health::read_history(&history_path)?;
    match quality::is_regression(&history, &after, quality::DEFAULT_K_SIGMA) {
        Some(reason) if !force => {
            rollback(plan);
            Err(format!(
                "retrieval regressed after adding {}: {reason}; rolled back \
                 (points + files removed). Re-run with --force to keep it anyway.",
                plan.source_id_prefix
            ))
        }
        Some(reason) => {
            eprintln!("WARNING: gate 2 regression kept due to --force: {reason}");
            health::append_history(&history_path, collection, &after)?;
            committed(plan)
        }
        None => {
            println!(
                "gate 2 passed: hit-rate@{}={:.0}%  MRR={:.3}  fragment-rate@5={:.0}%",
                after.k,
                after.hit_rate * 100.0,
                after.mrr,
                after.fragment_rate * 100.0,
            );
            health::append_history(&history_path, collection, &after)?;
            committed(plan)
        }
    }
}

/// Report the commit and return `Ok`. The single place the "committed" line is
/// printed so every success path (pass, kept-force, fail-open skip) agrees.
fn committed(plan: &AddPlan) -> Result<(), String> {
    println!("committed {}", plan.source_id_prefix);
    Ok(())
}

/// Config the ingest runs through for this plan. PDF previews via `pdf.toml` but
/// ingests its durable markdown through the sibling `text.toml` (decoupled,
/// issue 029); every other kind ingests through its own config. Rollback resolves
/// the same config so place/ingest and remove never drift onto different qdrant
/// collections or manifests.
fn ingest_config_path(plan: &AddPlan) -> PathBuf {
    match plan.kind {
        Kind::Pdf => plan.config_path.with_file_name("text.toml"),
        _ => plan.config_path.clone(),
    }
}

/// Source ids this add creates. In-place kinds (Ebook/Html/Code) ingest under
/// the bare prefix; the dir kinds (Pdf/Markdown) ingest the single placed
/// markdown file, whose id extends the prefix with `<slug>.md`. These match what
/// `canonical_source_id` produced for the placed file. Serves both the
/// idempotency pre-check ("are these already present?") and rollback.
fn expected_source_ids(plan: &AddPlan) -> Vec<String> {
    match plan.kind {
        Kind::Ebook | Kind::Html | Kind::Code => vec![plan.source_id_prefix.clone()],
        Kind::Pdf | Kind::Markdown => {
            vec![format!("{}/{}.md", plan.source_id_prefix, plan.slug)]
        }
    }
}

/// Undo a regressing addition: remove the just-ingested points, then delete the
/// placed files. Best-effort and non-aborting — a failed file delete warns but
/// does not stop the rest, so a partial rollback never leaves points orphaned
/// behind un-deletable files (or vice versa).
fn rollback(plan: &AddPlan) {
    let config = ingest_config_path(plan);
    for source_id in expected_source_ids(plan) {
        if let Err(e) = remove::cmd_remove(&config, &source_id) {
            eprintln!("WARNING: rollback: removing points {source_id} failed: {e}");
        }
    }

    match plan.kind {
        Kind::Pdf => {
            super::delete_file(&plan.raw_path, "rollback");
            super::delete_file(
                &plan.ingest_path.join(format!("{}.md", plan.slug)),
                "rollback",
            );
            remove_empty_dir(&plan.ingest_path);
        }
        Kind::Markdown => {
            super::delete_file(
                &plan.ingest_path.join(format!("{}.md", plan.slug)),
                "rollback",
            );
            remove_empty_dir(&plan.ingest_path);
        }
        Kind::Ebook | Kind::Html | Kind::Code => {
            super::delete_file(&plan.ingest_path, "rollback");
        }
    }
}

/// Drop the single `<slug>` dir this add created (dir kinds only). Deliberately
/// one level — the parent type dir (e.g. `markdown/`) predates this add and may
/// hold sibling resources, so we never walk up. Ignores all errors: a non-empty
/// or missing dir is fine and must not noise the rollback. (`undo` walks further
/// because it reverses an arbitrary earlier add, not a fresh placement.)
fn remove_empty_dir(path: &Path) {
    let _ = std::fs::remove_dir(path);
}

/// Join the extraction's spans into a markdown body and write it. Today's only
/// caller is the single-span PDF flow, so the `\n` join is a no-op; revisit the
/// separator (markdown wants a blank line between blocks) if multi-span sources
/// ever reach here.
fn write_markdown(dest: &Path, text: &ExtractedText) -> Result<(), String> {
    create_parent(dest)?;
    let body = text
        .spans
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(dest, body).map_err(|e| format!("write {}: {}", dest.display(), e))
}

fn copy_or_move(src: &Path, dest: &Path, move_: bool) -> Result<(), String> {
    create_parent(dest)?;
    if move_ {
        std::fs::rename(src, dest)
            .map_err(|e| format!("move {} -> {}: {}", src.display(), dest.display(), e))?;
    } else {
        std::fs::copy(src, dest)
            .map_err(|e| format!("copy {} -> {}: {}", src.display(), dest.display(), e))?;
    }
    Ok(())
}

fn create_parent(dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create dir {}: {}", parent.display(), e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_with(kind: Kind) -> AddPlan {
        AddPlan {
            kind,
            slug: "programming-rust".into(),
            raw_path: PathBuf::from("/corpus/software/pdf/programming-rust.pdf"),
            ingest_path: PathBuf::from("/corpus/software/markdown/programming-rust"),
            source_id_prefix: "software/markdown/programming-rust".into(),
            config_path: PathBuf::from("/cfg/software/pdf.toml"),
        }
    }

    #[test]
    fn expected_ids_for_in_place_kind_are_the_bare_prefix() {
        let mut plan = plan_with(Kind::Ebook);
        plan.source_id_prefix = "software/ebook/async-rust.epub".into();
        assert_eq!(
            expected_source_ids(&plan),
            vec!["software/ebook/async-rust.epub".to_string()]
        );
    }

    #[test]
    fn expected_ids_for_dir_kind_extend_prefix_with_md() {
        let plan = plan_with(Kind::Pdf);
        assert_eq!(
            expected_source_ids(&plan),
            vec!["software/markdown/programming-rust/programming-rust.md".to_string()]
        );
    }
}
