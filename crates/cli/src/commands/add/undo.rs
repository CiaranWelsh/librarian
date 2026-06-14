//! `librarian add --undo <source_id>` (L-071): reverse a previous add. Removes
//! the source's points and deletes the placed corpus file, mirroring the commit
//! rollback but keyed off a source_id the user supplies rather than a live plan.
//!
//! The removal config is always the collection's `text.toml`: every collection
//! has one, and it carries the qdrant url, collection name, and corpus_root that
//! locate both the points and the placed files. File deletes are best-effort —
//! a missing file warns but does not fail, so a half-undone state (points gone,
//! a stray file) never blocks completing the rest.

use std::path::Path;

use crate::commands::remove::cmd_remove;
use crate::config::Config;

/// Undo the add identified by `source_id` in `collection`. `config_root` locates
/// the per-collection `text.toml`.
pub(crate) fn run(source_id: &str, collection: &str, config_root: &Path) -> Result<(), String> {
    let text_config = config_root.join(collection).join("text.toml");
    if !text_config.exists() {
        return Err(format!(
            "no text.toml for collection {} at {} (every collection needs one to undo)",
            collection,
            text_config.display()
        ));
    }

    // Load the config first, so a broken text.toml fails BEFORE we remove any
    // points (avoiding a half-undone state: points gone but the file left behind).
    let cfg = Config::load(&text_config).map_err(|e| {
        format!(
            "load config {}: {} (needed to locate the corpus file)",
            text_config.display(),
            e
        )
    })?;
    let corpus_root = &cfg.ingest.corpus_root;

    cmd_remove(&text_config, source_id)?;

    let placed = corpus_root.join(source_id);
    super::delete_file(&placed, "undo");
    remove_empty_parents(&placed, corpus_root);

    // Symmetry with the decoupled PDF flow: a markdown source_id like
    // `<col>/markdown/<slug>/<slug>.md` has an archived raw pdf at
    // `<col>/pdf/<slug>.pdf`. Reconstruct <slug> from the dir after `markdown/`
    // and remove the archived pdf too, best-effort.
    if let Some(slug) = markdown_slug(source_id) {
        let archived_pdf = corpus_root
            .join(collection)
            .join("pdf")
            .join(format!("{slug}.pdf"));
        if archived_pdf.exists() {
            super::delete_file(&archived_pdf, "undo");
        }
    }

    println!("undone {source_id}");
    Ok(())
}

/// If `source_id` is `<col>/markdown/<slug>/...`, return `<slug>` (the dir
/// component right after `markdown/`). Otherwise `None`.
fn markdown_slug(source_id: &str) -> Option<&str> {
    let mut parts = source_id.split('/');
    let _col = parts.next()?;
    if parts.next()? != "markdown" {
        return None;
    }
    parts.next()
}

/// Walk up from the deleted file removing empty parent dirs, stopping at (and not
/// removing) `corpus_root`. A non-empty dir ends the walk — `remove_dir` fails on
/// it and that is fine.
fn remove_empty_parents(start: &Path, corpus_root: &Path) {
    let mut dir = start.parent();
    while let Some(d) = dir {
        if d == corpus_root || std::fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_slug_extracts_dir_after_markdown() {
        assert_eq!(
            markdown_slug("software/markdown/programming-rust/programming-rust.md"),
            Some("programming-rust")
        );
    }

    #[test]
    fn markdown_slug_none_for_non_markdown_ids() {
        assert_eq!(markdown_slug("software/ebook/async-rust.epub"), None);
        assert_eq!(markdown_slug("software/code/main.rs"), None);
    }
}
