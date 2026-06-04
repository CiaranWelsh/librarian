//! Integration test for `librarian health` (issue 028, Tier 2). Runs the CLI binary against an
//! in-process stub+mem daemon (no Qdrant / OpenAI), so it always runs. Checks the reported
//! metrics and that a history record is appended.

use assert_cmd::Command;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_reports_metrics_and_appends_history() {
    // stub+mem daemon seeded with `demo` = {apple, zebra}.
    let (base, _handle) = query_daemon::test_support::spawn().await;

    let dir = tempfile::tempdir().unwrap();
    let golden = dir.path().join("golden.json");
    std::fs::write(
        &golden,
        r#"[{"q":"apple","relevant":["apple"]},{"q":"zebra","relevant":["zebra"]}]"#,
    )
    .unwrap();
    let history = dir.path().join("health.jsonl");

    let golden_s = golden.to_str().unwrap().to_string();
    let history_s = history.to_str().unwrap().to_string();
    // assert_cmd is blocking; run it off the reactor so the daemon task keeps serving.
    let out = tokio::task::spawn_blocking(move || {
        Command::cargo_bin("librarian")
            .unwrap()
            .args([
                "health",
                "demo",
                "--golden",
                &golden_s,
                "--daemon",
                &base,
                "--k",
                "5",
                "--history",
                &history_s,
            ])
            .output()
            .unwrap()
    })
    .await
    .unwrap();

    assert!(
        out.status.success(),
        "health failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // apple→apple and zebra→zebra both hit at rank 1 → 100% hit-rate.
    assert!(
        stdout.contains("hit-rate@5=100%"),
        "expected 100% hit-rate, got:\n{stdout}"
    );

    let hist = std::fs::read_to_string(&history).unwrap();
    assert!(
        hist.contains("\"collection\":\"demo\"") && hist.contains("\"hit_rate\":1"),
        "history record not appended as expected: {hist}"
    );
}
