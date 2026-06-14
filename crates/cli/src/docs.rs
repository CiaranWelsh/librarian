//! Source-discovery helpers: walk the input path and produce `Document` records.

use adapter_extractor_code::{should_include, DEFAULT_INCLUDE_EXTS, DEFAULT_SKIP_DIRS};
use librarian_domain::{ContentType, Document, SourceHash, SourceId};
use librarian_runner::Outcome;
use sha2::{Digest, Sha256};
use std::path::Path;

/// Map the `[ingest] content_type` string to the domain enum. Shared between
/// `collect_docs` and the `add` preview so the two stay in lockstep.
pub(crate) fn parse_content_type(content_type: &str) -> Result<ContentType, String> {
    match content_type {
        "book" => Ok(ContentType::Book),
        "paper" => Ok(ContentType::Paper),
        "code" => Ok(ContentType::Code),
        other => Err(format!("unknown content_type: {other}")),
    }
}

pub fn collect_docs(
    input: &Path,
    content_type: &str,
    extractor: &str,
    corpus_root: &Path,
) -> Result<Vec<Document>, String> {
    let ct = parse_content_type(content_type)?;
    let code_mode = extractor == "code";
    let ebook_mode = extractor == "ebook";
    let html_mode = extractor == "html";
    let mut docs = Vec::new();
    if input.is_file() {
        docs.push(make_doc(input, ct, corpus_root)?);
    } else {
        for entry in walkdir::WalkDir::new(input)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if code_mode && !should_include(path, DEFAULT_SKIP_DIRS, DEFAULT_INCLUDE_EXTS) {
                continue;
            }
            if ebook_mode && !has_ebook_ext(path) {
                continue;
            }
            if html_mode && !has_html_ext(path) {
                continue;
            }
            docs.push(make_doc(path, ct, corpus_root)?);
        }
    }
    Ok(docs)
}

fn has_ebook_ext(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("epub" | "mobi" | "azw3" | "azw")
    )
}

fn has_html_ext(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("html" | "htm")
    )
}

fn make_doc(path: &Path, ct: ContentType, corpus_root: &Path) -> Result<Document, String> {
    let source_id = canonical_source_id(path, corpus_root)?;
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    let hash = SourceHash(hex::encode(Sha256::digest(&bytes)));
    Ok(Document {
        source_id,
        source_hash: hash,
        content_type: ct,
        path: path.to_path_buf(),
        work_id: None,
    })
}

/// Build a `Document` for an `add` preview, where the source file is still at its
/// ORIGINAL location (not yet placed under the corpus) so `canonical_source_id`
/// cannot derive the key. The caller supplies the planned `source_id`
/// (`AddPlan::source_id_prefix`); `path` stays the original file so extraction
/// reads the real bytes. Same Sha256 source-hash as `make_doc`.
pub(crate) fn doc_for_preview(
    path: &Path,
    ct: ContentType,
    source_id: SourceId,
) -> Result<Document, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    let hash = SourceHash(hex::encode(Sha256::digest(&bytes)));
    Ok(Document {
        source_id,
        source_hash: hash,
        content_type: ct,
        path: path.to_path_buf(),
        work_id: None,
    })
}

/// ADR-0007: `source_id` (and thus the point UUID) is the path RELATIVE to `corpus_root`,
/// so identity is portable across machines and tied to the resource, not its absolute
/// location. Files outside the corpus are rejected — the tool computes the canonical key;
/// the only human responsibility is correct placement (see /data/corpus/LAYOUT.md).
fn canonical_source_id(path: &Path, corpus_root: &Path) -> Result<SourceId, String> {
    match path.strip_prefix(corpus_root) {
        Ok(rel) => Ok(SourceId(rel.to_string_lossy().into_owned())),
        Err(_) => Err(format!(
            "refusing to ingest {}: not under corpus_root {} — place it under the corpus first (see LAYOUT.md)",
            path.display(),
            corpus_root.display()
        )),
    }
}

/// Structured one-line-per-document progress, tail-f friendly (F-7.4).
pub fn print_outcomes(outcomes: &[Outcome]) {
    for o in outcomes {
        match o {
            Outcome::Success {
                source_id,
                chunks_indexed,
            } => {
                println!("ok\tsource={}\tchunks={}", source_id.0, chunks_indexed);
            }
            Outcome::Skipped { source_id, reason } => {
                println!("skip\tsource={}\treason={}", source_id.0, reason);
            }
            Outcome::Failed {
                source_id,
                stage,
                error,
            } => {
                println!(
                    "fail\tsource={}\tstage={}\terror={}",
                    source_id.0, stage, error
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_id_is_relative_to_corpus_root() {
        let id = canonical_source_id(
            Path::new("/data/corpus/software/markdown/foo-book/chapter-1.md"),
            Path::new("/data/corpus"),
        )
        .unwrap();
        assert_eq!(id.0, "software/markdown/foo-book/chapter-1.md");
    }

    #[test]
    fn rejects_path_outside_corpus_root() {
        assert!(canonical_source_id(Path::new("/tmp/x.md"), Path::new("/data/corpus")).is_err());
    }
}
