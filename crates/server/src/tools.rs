//! MCP tool specifications. Single source of truth so the schema in
//! `tools/list` matches what `tools/call` actually accepts.

use serde_json::{json, Value};

pub fn tool_specs() -> Value {
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
