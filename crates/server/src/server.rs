//! `Server` — the JSON-RPC handler. Stateless wrt the protocol; keeps a
//! Qdrant indexer + manifest store open for the lifetime of the process.

use adapter_indexer_qdrant::QdrantIndexer;
use adapter_manifest_sqlite::{distinct_ingested_sources, SqliteManifest};
use librarian_domain::SourceId;
use serde_json::{json, Value};

use crate::config::{embed_query, embedder_dim, Config};
use crate::tools::tool_specs;

pub struct Server {
    cfg: Config,
    indexer: QdrantIndexer,
    manifest: SqliteManifest,
}

impl Server {
    pub fn open(config_path: &std::path::Path) -> Result<Self, String> {
        let s = std::fs::read_to_string(config_path).map_err(|e| format!("config io: {e}"))?;
        let cfg: Config = toml::from_str(&s).map_err(|e| format!("config parse: {e}"))?;
        let dim = embedder_dim(&cfg.embedder);
        let indexer = QdrantIndexer::open(&cfg.qdrant.url, &cfg.collection, dim).map_err(|e| e.to_string())?;
        let manifest = SqliteManifest::open(&cfg.paths.manifest).map_err(|e| e.to_string())?;
        Ok(Self { cfg, indexer, manifest })
    }

    pub fn handle(&self, msg: &Value) -> Option<Value> {
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
