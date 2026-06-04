//! e2e: librarian-serve over live qdrant + real embedder. Ignored.
//! Run: OPENAI_API_KEY=… LIBRARIAN_QDRANT_URL=http://localhost:6334 \
//!   cargo test -p query-daemon --test e2e -- --ignored
//!
//! Nothing is doubled here (B8): a real `librarian-serve` process talks to a
//! live qdrant and the real OpenAI embedder, queried over the same HTTP path
//! the CLI and MCP server take.

use std::time::Duration;

/// Poll `<base>/healthz` until the daemon answers 200, backing off ~50ms each
/// time up to `deadline`. Returns `true` once ready. This replaces a fixed
/// startup sleep so the test doesn't race the daemon's bind (DET-1).
fn wait_until_ready(base: &str, deadline: Duration) -> bool {
    let client = reqwest::blocking::Client::new();
    let url = format!("{base}/healthz");
    let start = std::time::Instant::now();
    while start.elapsed() < deadline {
        if let Ok(resp) = client.get(&url).send() {
            if resp.status().is_success() {
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
#[ignore = "needs live qdrant + OPENAI_API_KEY"]
fn serve_and_search_real_corpus() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("daemon.toml");
    let qurl =
        std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6334".into());
    std::fs::write(
        &cfg,
        format!(
            "bind = \"127.0.0.1:6789\"\nqdrant_url = \"{qurl}\"\nmax_concurrent_embeds = 4\n\
             [embedder]\nkind = \"openai\"\nmodel = \"text-embedding-3-large\"\ndimensions = 3072\n"
        ),
    )
    .unwrap();

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_librarian-serve"))
        .args(["--config", cfg.to_str().unwrap()])
        .spawn()
        .unwrap();

    let base = "http://127.0.0.1:6789";
    let ready = wait_until_ready(base, Duration::from_secs(5));

    // Only fire the search once the daemon is up; otherwise leave `result` empty
    // and let the readiness assertion below report the real failure.
    let result = ready.then(|| {
        let body = serde_json::json!({
            "collection": "particle-physics",
            "query": "time of arrival calibration",
            "limit": 3
        });
        reqwest::blocking::Client::new()
            .post(format!("{base}/v1/search"))
            .json(&body)
            .send()
            .and_then(|r| r.json::<serde_json::Value>())
    });

    child.kill().ok();
    child.wait().ok();

    assert!(ready, "daemon did not answer /healthz within the deadline");
    let v = result
        .expect("daemon was ready but no request was made")
        .expect("daemon search request failed");
    assert!(
        v["hits"].as_array().map(|a| !a.is_empty()).unwrap_or(false),
        "expected hits, got {v}"
    );
}
