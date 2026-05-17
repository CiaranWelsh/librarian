use adapter_cache_fs::FsCache;
use adapter_chunker_blankline::BlankLineChunker;
use adapter_embedder_stub::StubEmbedder;
use adapter_extractor_text::TextExtractor;
use adapter_indexer_qdrant::QdrantIndexer;
use adapter_manifest_sqlite::SqliteManifest;
use librarian_domain::{Embedder, SourceId};
use librarian_runner::{BatchRunner, Pipeline};
use std::path::Path;

use crate::config::Config;

pub fn cmd_remove(config_path: &Path, source_id: &str) -> Result<(), String> {
    let cfg = Config::load(config_path).map_err(|e| e.to_string())?;
    let manifest = SqliteManifest::open(&cfg.paths.manifest).map_err(|e| e.to_string())?;
    let cache = FsCache::open(&cfg.paths.cache).map_err(|e| e.to_string())?;
    // Build a noop pipeline — only the indexer matters for `remove`.
    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: TextExtractor::new(),
            chunker: BlankLineChunker::new(),
            embedder: StubEmbedder::new(),
            indexer: QdrantIndexer::open(&cfg.qdrant.url, &cfg.collection, StubEmbedder::new().dimension() as u64)
                .map_err(|e| e.to_string())?,
        },
        manifest, cache,
    };
    runner.remove(&SourceId(source_id.into()))?;
    println!("removed {source_id}");
    Ok(())
}
