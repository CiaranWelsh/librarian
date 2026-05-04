//! `librarian` CLI binary. Composition root for v1 commands.
//!
//! Adapter dispatch uses generics, no `Box<dyn Trait>` (per memory rule).

mod config;

use adapter_cache_fs::FsCache;
use adapter_chunker_blankline::BlankLineChunker;
use adapter_embedder_openai::{OpenAiConfig, OpenAiEmbedder};
use adapter_embedder_stub::StubEmbedder;
use adapter_extractor_pdf::PdfExtractor;
use adapter_extractor_text::TextExtractor;
use adapter_indexer_qdrant::QdrantIndexer;
use adapter_manifest_sqlite::SqliteManifest;
use adapter_snapshotter_qdrant_nas::QdrantNasSnapshotter;
use clap::{Parser, Subcommand};
use config::{Config, EmbedderConfig};
use librarian_domain::{
    ContentType, Document, Embedder, Extractor, ManifestStatus, ManifestStore, SnapshotId,
    SourceHash, SourceId,
};
use librarian_runner::{BatchRunner, Outcome, Pipeline, SnapshotOrchestrator};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "librarian", version, about = "Vector-DB framework CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Ingest a file or directory tree into the configured collection.
    Ingest {
        #[arg(long)] config: PathBuf,
        input: PathBuf,
    },
    /// Remove all chunks for a `source_id`.
    Remove {
        #[arg(long)] config: PathBuf,
        #[arg(long = "source-id")] source_id: String,
    },
    /// Show collection name and current point count.
    Status {
        #[arg(long)] config: PathBuf,
    },
    /// Snapshot the collection (slice 014 stub).
    Snapshot { #[arg(long)] config: PathBuf },
    /// Restore from snapshot id (slice 014 stub).
    Restore { #[arg(long)] config: PathBuf, snapshot_id: String },
    /// Start the per-collection MCP server (slice 015 stub).
    Start   { #[arg(long)] config: PathBuf },
    /// Stop the per-collection MCP server (slice 015 stub).
    Stop    { #[arg(long)] config: PathBuf },
    /// Restart the per-collection MCP server (slice 015 stub).
    Restart { #[arg(long)] config: PathBuf },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.cmd {
        Cmd::Ingest { config, input } => cmd_ingest(&config, &input),
        Cmd::Remove { config, source_id } => cmd_remove(&config, &source_id),
        Cmd::Status { config } => cmd_status(&config),
        Cmd::Snapshot { config } => cmd_snapshot(&config),
        Cmd::Restore { config, snapshot_id } => cmd_restore(&config, &snapshot_id),
        Cmd::Start { .. } | Cmd::Stop { .. } | Cmd::Restart { .. } => {
            eprintln!("not yet implemented (delivered in slice 015)");
            return ExitCode::from(64);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_ingest(config_path: &Path, input: &Path) -> Result<(), String> {
    let cfg = Config::load(config_path).map_err(|e| e.to_string())?;
    let docs = collect_docs(input, &cfg.ingest.content_type)?;
    if docs.is_empty() {
        println!("no input files found at {}", input.display());
        return Ok(());
    }

    // Dispatch on config-selected extractor + embedder. Each arm builds a
    // concrete `Pipeline<E, Ch, Em, Ix>` and a `BatchRunner` around it.
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

fn cmd_remove(config_path: &Path, source_id: &str) -> Result<(), String> {
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

fn cmd_status(config_path: &Path) -> Result<(), String> {
    let cfg = Config::load(config_path).map_err(|e| e.to_string())?;
    let dim = match &cfg.embedder {
        EmbedderConfig::Stub => StubEmbedder::new().dimension() as u64,
        EmbedderConfig::Openai { dimensions, .. } => *dimensions as u64,
    };
    let indexer = QdrantIndexer::open(&cfg.qdrant.url, &cfg.collection, dim).map_err(|e| e.to_string())?;
    let count = indexer.count().map_err(|e| e.to_string())?;
    println!("collection: {}", cfg.collection);
    println!("points: {count}");
    let manifest = SqliteManifest::open(&cfg.paths.manifest).map_err(|e| e.to_string())?;
    let succ = manifest.list_by_status(ManifestStatus::Success).map_err(|e| e.to_string())?.len();
    let cached = manifest.list_by_status(ManifestStatus::Cached).map_err(|e| e.to_string())?.len();
    let failed = manifest.list_by_status(ManifestStatus::Failed).map_err(|e| e.to_string())?.len();
    println!("manifest: success={succ} cached={cached} failed={failed}");
    Ok(())
}

fn collect_docs(input: &Path, content_type: &str) -> Result<Vec<Document>, String> {
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

fn snapshot_orchestrator(cfg: &Config) -> Result<SnapshotOrchestrator<QdrantNasSnapshotter, SqliteManifest>, String> {
    let nas = cfg.paths.snapshots.clone().ok_or_else(|| "config: paths.snapshots required for snapshot/restore".to_string())?;
    let snapshotter = QdrantNasSnapshotter::new(&cfg.qdrant.url, &cfg.collection, nas)
        .map_err(|e| e.to_string())?;
    let manifest = SqliteManifest::open(&cfg.paths.manifest).map_err(|e| e.to_string())?;
    Ok(SnapshotOrchestrator { snapshotter, manifest, retention: cfg.snapshot.retention })
}

fn cmd_snapshot(config_path: &Path) -> Result<(), String> {
    let cfg = Config::load(config_path).map_err(|e| e.to_string())?;
    let orch = snapshot_orchestrator(&cfg)?;
    let id = orch.snapshot()?;
    println!("snapshot\tid={}", id.0);
    Ok(())
}

fn cmd_restore(config_path: &Path, snapshot_id: &str) -> Result<(), String> {
    let cfg = Config::load(config_path).map_err(|e| e.to_string())?;
    let orch = snapshot_orchestrator(&cfg)?;
    orch.restore(&SnapshotId(snapshot_id.into()))?;
    println!("restore\tid={snapshot_id}");
    Ok(())
}

fn print_outcomes(outcomes: &[Outcome]) {
    // Structured one-line-per-document progress, tail-f friendly (F-7.4).
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
