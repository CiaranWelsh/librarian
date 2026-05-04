//! E2E: drive the `librarian` binary against the test Qdrant. Each test gets a
//! unique collection so they're hermetic against the shared instance.

use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;
use std::path::Path;
use tempfile::TempDir;

fn qdrant_url() -> String {
    std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6533".into())
}

fn unique_collection(label: &str) -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    format!("librarian-cli-{label}-{nanos}")
}

fn write_config(dir: &Path, collection: &str) -> std::path::PathBuf {
    let cfg_path = dir.join("librarian.toml");
    let body = format!(
        r#"collection = "{collection}"

[qdrant]
url = "{url}"

[paths]
cache = "{cache}"
manifest = "{manifest}"

[embedder]
kind = "stub"

[ingest]
content_type = "book"
extractor = "text"
"#,
        url = qdrant_url(),
        cache = dir.join("cache").display(),
        manifest = dir.join("manifest.sqlite").display(),
    );
    std::fs::write(&cfg_path, body).unwrap();
    cfg_path
}

fn fixture_path() -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.push("tests/fixtures/sample.txt");
    p
}

#[test]
fn ingest_status_remove_status_round_trip() {
    let collection = unique_collection("rt");
    let dir = TempDir::new().unwrap();
    let cfg = write_config(dir.path(), &collection);
    let fixture = fixture_path();

    // Before checking Qdrant via the CLI, ensure Qdrant is reachable. Using the
    // `status` command with no prior ingest as a probe — if the binary fails
    // here we skip the rest with a printed note.
    let probe = Command::cargo_bin("librarian").unwrap()
        .arg("status").arg("--config").arg(&cfg)
        .assert();
    if !probe.get_output().status.success() {
        eprintln!("skip: Qdrant not reachable at {}", qdrant_url());
        return;
    }

    Command::cargo_bin("librarian").unwrap()
        .args(["ingest", "--config"]).arg(&cfg).arg(&fixture)
        .assert()
        .success()
        .stdout(contains("ok\t").and(contains("chunks=3")));

    Command::cargo_bin("librarian").unwrap()
        .args(["status", "--config"]).arg(&cfg)
        .assert()
        .success()
        .stdout(contains("points: 3"));

    let source_id = fixture.display().to_string();
    Command::cargo_bin("librarian").unwrap()
        .args(["remove", "--config"]).arg(&cfg)
        .args(["--source-id", &source_id])
        .assert()
        .success()
        .stdout(contains(&format!("removed {source_id}")));

    Command::cargo_bin("librarian").unwrap()
        .args(["status", "--config"]).arg(&cfg)
        .assert()
        .success()
        .stdout(contains("points: 0"));
}

#[test]
fn missing_config_file_yields_human_readable_error_and_nonzero_exit() {
    Command::cargo_bin("librarian").unwrap()
        .args(["status", "--config", "/no/such/config.toml"])
        .assert()
        .failure()
        .stderr(contains("config io"));
}

#[test]
fn malformed_config_yields_parse_error() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("bad.toml");
    std::fs::write(&cfg, "not = a [valid").unwrap();

    Command::cargo_bin("librarian").unwrap()
        .args(["status", "--config"]).arg(&cfg)
        .assert()
        .failure()
        .stderr(contains("config parse"));
}

#[test]
fn unimplemented_subcommands_exit_with_distinct_code() {
    let dir = TempDir::new().unwrap();
    let cfg = write_config(dir.path(), &unique_collection("stub"));

    Command::cargo_bin("librarian").unwrap()
        .args(["start", "--config"]).arg(&cfg)
        .assert()
        .code(64);
}

#[test]
fn snapshot_invokes_orchestrator_and_prints_id() {
    use std::process::Command as StdCmd;
    let dir = TempDir::new().unwrap();
    let collection = unique_collection("snap");
    // Build a config with a `snapshots` path so the orchestrator can build.
    let cfg_path = dir.path().join("librarian.toml");
    let body = format!(
        r#"collection = "{collection}"

[qdrant]
url = "{url}"

[paths]
cache = "{cache}"
manifest = "{manifest}"
snapshots = "{nas}"

[embedder]
kind = "stub"

[ingest]
content_type = "book"
extractor = "text"
"#,
        url = qdrant_url(),
        cache = dir.path().join("cache").display(),
        manifest = dir.path().join("manifest.sqlite").display(),
        nas = dir.path().join("nas").display(),
    );
    std::fs::write(&cfg_path, body).unwrap();

    // Probe Qdrant; skip if absent.
    let probe = Command::cargo_bin("librarian").unwrap()
        .arg("status").arg("--config").arg(&cfg_path).assert();
    if !probe.get_output().status.success() { eprintln!("skip: no Qdrant"); return; }

    // Ingest a tiny fixture so the snapshot has content.
    let fixture = fixture_path();
    let _ = StdCmd::new(env!("CARGO_BIN_EXE_librarian"))
        .args(["ingest", "--config"]).arg(&cfg_path).arg(&fixture).output();

    Command::cargo_bin("librarian").unwrap()
        .args(["snapshot", "--config"]).arg(&cfg_path)
        .assert().success().stdout(contains("snapshot\tid="));
}
