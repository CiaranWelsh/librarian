//! `librarian ingest` — dispatch on (extractor × embedder) at the composition
//! root. Each arm builds a concrete `Pipeline<E, Ch, Em, Ix>` and a
//! `BatchRunner` around it. No `Box<dyn Trait>` (per memory rule).

use adapter_cache_fs::FsCache;
use adapter_chunker_blankline::BlankLineChunker;
use adapter_chunker_code::CodeChunker;
use adapter_embedder_openai::{OpenAiConfig, OpenAiEmbedder};
use adapter_embedder_stub::StubEmbedder;
use adapter_embedder_voyage::{VoyageConfig, VoyageEmbedder};
use adapter_extractor_code::CodeExtractor;
use adapter_extractor_ebook::EbookExtractor;
use adapter_extractor_html::HtmlExtractor;
use adapter_extractor_pdf::PdfExtractor;
use adapter_extractor_text::TextExtractor;
use adapter_indexer_qdrant::QdrantIndexer;
use adapter_manifest_sqlite::SqliteManifest;
use librarian_domain::{Chunker, Document, Embedder, Extractor};
use librarian_runner::{BatchRunner, Outcome, Pipeline};
use std::path::Path;

use crate::config::{Config, EmbedderConfig};
use crate::docs::{collect_docs, print_outcomes};

pub fn cmd_ingest(config_path: &Path, input: &Path) -> Result<(), String> {
    let cfg = Config::load(config_path).map_err(|e| e.to_string())?;
    let docs = collect_docs(input, &cfg.ingest.content_type, &cfg.ingest.extractor)?;
    if docs.is_empty() {
        println!("no input files found at {}", input.display());
        return Ok(());
    }

    let outcomes = match (cfg.ingest.extractor.as_str(), &cfg.embedder) {
        ("text", EmbedderConfig::Stub) => run_ingest(
            &cfg,
            TextExtractor::new(),
            BlankLineChunker::new(),
            StubEmbedder::new(),
            &docs,
        )?,
        ("pdf", EmbedderConfig::Stub) => run_ingest(
            &cfg,
            PdfExtractor::new(),
            BlankLineChunker::new(),
            StubEmbedder::new(),
            &docs,
        )?,
        ("ebook", EmbedderConfig::Stub) => run_ingest(
            &cfg,
            EbookExtractor::new(),
            BlankLineChunker::new(),
            StubEmbedder::new(),
            &docs,
        )?,
        ("html", EmbedderConfig::Stub) => run_ingest(
            &cfg,
            HtmlExtractor::new(),
            BlankLineChunker::new(),
            StubEmbedder::new(),
            &docs,
        )?,
        ("code", EmbedderConfig::Stub) => run_ingest(
            &cfg,
            CodeExtractor::new(),
            CodeChunker::new(),
            StubEmbedder::new(),
            &docs,
        )?,

        (
            "text",
            EmbedderConfig::Openai {
                model,
                dimensions,
                batch_size,
            },
        ) => {
            let emb = openai(model, *dimensions, *batch_size)?;
            run_ingest(
                &cfg,
                TextExtractor::new(),
                BlankLineChunker::new(),
                emb,
                &docs,
            )?
        }
        (
            "pdf",
            EmbedderConfig::Openai {
                model,
                dimensions,
                batch_size,
            },
        ) => {
            let emb = openai(model, *dimensions, *batch_size)?;
            run_ingest(
                &cfg,
                PdfExtractor::new(),
                BlankLineChunker::new(),
                emb,
                &docs,
            )?
        }
        (
            "code",
            EmbedderConfig::Openai {
                model,
                dimensions,
                batch_size,
            },
        ) => {
            let emb = openai(model, *dimensions, *batch_size)?;
            run_ingest(&cfg, CodeExtractor::new(), CodeChunker::new(), emb, &docs)?
        }
        (
            "ebook",
            EmbedderConfig::Openai {
                model,
                dimensions,
                batch_size,
            },
        ) => {
            let emb = openai(model, *dimensions, *batch_size)?;
            run_ingest(
                &cfg,
                EbookExtractor::new(),
                BlankLineChunker::new(),
                emb,
                &docs,
            )?
        }
        (
            "html",
            EmbedderConfig::Openai {
                model,
                dimensions,
                batch_size,
            },
        ) => {
            let emb = openai(model, *dimensions, *batch_size)?;
            run_ingest(
                &cfg,
                HtmlExtractor::new(),
                BlankLineChunker::new(),
                emb,
                &docs,
            )?
        }

        (
            "text",
            EmbedderConfig::Voyage {
                model,
                dimensions,
                batch_size,
            },
        ) => {
            let emb = voyage(model, *dimensions, *batch_size)?;
            run_ingest(
                &cfg,
                TextExtractor::new(),
                BlankLineChunker::new(),
                emb,
                &docs,
            )?
        }
        (
            "pdf",
            EmbedderConfig::Voyage {
                model,
                dimensions,
                batch_size,
            },
        ) => {
            let emb = voyage(model, *dimensions, *batch_size)?;
            run_ingest(
                &cfg,
                PdfExtractor::new(),
                BlankLineChunker::new(),
                emb,
                &docs,
            )?
        }
        (
            "code",
            EmbedderConfig::Voyage {
                model,
                dimensions,
                batch_size,
            },
        ) => {
            let emb = voyage(model, *dimensions, *batch_size)?;
            run_ingest(&cfg, CodeExtractor::new(), CodeChunker::new(), emb, &docs)?
        }
        (
            "ebook",
            EmbedderConfig::Voyage {
                model,
                dimensions,
                batch_size,
            },
        ) => {
            let emb = voyage(model, *dimensions, *batch_size)?;
            run_ingest(
                &cfg,
                EbookExtractor::new(),
                BlankLineChunker::new(),
                emb,
                &docs,
            )?
        }
        (
            "html",
            EmbedderConfig::Voyage {
                model,
                dimensions,
                batch_size,
            },
        ) => {
            let emb = voyage(model, *dimensions, *batch_size)?;
            run_ingest(
                &cfg,
                HtmlExtractor::new(),
                BlankLineChunker::new(),
                emb,
                &docs,
            )?
        }

        (kind, _) => return Err(format!("unsupported extractor: {kind}")),
    };

    print_outcomes(&outcomes);
    if outcomes.iter().any(Outcome::is_failed) {
        return Err(format!(
            "{} document(s) failed",
            outcomes.iter().filter(|o| o.is_failed()).count()
        ));
    }
    Ok(())
}

