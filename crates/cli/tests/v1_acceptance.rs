//! Slice 018 — v1 acceptance e2e. Drives the full lifecycle end-to-end
//! against the test Qdrant: ingest → status → idempotent re-ingest →
//! remove → snapshot → restore → fleet start/status/stop. Skipped silently
//! if Qdrant is unreachable.

use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;
use std::path::{Path, PathBuf};
use std::process::Command as StdCmd;
use tempfile::TempDir;

fn qdrant_url() -> String {
    std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6533".into())
}

fn qdrant_reachable() -> bool {
    use adapter_indexer_qdrant::QdrantIndexer;
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    QdrantIndexer::open(&qdrant_url(), &format!("librarian-probe-{nanos}"), 32).is_ok()
}

fn unique_collection() -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    format!("librarian-v1-acceptance-{nanos}")
}

fn child_bin() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent().expect("deps").parent().expect("debug").join("librarian-collection")
}

fn write_config(dir: &Path, collection: &str, with_snapshots: bool) -> PathBuf {
    let cfg_path = dir.join("librarian.toml");
    let snapshots_line = if with_snapshots {
        format!("snapshots = \"{}\"", dir.join("nas").display())
    } else { "".to_string() };

    let body = format!(
        r#"collection = "{collection}"

[qdrant]
url = "{url}"

[paths]
cache = "{cache}"
manifest = "{manifest}"
{snapshots}

[embedder]
kind = "stub"

[ingest]
content_type = "book"
extractor = "text"
"#,
        url = qdrant_url(),
        cache = dir.join("cache").display(),
        manifest = dir.join("manifest.sqlite").display(),
        snapshots = snapshots_line,
    );
    std::fs::write(&cfg_path, body).unwrap();
    cfg_path
}

fn run(reg: Option<&Path>) -> Command {
    let mut c = Command::cargo_bin("librarian").unwrap();
    if let Some(reg) = reg {
        c.env("LIBRARIAN_FLEET_DB", reg)
         .env("LIBRARIAN_COLLECTION_BIN", child_bin());
    }
    c
}

/// One long test that walks the entire v1 surface area. Ordering matters —
/// each phase depends on state set up by the previous one.
#[test]
fn v1_full_lifecycle_against_real_qdrant() {
    if !qdrant_reachable() { eprintln!("skip: no Qdrant"); return; }

    let dir = TempDir::new().unwrap();
    let collection = unique_collection();
    let cfg = write_config(dir.path(), &collection, /*snapshots=*/ true);

    // Build a 2-file fixture: 3 paragraphs in a.txt, 2 in b.txt → 5 chunks total.
    let fixtures = dir.path().join("fixtures");
    std::fs::create_dir_all(&fixtures).unwrap();
    std::fs::write(fixtures.join("a.txt"), "para 1\n\npara 2\n\npara 3").unwrap();
    std::fs::write(fixtures.join("b.txt"), "alpha\n\nbeta").unwrap();

    // ── 1. INGEST ────────────────────────────────────────────────────────
    run(None).args(["ingest", "--config"]).arg(&cfg).arg(&fixtures)
        .assert().success().stdout(contains("ok\t").and(contains("chunks=")));

    run(None).args(["status", "--config"]).arg(&cfg)
        .assert().success().stdout(contains("points: 5"));

    // ── 2. IDEMPOTENT RE-INGEST ──────────────────────────────────────────
    let out = run(None).args(["ingest", "--config"]).arg(&cfg).arg(&fixtures)
        .output().unwrap();
    assert!(out.status.success(), "second ingest succeeds");
    run(None).args(["status", "--config"]).arg(&cfg)
        .assert().success().stdout(contains("points: 5"));
    // Manifest reflects cache hits on the second run.
    let status = String::from_utf8_lossy(&run(None).args(["status", "--config"]).arg(&cfg)
        .output().unwrap().stdout).to_string();
    assert!(status.contains("cached="), "manifest summary present in status: {status}");

    // ── 3. UPDATE (modify input → fewer chunks; orphans drop) ────────────
    std::fs::write(fixtures.join("a.txt"), "single para only").unwrap();
    run(None).args(["ingest", "--config"]).arg(&cfg).arg(&fixtures).assert().success();
    run(None).args(["status", "--config"]).arg(&cfg)
        .assert().success().stdout(contains("points: 3"));

    // ── 4. REMOVE ────────────────────────────────────────────────────────
    let source_id_a = fixtures.join("a.txt").display().to_string();
    run(None).args(["remove", "--config"]).arg(&cfg)
        .args(["--source-id", &source_id_a])
        .assert().success().stdout(contains("removed"));
    run(None).args(["status", "--config"]).arg(&cfg)
        .assert().success().stdout(contains("points: 2"));

    // ── 5. SNAPSHOT to NAS ───────────────────────────────────────────────
    let snap_out = run(None).args(["snapshot", "--config"]).arg(&cfg).output().unwrap();
    let snap_stdout = String::from_utf8_lossy(&snap_out.stdout).to_string();
    let snap_id: String = snap_stdout
        .lines()
        .filter_map(|l| l.split('\t').find_map(|p| p.strip_prefix("id=")))
        .next().expect("snapshot id printed").to_string();
    let nas_file = dir.path().join("nas").join(&snap_id);
    assert!(nas_file.exists(), "snapshot file landed on NAS: {}", nas_file.display());

    // ── 6. RESTORE roundtrip — wipe the collection, restore from snapshot ──
    // Use the indexer directly to delete everything for the source we still have.
    use adapter_indexer_qdrant::QdrantIndexer;
    use librarian_domain::{Indexer, SourceId};
    let ix = QdrantIndexer::open(&qdrant_url(), &collection, 32).unwrap();
    let source_id_b = SourceId(fixtures.join("b.txt").display().to_string());
    ix.delete_by_source_id(&source_id_b).unwrap();
    assert_eq!(ix.count().unwrap(), 0, "collection wiped");

    run(None).args(["restore", "--config"]).arg(&cfg).arg(&snap_id)
        .assert().success().stdout(contains("restore"));
    let ix2 = QdrantIndexer::open(&qdrant_url(), &collection, 32).unwrap();
    assert_eq!(ix2.count().unwrap(), 2, "snapshot restored prior 2 points");

    // ── 7. FLEET LIFECYCLE — start, status, stop ─────────────────────────
    let fleet_db = dir.path().join("fleet.sqlite");
    run(Some(&fleet_db)).args(["start", "v1", "--config"]).arg(&cfg)
        .assert().success().stdout(contains("started"));
    run(Some(&fleet_db)).arg("status").assert().success()
        .stdout(contains("v1").and(contains("status=running")));
    run(Some(&fleet_db)).args(["stop", "v1"]).assert().success()
        .stdout(contains("stopped"));
    run(Some(&fleet_db)).arg("status").assert().success()
        .stdout(contains("status=stopped"));
}

