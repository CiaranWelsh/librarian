//! Source-discovery helpers: walk the input path and produce `Document` records.

use librarian_domain::{ContentType, Document, SourceHash, SourceId};
use librarian_runner::Outcome;
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn collect_docs(input: &Path, content_type: &str) -> Result<Vec<Document>, String> {
    let ct = match content_type {
        "book" => ContentType::Book,
        "paper" => ContentType::Paper,
        "code" => ContentType::Code,
        other => return Err(format!("unknown content_type: {other}")),
    };
    let mut docs = Vec::new();
    if input.is_file() {
        docs.push(make_doc(input, ct)?);
    } else {
        for entry in walkdir::WalkDir::new(input).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                docs.push(make_doc(entry.path(), ct)?);
            }
        }
    }
    Ok(docs)
}

fn make_doc(path: &Path, ct: ContentType) -> Result<Document, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    let hash = SourceHash(hex::encode(Sha256::digest(&bytes)));
    Ok(Document {
        source_id: SourceId(path.display().to_string()),
        source_hash: hash,
        content_type: ct,
        path: path.to_path_buf(),
        work_id: None,
    })
}

/// Structured one-line-per-document progress, tail-f friendly (F-7.4).
pub fn print_outcomes(outcomes: &[Outcome]) {
    for o in outcomes {
        match o {
            Outcome::Success { source_id, chunks_indexed } => {
                println!("ok\tsource={}\tchunks={}", source_id.0, chunks_indexed);
            }
            Outcome::Failed { source_id, stage, error } => {
                println!("fail\tsource={}\tstage={}\terror={}", source_id.0, stage, error);
            }
        }
    }
}
