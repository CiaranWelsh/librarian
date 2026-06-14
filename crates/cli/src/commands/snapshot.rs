use adapter_manifest_sqlite::SqliteManifest;
use adapter_snapshotter_qdrant_nas::QdrantNasSnapshotter;
use librarian_domain::SnapshotId;
use librarian_runner::SnapshotOrchestrator;
use std::path::Path;

use crate::config::Config;

fn snapshot_orchestrator(
    cfg: &Config,
) -> Result<SnapshotOrchestrator<QdrantNasSnapshotter, SqliteManifest>, String> {
    let nas = cfg
        .paths
        .snapshots
        .clone()
        .ok_or_else(|| "config: paths.snapshots required for snapshot/restore".to_string())?;
    let snapshotter = QdrantNasSnapshotter::new(&cfg.qdrant.rest_url(), &cfg.collection, nas)
        .map_err(|e| e.to_string())?;
    let manifest = SqliteManifest::open(&cfg.paths.manifest).map_err(|e| e.to_string())?;
    Ok(SnapshotOrchestrator {
        snapshotter,
        manifest,
        retention: cfg.snapshot.retention,
    })
}

pub fn cmd_snapshot(config_path: &Path) -> Result<(), String> {
    let cfg = Config::load(config_path).map_err(|e| e.to_string())?;
    let orch = snapshot_orchestrator(&cfg)?;
    let id = orch.snapshot()?;
    println!("snapshot\tid={}", id.0);
    Ok(())
}

pub fn cmd_restore(config_path: &Path, snapshot_id: &str) -> Result<(), String> {
    let cfg = Config::load(config_path).map_err(|e| e.to_string())?;
    let orch = snapshot_orchestrator(&cfg)?;
    orch.restore(&SnapshotId(snapshot_id.into()))?;
    println!("restore\tid={snapshot_id}");
    Ok(())
}