/// Smoke-query via the MCP server: spawn the binary, issue a `tools/call` for
/// `search`, verify hits come back. Demonstrates §6 acceptance language about
/// "Smoke queries from Claude Code on Mac return relevant results".
#[test]
fn v1_mcp_smoke_query_returns_hits_for_ingested_paragraph() {
    if !qdrant_reachable() { eprintln!("skip: no Qdrant"); return; }

    let dir = TempDir::new().unwrap();
    let collection = unique_collection();
    let cfg = write_config(dir.path(), &collection, /*snapshots=*/ false);

    let fixtures = dir.path().join("fixtures");
    std::fs::create_dir_all(&fixtures).unwrap();
    std::fs::write(fixtures.join("a.txt"), "the dragon was huge\n\nbut the knight was brave").unwrap();

    run(None).args(["ingest", "--config"]).arg(&cfg).arg(&fixtures).assert().success();

    // Drive `librarian-collection` over stdio.
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command as StdCmd2, Stdio};
    let mut child = StdCmd2::new(child_bin())
        .arg("--config").arg(&cfg)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().expect("spawn");
    let mut sin = child.stdin.take().unwrap();
    let mut sout = BufReader::new(child.stdout.take().unwrap());

    // tools/call → search
    let req = serde_json::json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"search","arguments":{"query":"dragon","k":3}}
    });
    writeln!(sin, "{req}").unwrap();
    sin.flush().unwrap();
    let mut line = String::new();
    sout.read_line(&mut line).unwrap();
    let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
    let body = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(body).unwrap();
    assert!(!parsed["hits"].as_array().unwrap().is_empty(), "MCP search returned hits");

    drop(sin);
    let _ = child.wait();
}

/// Demonstrate that Snapshot Orchestrator's retention budget actually prunes —
/// take 4 snapshots, retention=2, verify only 2 files remain on NAS.
#[test]
fn v1_snapshot_retention_prunes_old_snapshots() {
    if !qdrant_reachable() { eprintln!("skip: no Qdrant"); return; }

    let dir = TempDir::new().unwrap();
    let collection = unique_collection();
    // Override the default retention=5 to retention=2.
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

[snapshot]
retention = 2
"#,
        url = qdrant_url(),
        cache = dir.path().join("cache").display(),
        manifest = dir.path().join("manifest.sqlite").display(),
        nas = dir.path().join("nas").display(),
    );
    std::fs::write(&cfg_path, body).unwrap();

    // Seed something so snapshots aren't empty.
    let fixtures = dir.path().join("fixtures");
    std::fs::create_dir_all(&fixtures).unwrap();
    std::fs::write(fixtures.join("seed.txt"), "x").unwrap();
    let _ = StdCmd::new(env!("CARGO_BIN_EXE_librarian"))
        .args(["ingest", "--config"]).arg(&cfg_path).arg(&fixtures).output();

    for _ in 0..4 {
        let _ = StdCmd::new(env!("CARGO_BIN_EXE_librarian"))
            .args(["snapshot", "--config"]).arg(&cfg_path).output().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
    }

    let count = std::fs::read_dir(dir.path().join("nas")).unwrap().count();
    assert_eq!(count, 2, "retention=2 leaves 2 snapshot files on NAS");
}
