//! Protocol-level smoke tests for the MCP server, driven over stdio. These
//! cover the stateless JSON-RPC dispatch (`initialize`, `tools/list`, unknown
//! method) which needs no daemon — the server replies before any HTTP call.
//! Tool calls that reach the daemon are covered in `mcp_integration.rs` (B7).

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// Write a minimal thin-client config. `daemon_url` is never contacted by the
/// methods exercised here.
fn write_config(dir: &Path) -> std::path::PathBuf {
    let cfg = dir.join("server.toml");
    std::fs::write(
        &cfg,
        "collection = \"demo\"\ndaemon_url = \"http://127.0.0.1:1\"\n",
    )
    .unwrap();
    cfg
}

/// Send a single JSON-RPC request, close stdin to signal EOF, and return the
/// parsed reply. Closing stdin is what lets the server exit (it reads lines
/// until EOF).
fn roundtrip(cfg: &Path, req: Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_librarian-collection"))
        .args(["--config", cfg.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn server");

    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{req}").unwrap();
    drop(stdin);

    let mut out = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    let _ = child.wait();

    let line = out.lines().next().expect("a reply line");
    serde_json::from_str(line).unwrap()
}

#[test]
fn initialize_returns_protocol_version_and_server_info() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path());
    let r = roundtrip(
        &cfg,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
    );
    let result = r.get("result").expect("result");
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["serverInfo"]["name"], "librarian-collection");
}

#[test]
fn tools_list_advertises_search_list_documents_and_get_extract() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path());
    let r = roundtrip(
        &cfg,
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}),
    );
    let names: Vec<&str> = r["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"search"));
    assert!(names.contains(&"list_documents"));
    assert!(names.contains(&"get_extract"));

    for tool in r["result"]["tools"].as_array().unwrap() {
        assert!(
            tool["inputSchema"].is_object(),
            "tool {} has inputSchema",
            tool["name"]
        );
    }
}

#[test]
fn unknown_method_returns_jsonrpc_error() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path());
    let r = roundtrip(
        &cfg,
        json!({"jsonrpc":"2.0","id":1,"method":"no/such/method","params":{}}),
    );
    assert!(
        r.get("error").is_some(),
        "error returned for unknown method: {r}"
    );
}
