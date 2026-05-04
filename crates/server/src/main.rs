//! `librarian-collection` — read-only MCP server. Hand-rolled JSON-RPC 2.0
//! over line-delimited stdio. Three tools: search, list_documents, get_extract.
//!
//! No auth in v1 — local-network trust on Turbo per ADR-0004 / deployment view.

use adapter_embedder_openai::{OpenAiConfig, OpenAiEmbedder};
use adapter_embedder_stub::StubEmbedder;
use adapter_indexer_qdrant::QdrantIndexer;
use adapter_manifest_sqlite::{distinct_ingested_sources, SqliteManifest};
use clap::Parser;
use librarian_domain::{Embedder, SourceId};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "librarian-collection", about = "Per-collection MCP server")]
struct Cli {
    #[arg(long)] config: PathBuf,
    /// Allocated by the supervisor (slice 015). Currently unused — server runs over stdio.
    #[arg(long)] port: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct Config {
    collection: String,
    qdrant: QdrantCfg,
    paths: Paths,
    embedder: EmbedderCfg,
}

#[derive(Debug, Deserialize)]
struct QdrantCfg { url: String }

#[derive(Debug, Deserialize)]
struct Paths { manifest: PathBuf }

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum EmbedderCfg {
    Stub,
    Openai { model: String, dimensions: usize, #[serde(default)] batch_size: Option<usize> },
}

/// Type-erased dimension lookup used by composition root before constructing
/// the concrete embedder.
fn embedder_dim(cfg: &EmbedderCfg) -> u64 {
    match cfg {
        EmbedderCfg::Stub => StubEmbedder::new().dimension() as u64,
        EmbedderCfg::Openai { dimensions, .. } => *dimensions as u64,
    }
}

fn embed_query(cfg: &EmbedderCfg, q: &str) -> Result<Vec<f32>, String> {
    match cfg {
        EmbedderCfg::Stub => StubEmbedder::new().embed(&[q]).map(|v| v.into_iter().next().unwrap()).map_err(|e| e.to_string()),
        EmbedderCfg::Openai { model, dimensions, batch_size } => {
            let e = OpenAiEmbedder::from_env(OpenAiConfig {
                model: model.clone(), dimensions: *dimensions,
                endpoint: None, batch_size: *batch_size, timeout: None,
            }).map_err(|e| e.to_string())?;
            e.embed(&[q]).map(|v| v.into_iter().next().unwrap()).map_err(|e| e.to_string())
        }
    }
}

struct Server {
    cfg: Config,
    indexer: QdrantIndexer,
    manifest: SqliteManifest,
}

impl Server {
    fn open(config_path: &std::path::Path) -> Result<Self, String> {
        let s = std::fs::read_to_string(config_path).map_err(|e| format!("config io: {e}"))?;
        let cfg: Config = toml::from_str(&s).map_err(|e| format!("config parse: {e}"))?;
        let dim = embedder_dim(&cfg.embedder);
        let indexer = QdrantIndexer::open(&cfg.qdrant.url, &cfg.collection, dim).map_err(|e| e.to_string())?;
        let manifest = SqliteManifest::open(&cfg.paths.manifest).map_err(|e| e.to_string())?;
        Ok(Self { cfg, indexer, manifest })
    }

    fn handle(&self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);

        // Notifications carry no id and need no reply (e.g. notifications/initialized).
        if id.is_none() { return None; }

        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "librarian-collection", "version": env!("CARGO_PKG_VERSION") }
            })),
            "tools/list" => Ok(json!({ "tools": tool_specs() })),
            "tools/call" => self.call_tool(&params),
            other => Err(format!("method not found: {other}")),
        };

        Some(match result {
            Ok(v) => json!({ "jsonrpc": "2.0", "id": id, "result": v }),
            Err(msg) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32603, "message": msg } }),
        })
    }

    fn call_tool(&self, params: &Value) -> Result<Value, String> {
        let name = params.get("name").and_then(|n| n.as_str()).ok_or("missing tool name")?;
        let args = params.get("arguments").cloned().unwrap_or(Value::Null);
        let payload = match name {
            "search" => self.tool_search(&args)?,
            "list_documents" => self.tool_list_documents()?,
            "get_extract" => self.tool_get_extract(&args)?,
            other => return Err(format!("unknown tool: {other}")),
        };
        Ok(json!({ "content": [{ "type": "text", "text": payload.to_string() }] }))
    }

    fn tool_search(&self, args: &Value) -> Result<Value, String> {
        let query = args.get("query").and_then(|v| v.as_str()).ok_or("query required")?;
        let k = args.get("k").and_then(|v| v.as_u64()).unwrap_or(5);
        let filter = args.get("content_type").and_then(|v| v.as_str());
        let qv = embed_query(&self.cfg.embedder, query)?;
        let hits = self.indexer.search(&qv, k, filter).map_err(|e| e.to_string())?;
        Ok(json!({
            "hits": hits.iter().map(|h| json!({
                "score": h.score, "source_id": h.source_id, "chunk_index": h.chunk_index,
                "content_type": h.content_type, "text": h.text,
            })).collect::<Vec<_>>()
        }))
    }

    fn tool_list_documents(&self) -> Result<Value, String> {
        let ids = distinct_ingested_sources(&self.manifest).map_err(|e| e.to_string())?;
        Ok(json!({
            "documents": ids.iter().map(|id| json!({ "source_id": id.0 })).collect::<Vec<_>>()
        }))
    }

    fn tool_get_extract(&self, args: &Value) -> Result<Value, String> {
        let sid = args.get("source_id").and_then(|v| v.as_str()).ok_or("source_id required")?;
        let start = args.get("start").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let end = args.get("end").and_then(|v| v.as_u64()).unwrap_or(u32::MAX as u64) as u32;
        let chunks = self.indexer.get_extract(&SourceId(sid.into()), start, end).map_err(|e| e.to_string())?;
        Ok(json!({
            "source_id": sid,
            "chunks": chunks.iter().map(|(i, t)| json!({ "chunk_index": i, "text": t })).collect::<Vec<_>>(),
        }))
    }
}

fn tool_specs() -> Value {
    json!([
        { "name": "search",
          "description": "Semantic search over the collection. Returns top-k chunks.",
          "inputSchema": {
              "type": "object",
              "properties": {
                  "query": { "type": "string" },
                  "k": { "type": "integer", "default": 5 },
                  "content_type": { "type": "string", "enum": ["book","paper","code"] }
              },
              "required": ["query"]
          }},
        { "name": "list_documents",
          "description": "Every Document in the collection (from manifest).",
          "inputSchema": { "type": "object", "properties": {} } },
        { "name": "get_extract",
          "description": "Scoped retrieval: chunks of source_id with chunk_index in [start, end).",
          "inputSchema": {
              "type": "object",
              "properties": {
                  "source_id": { "type": "string" },
                  "start": { "type": "integer", "default": 0 },
                  "end": { "type": "integer" }
              },
              "required": ["source_id"]
          }}
    ])
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let server = match Server::open(&cli.config) {
        Ok(s) => s,
        Err(e) => { eprintln!("error: {e}"); return ExitCode::FAILURE; }
    };

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        if line.trim().is_empty() { continue; }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let err = json!({ "jsonrpc":"2.0","id":null,"error":{"code":-32700,"message": format!("parse: {e}")}});
                let _ = writeln!(out, "{err}");
                continue;
            }
        };
        if let Some(reply) = server.handle(&msg) {
            let _ = writeln!(out, "{reply}");
            let _ = out.flush();
        }
    }
    ExitCode::SUCCESS
}
