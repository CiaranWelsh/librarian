//! `Server` — the JSON-RPC handler. Stateless wrt the protocol. Each tool call
//! is translated into an HTTP request to the query daemon; this adapter holds
//! no query logic of its own.

use serde_json::{json, Value};

use crate::config::load;
use crate::tools::tool_specs;

pub struct Server {
    collection: String,
    daemon_url: String,
    http: reqwest::blocking::Client,
}

impl Server {
    pub fn open(config_path: &std::path::Path) -> Result<Self, String> {
        let cfg = load(config_path)?;
        Ok(Self {
            collection: cfg.collection,
            daemon_url: cfg.daemon_url.trim_end_matches('/').to_string(),
            http: reqwest::blocking::Client::new(),
        })
    }

    pub fn handle(&self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);

        // Notification: no id, no reply (e.g. notifications/initialized). `id` is
        // reused by reference below, so an early guard reads clearer than `?`.
        #[allow(clippy::question_mark)]
        if id.is_none() {
            return None;
        }

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
            Err(msg) => {
                json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32603, "message": msg } })
            }
        })
    }

    fn call_tool(&self, params: &Value) -> Result<Value, String> {
        let name = params
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or("missing tool name")?;
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
        let (path, body) = build_search(&self.collection, args);
        self.post(path, body)
    }

    fn tool_list_documents(&self) -> Result<Value, String> {
        let raw = self.get(&format!("/v1/documents?collection={}", self.collection))?;
        // The daemon returns {documents:[string]}; MCP contract is {documents:[{source_id}]}.
        let ids = raw["documents"]
            .as_array()
            .ok_or_else(|| format!("unexpected documents response: {raw}"))?;
        let docs: Vec<Value> = ids
            .iter()
            .filter_map(|v| v.as_str())
            .map(|id| json!({ "source_id": id }))
            .collect();
        Ok(json!({ "documents": docs }))
    }

    fn tool_get_extract(&self, args: &Value) -> Result<Value, String> {
        let sid = args
            .get("source_id")
            .and_then(|v| v.as_str())
            .ok_or("source_id required")?;
        let mut body = json!({
            "collection": self.collection,
            "source_id": sid,
        });
        if let Some(start) = args.get("start").and_then(|v| v.as_u64()) {
            body["start"] = json!(start);
        }
        if let Some(end) = args.get("end").and_then(|v| v.as_u64()) {
            body["end"] = json!(end);
        }
        self.post("/v1/extract", body)
    }

    fn post(&self, path: &str, body: Value) -> Result<Value, String> {
        let resp = self
            .http
            .post(format!("{}{}", self.daemon_url, path))
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?;
        let status = resp.status();
        let val: Value = resp.json().map_err(|e| e.to_string())?;
        if !status.is_success() {
            let msg = val["error"]["message"].as_str().unwrap_or("daemon error");
            return Err(format!("daemon {status}: {msg}"));
        }
        Ok(val)
    }

    fn get(&self, path: &str) -> Result<Value, String> {
        let resp = self
            .http
            .get(format!("{}{}", self.daemon_url, path))
            .send()
            .map_err(|e| e.to_string())?;
        let status = resp.status();
        let val: Value = resp.json().map_err(|e| e.to_string())?;
        if !status.is_success() {
            let msg = val["error"]["message"].as_str().unwrap_or("daemon error");
            return Err(format!("daemon {status}: {msg}"));
        }
        Ok(val)
    }
}

/// Pure builder for the `/v1/search` request body. Kept separate so the
/// argument translation (MCP `k` → daemon `limit`, default 5) is unit-testable
/// without a running daemon.
fn build_search(collection: &str, args: &Value) -> (&'static str, Value) {
    let mut body = json!({
        "collection": collection,
        "query": args.get("query").and_then(|v| v.as_str()).unwrap_or(""),
        "limit": args.get("k").and_then(|v| v.as_u64()).unwrap_or(5),
    });
    if let Some(ct) = args.get("content_type").and_then(|v| v.as_str()) {
        body["content_type"] = Value::from(ct);
    }
    ("/v1/search", body)
}

#[cfg(test)]
mod adapter_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn search_tool_maps_to_daemon_search_body() {
        let args = json!({"query": "alpha", "k": 7, "content_type": "book"});
        let (path, body) = build_search("physics", &args);
        assert_eq!(path, "/v1/search");
        assert_eq!(body["collection"], "physics");
        assert_eq!(body["query"], "alpha");
        assert_eq!(body["limit"], 7);
        assert_eq!(body["content_type"], "book");
    }
}
