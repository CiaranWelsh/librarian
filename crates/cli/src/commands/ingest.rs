//! `librarian ingest` — dispatch on (extractor × embedder) at the composition
//! root. Each arm builds a concrete `Pipeline<E, Ch, Em, Ix>` and a
//! `BatchRunner` around it. No `Box<dyn Trait>` (per memory rule).

use adapter_cache_fs::FsCache;
use adapter_chunker_blankline::BlankLineChunker;
use adapter_embedder_openai::{OpenAiConfig, OpenAiEmbedder};
use adapter_embedder_stub::StubEmbedder;
use adapter_extractor_pdf::PdfExtractor;
use adapter_extractor_text::TextExtractor;
use adapter_indexer_qdrant::QdrantIndexer;
use adapter_manifest_sqlite::SqliteManifest;
use librarian_domain::{Document, Embedder, Extractor};
use librarian_runner::{BatchRunner, Outcome, Pipeline};
use std::path::Path;

use crate::config::{Config, EmbedderConfig};
use crate::docs::{collect_docs, print_outcomes};

pub fn cmd_ingest(config_path: &Path, input: &Path) -> Result<(), String> {
    let cfg = Config::load(config_path).map_err(|e| e.to_string())?;
    let docs = collect_docs(input, &cfg.ingest.content_type)?;
    if docs.is_empty() {
        println!("no input files found at {}", input.display());
        return Ok(());
    }

    let outcomes = match (cfg.ingest.extractor.as_str(), &cfg.embedder) {
        ("text", EmbedderConfig::Stub) => run_ingest(&cfg, TextExtractor::new(), StubEmbedder::new(), &docs)?,
        ("pdf",  EmbedderConfig::Stub) => run_ingest(&cfg, PdfExtractor::new(),  StubEmbedder::new(), &docs)?,
        ("text", EmbedderConfig::Openai { model, dimensions, batch_size }) => {
            let emb = OpenAiEmbedder::from_env(OpenAiConfig {
                model: model.clone(), dimensions: *dimensions,
                endpoint: None, batch_size: *batch_size, timeout: None,
            }).map_err(|e| e.to_string())?;
            run_ingest(&cfg, TextExtractor::new(), emb, &docs)?
        }
        ("pdf", EmbedderConfig::Openai { model, dimensions, batch_size }) => {
            let emb = OpenAiEmbedder::from_env(OpenAiConfig {
                model: model.clone(), dimensions: *dimensions,
                endpoint: None, batch_size: *batch_size, timeout: None,
            }).map_err(|e| e.to_string())?;
            run_ingest(&cfg, PdfExtractor::new(), emb, &docs)?
        }
        (kind, _) => return Err(format!("unsupported extractor: {kind}")),
    };

    print_outcomes(&outcomes);
    if outcomes.iter().any(|o| !o.is_success()) {
        return Err(format!("{} document(s) failed", outcomes.iter().filter(|o| !o.is_success()).count()));
    }
    Ok(())
}

fn run_ingest<E, Em>(
    cfg: &Config, extractor: E, embedder: Em, docs: &[Document],
) -> Result<Vec<Outcome>, String>
where
    E: Extractor,
    Em: Embedder,
{
    let cache = FsCache::open(&cfg.paths.cache).map_err(|e| e.to_string())?;
    let manifest = SqliteManifest::open(&cfg.paths.manifest).map_err(|e| e.to_string())?;
    let dim = embedder.dimension() as u64;
    let indexer = QdrantIndexer::open(&cfg.qdrant.url, &cfg.collection, dim).map_err(|e| e.to_string())?;
    let runner = BatchRunner {
        pipeline: Pipeline { extractor, chunker: BlankLineChunker::new(), embedder, indexer },
        manifest, cache,
    };
    Ok(runner.ingest_batch(docs))
}
