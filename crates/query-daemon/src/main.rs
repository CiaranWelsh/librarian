//! `librarian-serve` — the query daemon binary.
//! Usage: librarian-serve --config /path/to/daemon.toml

use std::sync::Arc;

use adapter_indexer_qdrant::QdrantSearcher;
use query_core::QueryService;
use query_daemon::config::{AppEmbedder, DaemonConfig};
use query_daemon::{router, AppState};

fn main() {
    if let Err(e) = run() {
        eprintln!("librarian-serve: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut config_path = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--config" => config_path = args.next(),
            other => return Err(format!("unexpected argument: {other}")),
        }
    }
    let config_path = config_path.ok_or("missing --config <path>")?;
    let text = std::fs::read_to_string(&config_path).map_err(|e| format!("read config: {e}"))?;
    let cfg: DaemonConfig = toml::from_str(&text).map_err(|e| format!("parse config: {e}"))?;

    let embedder = AppEmbedder::from_cfg(&cfg.embedder)?;
    let searcher = QdrantSearcher::open(&cfg.qdrant_url).map_err(|e| e.to_string())?;
    let svc = QueryService::new(Arc::new(embedder), searcher, cfg.max_concurrent_embeds);
    let app = router(AppState { svc: Arc::new(svc) });

    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async move {
        let listener = tokio::net::TcpListener::bind(&cfg.bind)
            .await
            .map_err(|e| format!("bind {}: {e}", cfg.bind))?;
        eprintln!("librarian-serve listening on {}", cfg.bind);
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|e| format!("serve: {e}"))
    })
}

/// Resolve when the process receives Ctrl-C, letting axum drain in-flight
/// requests before the runtime tears down.
async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.ok();
}
