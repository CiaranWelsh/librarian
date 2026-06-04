#[tokio::test(flavor = "multi_thread")]
async fn cli_query_prints_hits_from_daemon() {
    let (base, _h) = query_daemon::test_support::spawn().await;
    let out = tokio::task::spawn_blocking(move || {
        std::process::Command::new(env!("CARGO_BIN_EXE_librarian"))
            .args(["query", "demo", "apple", "--limit", "5", "--daemon", &base])
            .output()
            .expect("run librarian")
    })
    .await
    .unwrap();
    assert!(
        out.status.success(),
        "exit failure; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("apple"), "stdout was: {stdout}");
}
