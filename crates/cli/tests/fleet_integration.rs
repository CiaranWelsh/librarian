//! Slice 015: supervisor + fleet registry. Drives the `librarian` binary
//! through `start`/`stop`/`restart`/`status`, with the registry isolated to a
//! tempfile and the child binary pointed at the real `librarian-collection`
//! pre-pointed at the test Qdrant.

use assert_cmd::Command;
use predicates::str::contains;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Locate the `librarian-collection` binary built by cargo. Test executables
/// live in `target/debug/deps/<test-binary>`; siblings are at `target/debug/`.
fn child_bin() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let target_debug = exe.parent().expect("deps").parent().expect("debug").to_path_buf();
    target_debug.join("librarian-collection")
}

fn qdrant_url() -> String {
    std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6533".into())
}

fn unique_name(label: &str) -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    format!("fleet-{label}-{nanos}")
}

fn write_collection_config(dir: &Path, collection: &str) -> std::path::PathBuf {
    let cfg = dir.join("librarian.toml");
    let body = format!(
        r#"collection = "{collection}"

[qdrant]
url = "{url}"

[paths]
cache = "{cache}"
manifest = "{manifest}"

[embedder]
kind = "stub"
"#,
        url = qdrant_url(),
        cache = dir.join("cache").display(),
        manifest = dir.join("manifest.sqlite").display(),
    );
    std::fs::write(&cfg, body).unwrap();
    cfg
}

fn run_librarian(reg: &Path) -> Command {
    let mut c = Command::cargo_bin("librarian").unwrap();
    c.env("LIBRARIAN_FLEET_DB", reg)
     .env("LIBRARIAN_COLLECTION_BIN", child_bin());
    c
}

fn qdrant_reachable() -> bool {
    use adapter_indexer_qdrant::QdrantIndexer;
    QdrantIndexer::open(&qdrant_url(), &unique_name("probe"), 32).is_ok()
}

#[test]
fn empty_fleet_prints_marker() {
    let dir = TempDir::new().unwrap();
    let reg = dir.path().join("fleet.sqlite");
    run_librarian(&reg)
        .arg("status")
        .assert()
        .success()
        .stdout(contains("(empty fleet)"));
}

#[test]
fn start_two_collections_listed_with_distinct_ports() {
    if !qdrant_reachable() { eprintln!("skip: no Qdrant"); return; }
    let dir = TempDir::new().unwrap();
    let reg = dir.path().join("fleet.sqlite");

    let cfg_a = write_collection_config(&dir.path().join("a"), &unique_name("a"));
    let cfg_b = write_collection_config(&dir.path().join("b"), &unique_name("b"));
    std::fs::create_dir_all(cfg_a.parent().unwrap()).ok();
    std::fs::create_dir_all(cfg_b.parent().unwrap()).ok();

    run_librarian(&reg).args(["start", "coll-a", "--config"]).arg(&cfg_a).assert().success();
    run_librarian(&reg).args(["start", "coll-b", "--config"]).arg(&cfg_b).assert().success();

    let out = run_librarian(&reg).arg("status").assert().success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains("coll-a"));
    assert!(stdout.contains("coll-b"));
    // Distinct ports — extract `port=NNNN` for each line, assert ≠.
    let port_a = extract_port(&stdout, "coll-a").expect("port a");
    let port_b = extract_port(&stdout, "coll-b").expect("port b");
    assert_ne!(port_a, port_b);

    // Cleanup
    run_librarian(&reg).args(["stop", "coll-a"]).assert().success();
    run_librarian(&reg).args(["stop", "coll-b"]).assert().success();
}

#[test]
fn stop_marks_collection_stopped() {
    if !qdrant_reachable() { eprintln!("skip: no Qdrant"); return; }
    let dir = TempDir::new().unwrap();
    let reg = dir.path().join("fleet.sqlite");
    let cfg = write_collection_config(dir.path(), &unique_name("stop"));

    run_librarian(&reg).args(["start", "c1", "--config"]).arg(&cfg).assert().success()
        .stdout(contains("started"));
    run_librarian(&reg).arg("status").assert().success().stdout(contains("status=running"));

    run_librarian(&reg).args(["stop", "c1"]).assert().success().stdout(contains("stopped"));
    run_librarian(&reg).arg("status").assert().success().stdout(contains("status=stopped"));
}

#[test]
fn idempotent_start_and_stop() {
    if !qdrant_reachable() { eprintln!("skip: no Qdrant"); return; }
    let dir = TempDir::new().unwrap();
    let reg = dir.path().join("fleet.sqlite");
    let cfg = write_collection_config(dir.path(), &unique_name("idem"));

    run_librarian(&reg).args(["start", "c1", "--config"]).arg(&cfg).assert().success();
    // Second start: no-op.
    run_librarian(&reg).args(["start", "c1", "--config"]).arg(&cfg).assert().success()
        .stdout(contains("already running"));
    run_librarian(&reg).args(["stop", "c1"]).assert().success();
    // Second stop: no-op.
    run_librarian(&reg).args(["stop", "c1"]).assert().success().stdout(contains("already stopped"));
}

#[test]
fn external_kill_is_reflected_on_next_status() {
    if !qdrant_reachable() { eprintln!("skip: no Qdrant"); return; }
    let dir = TempDir::new().unwrap();
    let reg = dir.path().join("fleet.sqlite");
    let cfg = write_collection_config(dir.path(), &unique_name("crash"));

    let started = run_librarian(&reg).args(["start", "c1", "--config"]).arg(&cfg).output().unwrap();
    let stdout = String::from_utf8_lossy(&started.stdout).to_string();
    let pid: i32 = stdout
        .lines()
        .filter_map(|l| l.split('\t').find_map(|p| p.strip_prefix("pid=")))
        .next().expect("pid line").parse().expect("pid int");

    // Externally kill the child.
    unsafe { libc::kill(pid, libc::SIGKILL); }
    // Give the OS a moment to reap.
    std::thread::sleep(std::time::Duration::from_millis(200));

    run_librarian(&reg).arg("status").assert().success().stdout(contains("status=stopped"));
}

#[test]
fn stop_of_unknown_name_is_a_noop() {
    let dir = TempDir::new().unwrap();
    let reg = dir.path().join("fleet.sqlite");
    run_librarian(&reg).args(["stop", "nope"]).assert().success()
        .stdout(contains("not registered"));
}

fn extract_port(stdout: &str, name: &str) -> Option<u16> {
    let line = stdout.lines().find(|l| l.starts_with(name))?;
    line.split('\t').find_map(|p| p.strip_prefix("port=")?.parse().ok())
}
