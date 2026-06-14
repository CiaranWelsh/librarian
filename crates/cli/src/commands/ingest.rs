//! `librarian ingest` — dispatch on (extractor × embedder) at the composition
//! root. Each arm builds a concrete `Pipeline<E, Ch, Em, Ix>` and a
//! `BatchRunner` around it. No `Box<dyn Trait>` (per memory rule).

use adapter_cache_fs::FsCache;
use adapter_chunker_blankline::{BlankLineChunkError, BlankLineChunker};
use adapter_chunker_code::CodeChunker;
use adapter_chunker_recursive::{RecursiveChunkError, RecursiveChunker};
use adapter_embedder_openai::{OpenAiConfig, OpenAiEmbedder};
use adapter_embedder_stub::StubEmbedder;
use adapter_embedder_voyage::{VoyageConfig, VoyageEmbedder};
use adapter_extractor_code::CodeExtractor;
use adapter_extractor_ebook::EbookExtractor;
use adapter_extractor_html::HtmlExtractor;
use adapter_extractor_pdf::{MarkerConfig, PdfExtractor};
use adapter_extractor_text::TextExtractor;
use adapter_indexer_qdrant::QdrantIndexer;
use adapter_manifest_sqlite::SqliteManifest;
use librarian_domain::{
    AdapterIdentity, Chunk, Chunker, ConfigHash, Document, Embedder, ExtractedText, Extractor,
    StageVersion,
};
use librarian_runner::{BatchRunner, Outcome, Pipeline};
use std::path::Path;

use crate::config::{Config, EmbedderConfig, IngestConfig};
use crate::docs::{collect_docs, print_outcomes};

/// Build the PDF extractor with the `[ingest.marker]` knobs (issue 030) so constrained
/// GPUs get their batch flags and the markdown can land somewhere durable.
pub(crate) fn pdf_extractor(cfg: &Config) -> PdfExtractor {
    let m = &cfg.ingest.marker;
    PdfExtractor::new().with_config(MarkerConfig {
        device: m.device.clone(),
        recognition_batch_size: m.recognition_batch_size,
        detection_batch_size: m.detection_batch_size,
        layout_batch_size: m.layout_batch_size,
        table_rec_batch_size: m.table_rec_batch_size,
        output_dir: m.output_dir.clone(),
    })
}

pub fn cmd_ingest(config_path: &Path, input: &Path) -> Result<(), String> {
    let cfg = Config::load(config_path).map_err(|e| e.to_string())?;
    let docs = collect_docs(
        input,
        &cfg.ingest.content_type,
        &cfg.ingest.extractor,
        &cfg.ingest.corpus_root,
    )?;
    if docs.is_empty() {
        println!("no input files found at {}", input.display());
        return Ok(());
    }

    // Chunker for text content (code always uses CodeChunker). Built once; the match arms
    // below move it into the chosen pipeline.
    let chunker = select_chunker(&cfg.ingest)?;

    let outcomes = match (cfg.ingest.extractor.as_str(), &cfg.embedder) {
        ("text", EmbedderConfig::Stub) => run_ingest(
            &cfg,
            TextExtractor::new(),
            chunker,
            StubEmbedder::new(),
            &docs,
        )?,
        ("pdf", EmbedderConfig::Stub) => run_ingest(
            &cfg,
            pdf_extractor(&cfg),
            chunker,
            StubEmbedder::new(),
            &docs,
        )?,
        ("ebook", EmbedderConfig::Stub) => run_ingest(
            &cfg,
            EbookExtractor::new(),
            chunker,
            StubEmbedder::new(),
            &docs,
        )?,
        ("html", EmbedderConfig::Stub) => run_ingest(
            &cfg,
            HtmlExtractor::new(),
            chunker,
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
            run_ingest(&cfg, TextExtractor::new(), chunker, emb, &docs)?
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
            run_ingest(&cfg, pdf_extractor(&cfg), chunker, emb, &docs)?
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
            run_ingest(&cfg, EbookExtractor::new(), chunker, emb, &docs)?
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
            run_ingest(&cfg, HtmlExtractor::new(), chunker, emb, &docs)?
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
            run_ingest(&cfg, TextExtractor::new(), chunker, emb, &docs)?
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
            run_ingest(&cfg, pdf_extractor(&cfg), chunker, emb, &docs)?
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
            run_ingest(&cfg, EbookExtractor::new(), chunker, emb, &docs)?
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
            run_ingest(&cfg, HtmlExtractor::new(), chunker, emb, &docs)?
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

/// Runtime chunker selection via enum dispatch (no `Box<dyn>`, per project rule). Code
/// content always uses `CodeChunker`; text content chooses between the recursive (issue 027)
/// and legacy blank-line chunkers via `[ingest] chunker`.
pub(crate) enum SelectedChunker {
    BlankLine(BlankLineChunker),
    Recursive(RecursiveChunker),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SelectedChunkError {
    #[error(transparent)]
    BlankLine(#[from] BlankLineChunkError),
    #[error(transparent)]
    Recursive(#[from] RecursiveChunkError),
}

impl AdapterIdentity for SelectedChunker {
    fn name(&self) -> &str {
        match self {
            SelectedChunker::BlankLine(c) => c.name(),
            SelectedChunker::Recursive(c) => c.name(),
        }
    }
    fn version(&self) -> StageVersion {
        match self {
            SelectedChunker::BlankLine(c) => c.version(),
            SelectedChunker::Recursive(c) => c.version(),
        }
    }
    fn config_hash(&self) -> ConfigHash {
        match self {
            SelectedChunker::BlankLine(c) => c.config_hash(),
            SelectedChunker::Recursive(c) => c.config_hash(),
        }
    }
}

impl Chunker for SelectedChunker {
    type Error = SelectedChunkError;
    fn chunk(&self, doc: &Document, text: ExtractedText) -> Result<Vec<Chunk>, Self::Error> {
        match self {
            SelectedChunker::BlankLine(c) => Ok(c.chunk(doc, text)?),
            SelectedChunker::Recursive(c) => Ok(c.chunk(doc, text)?),
        }
    }
}

pub(crate) fn select_chunker(cfg: &IngestConfig) -> Result<SelectedChunker, String> {
    match cfg.chunker.as_str() {
        "blankline" => Ok(SelectedChunker::BlankLine(BlankLineChunker::new())),
        "recursive" => Ok(SelectedChunker::Recursive(RecursiveChunker::with_budget(
            cfg.chunk_size,
            cfg.chunk_overlap,
        ))),
        other => Err(format!(
            "unknown chunker '{other}' (expected 'recursive' or 'blankline')"
        )),
    }
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
