//! B7 — the real `librarian-collection` binary against a real (stub+mem)
//! daemon. Proves the demoted server is pure translation: a JSON-RPC
//! `tools/call` for `search` becomes an HTTP call to the daemon and the hit
//! comes back inside the preserved MCP content envelope.

use std::io::{Read, Write};

#[tokio::test(flavor = "multi_thread")]
async fn mcp_search_forwards_daemon_hit() {
    let (base, _handle) = query_daemon::test_support::spawn().await;

    let out = tokio::task::spawn_blocking(move || {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("server.toml");
        std::fs::write(
            &cfg,
            format!("collection = \"demo\"\ndaemon_url = \"{base}\"\n"),
        )
        .unwrap();

        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_librarian-collection"))
            .args(["--config", cfg.to_str().unwrap()])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let req = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "search", "arguments": {"query": "apple", "k": 5}}
        });

        // The server reads stdin until EOF; close it so the child can exit and
        // we don't deadlock waiting on its output (LF-2).
        let mut stdin = child.stdin.take().unwrap();
        writeln!(stdin, "{req}").unwrap();
        drop(stdin);

        let mut stdout = String::new();
        child
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut stdout)
            .unwrap();
        let _ = child.wait();
        stdout
    })
    .await
    .unwrap();

    let line = out.lines().next().expect("a JSON-RPC reply");
    let reply: serde_json::Value = serde_json::from_str(line).expect("reply parses");

    // The content envelope is preserved: result.content[0].text is a JSON
    // string holding the daemon's response. Parse it and confirm the apple hit.
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .expect("content text");
    assert!(
        text.contains("\"source_id\":\"apple\""),
        "expected the daemon's apple hit inside the content envelope, got: {text}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn mcp_list_documents_forwards_reshaped_documents() {
    let (base, _handle) = query_daemon::test_support::spawn().await;

    let out = tokio::task::spawn_blocking(move || {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("server.toml");
        std::fs::write(
            &cfg,
            format!("collection = \"demo\"\ndaemon_url = \"{base}\"\n"),
        )
        .unwrap();

        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_librarian-collection"))
            .args(["--config", cfg.to_str().unwrap()])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let req = serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": {"name": "list_documents", "arguments": {}}
        });

        let mut stdin = child.stdin.take().unwrap();
        writeln!(stdin, "{req}").unwrap();
        drop(stdin);

        let mut stdout = String::new();
        child
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut stdout)
            .unwrap();
        let _ = child.wait();
        stdout
    })
    .await
    .unwrap();

    let line = out.lines().next().expect("a JSON-RPC reply");
    let reply: serde_json::Value = serde_json::from_str(line).expect("reply parses");

    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .expect("content text");
    let body: serde_json::Value = serde_json::from_str(text).expect("text is JSON");

    let docs = body["documents"].as_array().expect("documents is an array");
    assert!(
        docs.iter().all(|d| d.get("source_id").is_some()),
        "each document must have a source_id field, got: {docs:?}"
    );

    let ids: Vec<&str> = docs
        .iter()
        .filter_map(|d| d["source_id"].as_str())
        .collect();
    assert!(
        ids.contains(&"apple"),
        "expected apple in documents, got: {ids:?}"
    );
    assert!(
        ids.contains(&"zebra"),
        "expected zebra in documents, got: {ids:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn mcp_search_unknown_collection_returns_jsonrpc_error() {
    // The daemon only seeds `demo`. Pointing the thin server at a `missing`
    // collection drives the demoted error path: unknown collection -> daemon
    // 404 -> thin-server `Err` -> JSON-RPC `error`, never a "successful" reply.
    let (base, _handle) = query_daemon::test_support::spawn().await;

    let out = tokio::task::spawn_blocking(move || {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("server.toml");
        std::fs::write(
            &cfg,
            format!("collection = \"missing\"\ndaemon_url = \"{base}\"\n"),
        )
        .unwrap();

        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_librarian-collection"))
            .args(["--config", cfg.to_str().unwrap()])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let req = serde_json::json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {"name": "search", "arguments": {"query": "apple", "k": 5}}
        });

        let mut stdin = child.stdin.take().unwrap();
        writeln!(stdin, "{req}").unwrap();
        drop(stdin);

        let mut stdout = String::new();
        child
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut stdout)
            .unwrap();
        let _ = child.wait();
        stdout
    })
    .await
    .unwrap();

    let line = out.lines().next().expect("a JSON-RPC reply");
    let reply: serde_json::Value = serde_json::from_str(line).expect("reply parses");

    assert!(
        reply["error"].is_object(),
        "expected a JSON-RPC error object, got: {reply}"
    );
    assert!(
        reply.get("result").is_none(),
        "a daemon error must not surface as a successful result, got: {reply}"
    );
}
