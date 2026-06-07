//! Integration test for `librarian judge` (issue 028, Tier 1): runs the CLI against the
//! in-process stub+mem daemon for retrieval and a mockito-mocked OpenAI chat for judging — no
//! real OpenAI / Qdrant, so it always runs.

use assert_cmd::Command;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn judge_scores_chunks_via_mock_llm() {
    let (daemon_base, _handle) = query_daemon::test_support::spawn().await;

    // Mock OpenAI chat: always return the score "2".
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"choices":[{"message":{"content":"2"}}]}"#)
        .create_async()
        .await;
    let openai_base = server.url();

    let out = tokio::task::spawn_blocking(move || {
        Command::cargo_bin("librarian")
            .unwrap()
            .env("OPENAI_API_KEY", "test-key")
            .env("OPENAI_BASE_URL", openai_base)
            .args([
                "judge",
                "demo",
                "apple",
                "--k",
                "2",
                "--daemon",
                &daemon_base,
            ])
            .output()
            .unwrap()
    })
    .await
    .unwrap();

    assert!(
        out.status.success(),
        "judge failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // demo has 2 docs; the mock judge returns 2 for each → mean 2.00, all directly relevant.
    assert!(
        stdout.contains("mean=2.00"),
        "expected mean 2.00, got:\n{stdout}"
    );
    assert!(
        stdout.contains("directly-relevant=2/2"),
        "expected 2/2 relevant, got:\n{stdout}"
    );
}