fn openai(
    model: &str,
    dimensions: usize,
    batch_size: Option<usize>,
) -> Result<OpenAiEmbedder, String> {
    OpenAiEmbedder::from_env(OpenAiConfig {
        model: model.into(),
        dimensions,
        endpoint: None,
        batch_size,
        timeout: None,
    })
    .map_err(|e| e.to_string())
}

fn voyage(
    model: &str,
    dimensions: usize,
    batch_size: Option<usize>,
) -> Result<VoyageEmbedder, String> {
    VoyageEmbedder::from_env(VoyageConfig {
        model: model.into(),
        dimensions,
        endpoint: None,
        batch_size,
        timeout: None,
    })
    .map_err(|e| e.to_string())
}

fn run_ingest<E, Ch, Em>(
    cfg: &Config,
    extractor: E,
    chunker: Ch,
    embedder: Em,
    docs: &[Document],
) -> Result<Vec<Outcome>, String>
where
    E: Extractor,
    Ch: Chunker,
    Em: Embedder,
{
    let cache = FsCache::open(&cfg.paths.cache).map_err(|e| e.to_string())?;
    let manifest = SqliteManifest::open(&cfg.paths.manifest).map_err(|e| e.to_string())?;
    let dim = embedder.dimension() as u64;
    let indexer =
        QdrantIndexer::open(&cfg.qdrant.url, &cfg.collection, dim).map_err(|e| e.to_string())?;
    let quality = cfg.quality.to_domain();
    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor,
            chunker,
            embedder,
            indexer,
        },
        manifest,
        cache,
        quality,
    };
    Ok(runner.ingest_batch(docs))
}
