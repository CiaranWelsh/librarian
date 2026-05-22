//! `librarian status` — without --config: fleet status. With --config:
//! per-collection point count + manifest summary.

use adapter_embedder_stub::StubEmbedder;
use adapter_indexer_qdrant::QdrantIndexer;
use adapter_manifest_sqlite::SqliteManifest;
use librarian_domain::{Embedder, ManifestStatus, ManifestStore};
use std::path::Path;

use crate::config::{Config, EmbedderConfig};
use crate::fleet;

pub fn cmd_status_collection(config_path: &Path) -> Result<(), String> {
    let cfg = Config::load(config_path).map_err(|e| e.to_string())?;
    let dim = match &cfg.embedder {
        EmbedderConfig::Stub => StubEmbedder::new().dimension() as u64,
        EmbedderConfig::Openai { dimensions, .. } => *dimensions as u64,
        EmbedderConfig::Voyage { dimensions, .. } => *dimensions as u64,
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

pub fn cmd_fleet_status() -> Result<(), String> {
    let reg = fleet::Registry::open(&fleet::registry_path())?;
    let rows = fleet::fleet_status(&reg)?;
    if rows.is_empty() {
        println!("(empty fleet)");
        return Ok(());
    }
    for (r, uptime) in rows {
        println!(
            "{}\tport={}\tstatus={}\tuptime={}s\tpid={}",
            r.name, r.port, r.status, uptime,
            r.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".into()),
        );
    }
    Ok(())
}
