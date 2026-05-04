//! Walking skeleton — runs one fixture through the in-memory pipeline end-to-end.

use adapter_chunker_blankline::BlankLineChunker;
use adapter_embedder_stub::StubEmbedder;
use adapter_extractor_text::TextExtractor;
use adapter_indexer_mem::MemIndexer;
use librarian_domain::{ContentType, Document, SourceHash, SourceId};
use librarian_runner::Pipeline;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path: PathBuf = std::env::args()
        .nth(1)
        .ok_or("usage: walking-skeleton <fixture-path>")?
        .into();

    let bytes = std::fs::read(&path)?;
    let source_hash = SourceHash(hex::encode(Sha256::digest(&bytes)));
    let source_id = SourceId(path.display().to_string());

    let doc = Document {
        source_id: source_id.clone(),
        source_hash,
        content_type: ContentType::Book,
        path,
        work_id: None,
    };

    let pipeline = Pipeline {
        extractor: TextExtractor::new(),
        chunker: BlankLineChunker::new(),
        embedder: StubEmbedder::new(),
        indexer: MemIndexer::new(),
    };

    let summary = pipeline.run(&doc).map_err(|e| e.to_string())?;
    println!(
        "indexed {} chunks for source_id={}",
        summary.chunks_indexed, source_id.0
    );
    Ok(())
}
