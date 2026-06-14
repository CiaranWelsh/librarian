//! `librarian add` — quality-gated single-resource ingest (L-065, L-069).
//!
//! Preview is the default: derive the canonical path plan, extract and chunk the
//! resource in memory, and report an intrinsic-quality verdict (Gate 1: garble,
//! plus advisory fragment / section stats) WITHOUT writing anything. A garbled
//! extraction aborts unless `--force`. With `--commit`, the same verdict is
//! rendered and then [`commit::run`] places the resource under the corpus and
//! ingests it. `--undo` reverses a previous add by source_id (`undo::run`).
//!
//! This module is a thin dispatcher; each flow has its own submodule (`plan`,
//! `preview`, `commit`, `quality`, `undo`), mirroring the one-file-per-command
//! shape of `commands/`.

mod commit;
mod plan;
mod preview;
pub(crate) mod quality;
mod undo;

use std::path::PathBuf;

use adapter_chunker_code::CodeChunker;
use adapter_extractor_code::CodeExtractor;
use adapter_extractor_ebook::EbookExtractor;
use adapter_extractor_html::HtmlExtractor;
use adapter_extractor_text::TextExtractor;
use librarian_domain::{Chunk, Chunker, Document, ExtractedText, Extractor};

use crate::commands::ingest::{pdf_extractor, select_chunker};
use crate::commands::output::Render;
use crate::config::Config;
use crate::docs::{doc_for_preview, parse_content_type};

use plan::{AddPlan, Kind as PlanKind};
use preview::{preview_quality, render_preview};

pub struct AddArgs {
    pub path: Option<PathBuf>,
    pub to: String,
    pub commit: bool,
    pub shelf: Option<String>,
    pub slug: Option<String>,
    pub move_: bool,
    pub force: bool,
    // `judge` is parsed and threaded now, but only the post-commit judge (Task 6)
    // consumes it; the preview and commit ignore it.
    #[allow(dead_code)]
    pub judge: bool,
    pub undo: Option<String>,
}

/// Thin dispatcher: do the shared setup (resolve the config, load it, derive the
/// full plan, build the preview doc, extract+chunk, score), ALWAYS render the
/// verdict, enforce gate 1, then commit or stop at the preview.
pub fn cmd_add(a: AddArgs, render: Render) -> Result<(), String> {
    // Undo needs no source path — dispatch it before requiring one.
    if let Some(id) = a.undo.as_deref() {
        return undo::run(id, &a.to, &config_root());
    }

    let src = a
        .path
        .as_deref()
        .ok_or("a file path is required (omit only with --undo)")?;

    // Resolve the per-collection config from the source name + config_root (neither
    // needs corpus_root), load it to learn corpus_root, then derive the full plan.
    let config_root = config_root();
    let (_, config_path) = AddPlan::config_path_for(src, &a.to, &config_root)?;
    let cfg = Config::load(&config_path).map_err(|e| {
        format!(
            "load config {}: {} (expected a per-collection config there)",
            config_path.display(),
            e
        )
    })?;
    let corpus_root = cfg.ingest.corpus_root.clone();

    let plan = AddPlan::derive(
        src,
        &a.to,
        a.shelf.as_deref(),
        a.slug.as_deref(),
        &corpus_root,
        &config_root,
    )?;

    let ct = parse_content_type(&cfg.ingest.content_type)?;
    let source_id = librarian_domain::SourceId(plan.source_id_prefix.clone());
    let doc = doc_for_preview(src, ct, source_id)?;

    let (text, chunks) = extract_and_chunk(&plan, &cfg, &doc)?;
    let q = cfg.quality.to_domain();
    let verdict = preview_quality(&plan.slug, &text, &chunks, &q);

    render_preview(render, &plan, &verdict, a.commit);

    if !verdict.gate1_pass && !a.force {
        return Err(format!(
            "{} looks garbled (garble value {:.3} > threshold {:.3}); nothing was written. \
             Re-run with --force to add it anyway.",
            src.display(),
            verdict.garble.value,
            q.garble.flag_above
        ));
    }

    if a.commit {
        commit::run(&plan, &cfg, src, a.move_, &text, &config_root, a.force)
    } else {
        Ok(()) // preview only; nothing was written
    }
}

/// Config root, honoring `LIBRARIAN_CONFIG_ROOT` first (integration tests point
/// it at a temp dir), then `$HOME/.librarian`, then `/var/lib/librarian/.librarian`.
fn config_root() -> PathBuf {
    if let Ok(root) = std::env::var("LIBRARIAN_CONFIG_ROOT") {
        return PathBuf::from(root);
    }
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/var/lib/librarian".into()))
        .join(".librarian")
}

/// Delete a placed corpus file, best-effort. `context` ("rollback" / "undo")
/// prefixes the warning. A missing file is not an error — it just means the
/// placement never happened or was already cleaned up. Shared by `commit`'s
/// rollback and `undo` so both report consistently.
fn delete_file(path: &std::path::Path, context: &str) {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("WARNING: {context}: {} already gone", path.display());
        }
        Err(e) => eprintln!(
            "WARNING: {context}: deleting {} failed: {e}",
            path.display()
        ),
    }
}

/// Run extraction then chunking for the planned resource kind. Dispatch is a match
/// over the kind into a generic helper, so there is no `Box<dyn Trait>` (project rule).
/// The text is returned alongside the chunks so the caller can run the garble signal
/// on it (`chunk` consumes the text by value).
fn extract_and_chunk(
    plan: &AddPlan,
    cfg: &Config,
    doc: &Document,
) -> Result<(ExtractedText, Vec<Chunk>), String> {
    match plan.kind {
        PlanKind::Pdf => run_extract_chunk(pdf_extractor(cfg), select_chunker(&cfg.ingest)?, doc),
        PlanKind::Ebook => {
            run_extract_chunk(EbookExtractor::new(), select_chunker(&cfg.ingest)?, doc)
        }
        PlanKind::Html => {
            run_extract_chunk(HtmlExtractor::new(), select_chunker(&cfg.ingest)?, doc)
        }
        PlanKind::Markdown => {
            run_extract_chunk(TextExtractor::new(), select_chunker(&cfg.ingest)?, doc)
        }
        PlanKind::Code => run_extract_chunk(CodeExtractor::new(), CodeChunker::new(), doc),
    }
}

fn run_extract_chunk<E, Ch>(
    extractor: E,
    chunker: Ch,
    doc: &Document,
) -> Result<(ExtractedText, Vec<Chunk>), String>
where
    E: Extractor,
    Ch: Chunker,
{
    let text = extractor
        .extract(doc)
        .map_err(|e| format!("extract: {e}"))?;
    let chunks = chunker
        .chunk(doc, text.clone())
        .map_err(|e| format!("chunk: {e}"))?;
    Ok((text, chunks))
}
