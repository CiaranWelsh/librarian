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
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
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
corpus_root = "{corpus_root}"
"#,
        url = qdrant_url(),
        cache = dir.join("cache").display(),
        manifest = dir.join("manifest.sqlite").display(),
        corpus_root = fixtures_root().display(),
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

/// Corpus root for tests: the fixtures dir, so ingest accepts `sample.txt` under it (ADR-0007).
fn fixtures_root() -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.push("tests/fixtures");
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
    let probe = Command::cargo_bin("librarian")
        .unwrap()
        .arg("status")
        .arg("--config")
        .arg(&cfg)
        .assert();
    if !probe.get_output().status.success() {
        eprintln!("skip: Qdrant not reachable at {}", qdrant_url());
        return;
    }

    // The recursive chunker (default since issue 027) packs the small `sample.txt`
    // fixture into a single chunk; the round-trip checks the ingest -> status ->
    // remove invariant, so the exact count just has to match the default chunker.
    Command::cargo_bin("librarian")
        .unwrap()
        .args(["ingest", "--config"])
        .arg(&cfg)
        .arg(&fixture)
        .assert()
        .success()
        .stdout(contains("ok\t").and(contains("chunks=1")));

    Command::cargo_bin("librarian")
        .unwrap()
        .args(["status", "--config"])
        .arg(&cfg)
        .assert()
        .success()
        .stdout(contains("points: 1"));

    // G6 (ADR-0007) makes the ingested source_id relative to corpus_root, so the
    // fixture under `fixtures_root()` is just "sample.txt" — remove must key off
    // that canonical id, not the absolute path it was once.
    let source_id = "sample.txt".to_string();
    Command::cargo_bin("librarian")
        .unwrap()
        .args(["remove", "--config"])
        .arg(&cfg)
        .args(["--source-id", &source_id])
        .assert()
        .success()
        .stdout(contains(&format!("removed {source_id}")));

    Command::cargo_bin("librarian")
        .unwrap()
        .args(["status", "--config"])
        .arg(&cfg)
        .assert()
        .success()
        .stdout(contains("points: 0"));
}

#[test]
fn missing_config_file_yields_human_readable_error_and_nonzero_exit() {
    Command::cargo_bin("librarian")
        .unwrap()
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

    Command::cargo_bin("librarian")
        .unwrap()
        .args(["status", "--config"])
        .arg(&cfg)
        .assert()
        .failure()
        .stderr(contains("config parse"));
}

// `start`/`stop`/`restart` are now implemented (slice 015). No remaining
// stub subcommands carry a distinct exit code.

#[test]
fn add_requires_to_flag() {
    Command::cargo_bin("librarian")
        .unwrap()
        .args(["add", "x.pdf"])
        .assert()
        .failure();
    Command::cargo_bin("librarian")
        .unwrap()
        .args(["add", "--help"])
        .assert()
        .success()
        .stdout(contains("--to"));
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
corpus_root = "{corpus_root}"
"#,
        url = qdrant_url(),
        cache = dir.path().join("cache").display(),
        manifest = dir.path().join("manifest.sqlite").display(),
        nas = dir.path().join("nas").display(),
        corpus_root = fixtures_root().display(),
    );
    std::fs::write(&cfg_path, body).unwrap();

    // Probe Qdrant; skip if absent.
    let probe = Command::cargo_bin("librarian")
        .unwrap()
        .arg("status")
        .arg("--config")
        .arg(&cfg_path)
        .assert();
    if !probe.get_output().status.success() {
        eprintln!("skip: no Qdrant");
        return;
    }

    // Ingest a tiny fixture so the snapshot has content.
    let fixture = fixture_path();
    let _ = StdCmd::new(env!("CARGO_BIN_EXE_librarian"))
        .args(["ingest", "--config"])
        .arg(&cfg_path)
        .arg(&fixture)
        .output();

    Command::cargo_bin("librarian")
        .unwrap()
        .args(["snapshot", "--config"])
        .arg(&cfg_path)
        .assert()
        .success()
        .stdout(contains("snapshot\tid="));
}
