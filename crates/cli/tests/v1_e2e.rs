//! v1 end-to-end suite. One journey per test, all driven through a small set
//! of harnesses that hide CLI/MCP/Qdrant infrastructure behind domain-meaningful
//! methods.
//!
//! These tests treat the CLI binary as the system under test. State for the
//! *arrange* phase is set via direct adapter APIs (QdrantProbe), not by
//! running the CLI through prior journeys.

use std::path::{Path, PathBuf};
use tempfile::TempDir;

mod harness {
    use super::*;
    use adapter_indexer_qdrant::QdrantIndexer;
    use assert_cmd::Command;
    use librarian_domain::{Indexer, SourceId};
    use serde_json::{json, Value};
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Child, ChildStdin, ChildStdout, Stdio};

    /// Resolve the test Qdrant URL from the env or the local default.
    pub fn qdrant_url() -> String {
        std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6533".into())
    }

    /// Locate the `librarian-collection` binary by walking up from the test
    /// executable's location (target/debug/deps → target/debug).
    pub fn collection_bin() -> PathBuf {
        let exe = std::env::current_exe().expect("current_exe");
        exe.parent()
            .expect("deps")
            .parent()
            .expect("debug")
            .join("librarian-collection")
    }

    fn unique_label(label: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{label}-{nanos}")
    }

    // ─── Domain return types ─────────────────────────────────────────────

    #[derive(Debug)]
    pub struct IngestSummary {
        pub success_count: usize,
        pub fail_count: usize,
        pub raw_stdout: String,
    }

    #[derive(Debug)]
    pub struct CollectionStatus {
        pub points: u64,
        pub success_rows: u64,
        pub cached_rows: u64,
        pub failed_rows: u64,
    }

    #[derive(Debug)]
    #[allow(dead_code)]
    pub struct FleetEntry {
        pub name: String,
        pub status: String,
        pub port: u16,
        pub pid: Option<i32>,
    }

    #[derive(Debug)]
    pub struct SnapshotId(pub String);

    // ─── LibrarianHarness ────────────────────────────────────────────────

    /// Wraps the `librarian` binary. Owns its tempdir, config, fleet registry,
    /// and unique Qdrant collection name. Drop = teardown of fs state; the
    /// Qdrant collection itself uses unique nanos-suffix names so concurrent
    /// runs don't collide.
    pub struct LibrarianHarness {
        pub workdir: TempDir,
        pub collection: String,
        pub config: PathBuf,
        pub fleet_db: PathBuf,
        pub embedder_dim: usize,
    }

    impl LibrarianHarness {
        /// Open a fresh harness. `None` if Qdrant is unreachable — tests that
        /// receive `None` should `eprintln!("skip"); return;` (the project's
        /// established skip pattern).
        pub fn fresh(label: &str) -> Option<Self> {
            // Probe Qdrant before doing any other setup.
            let probe = unique_label("librarian-probe");
            QdrantIndexer::open(&qdrant_url(), &probe, 32).ok()?;

            let workdir = TempDir::new().ok()?;
            let collection = unique_label(&format!("librarian-e2e-{label}"));
            let fleet_db = workdir.path().join("fleet.sqlite");
            let config = Self::write_config(workdir.path(), &collection);

            Some(Self {
                workdir,
                collection,
                config,
                fleet_db,
                embedder_dim: 32,
            })
        }

        fn write_config(dir: &Path, collection: &str) -> PathBuf {
            let p = dir.join("librarian.toml");
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

[snapshot]
retention = 5
"#,
                url = qdrant_url(),
                cache = dir.join("cache").display(),
                manifest = dir.join("manifest.sqlite").display(),
                nas = dir.join("nas").display(),
            );
            std::fs::write(&p, body).unwrap();
            p
        }

        fn cli(&self) -> Command {
            let mut c = Command::cargo_bin("librarian").unwrap();
            c.env("LIBRARIAN_FLEET_DB", &self.fleet_db)
                .env("LIBRARIAN_COLLECTION_BIN", collection_bin());
            c
        }

        // ── CLI journeys ────────────────────────────────────────────────

        pub fn ingest(&self, input: &Path) -> Result<IngestSummary, String> {
            let out = self
                .cli()
                .args(["ingest", "--config"])
                .arg(&self.config)
                .arg(input)
                .output()
                .map_err(|e| e.to_string())?;
            if !out.status.success() {
                return Err(format!(
                    "ingest failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let success_count = stdout.lines().filter(|l| l.starts_with("ok\t")).count();
            let fail_count = stdout.lines().filter(|l| l.starts_with("fail\t")).count();
            Ok(IngestSummary {
                success_count,
                fail_count,
                raw_stdout: stdout,
            })
        }

        pub fn status(&self) -> Result<CollectionStatus, String> {
            let out = self
                .cli()
                .args(["status", "--config"])
                .arg(&self.config)
                .output()
                .map_err(|e| e.to_string())?;
            if !out.status.success() {
                return Err(format!(
                    "status failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let points = parse_after(&stdout, "points: ").parse().unwrap_or(0);
            let s = parse_kv(&stdout, "success=");
            let c = parse_kv(&stdout, "cached=");
            let f = parse_kv(&stdout, "failed=");
            Ok(CollectionStatus {
                points,
                success_rows: s,
                cached_rows: c,
                failed_rows: f,
            })
        }

        pub fn remove(&self, source_id: &str) -> Result<(), String> {
            let out = self
                .cli()
                .args(["remove", "--config"])
                .arg(&self.config)
                .args(["--source-id", source_id])
                .output()
                .map_err(|e| e.to_string())?;
            if !out.status.success() {
                return Err(String::from_utf8_lossy(&out.stderr).to_string());
            }
            Ok(())
        }

        pub fn snapshot(&self) -> Result<SnapshotId, String> {
            let out = self
                .cli()
                .args(["snapshot", "--config"])
                .arg(&self.config)
                .output()
                .map_err(|e| e.to_string())?;
            if !out.status.success() {
                return Err(String::from_utf8_lossy(&out.stderr).to_string());
            }
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let id = stdout
                .lines()
                .filter_map(|l| l.split('\t').find_map(|p| p.strip_prefix("id=")))
                .next()
                .ok_or("snapshot id not in stdout")?
                .to_string();
            Ok(SnapshotId(id))
        }

        pub fn restore(&self, id: &SnapshotId) -> Result<(), String> {
            let out = self
                .cli()
                .args(["restore", "--config"])
                .arg(&self.config)
                .arg(&id.0)
                .output()
                .map_err(|e| e.to_string())?;
            if !out.status.success() {
                return Err(String::from_utf8_lossy(&out.stderr).to_string());
            }
            Ok(())
        }

        // ── Fleet journeys ──────────────────────────────────────────────

        pub fn fleet_start(&self, name: &str) -> Result<(), String> {
            let out = self
                .cli()
                .args(["start", name, "--config"])
                .arg(&self.config)
                .output()
                .map_err(|e| e.to_string())?;
            if !out.status.success() {
                return Err(String::from_utf8_lossy(&out.stderr).to_string());
            }
            Ok(())
        }

        pub fn fleet_stop(&self, name: &str) -> Result<(), String> {
            let out = self
                .cli()
                .args(["stop", name])
                .output()
                .map_err(|e| e.to_string())?;
            if !out.status.success() {
                return Err(String::from_utf8_lossy(&out.stderr).to_string());
            }
            Ok(())
        }

        pub fn fleet_list(&self) -> Result<Vec<FleetEntry>, String> {
            let out = self
                .cli()
                .arg("status")
                .output()
                .map_err(|e| e.to_string())?;
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let mut entries = Vec::new();
            for line in stdout.lines() {
                if line.starts_with('(') {
                    continue;
                }
                if let Some(name) = line.split('\t').next() {
                    let port: u16 = parse_after(line, "port=").parse().unwrap_or(0);
                    let status = line
                        .split('\t')
                        .find_map(|p| p.strip_prefix("status="))
                        .unwrap_or("")
                        .to_string();
                    let pid = line
                        .split('\t')
                        .find_map(|p| p.strip_prefix("pid="))
                        .and_then(|s| s.parse().ok());
                    entries.push(FleetEntry {
                        name: name.to_string(),
                        status,
                        port,
                        pid,
                    });
                }
            }
            Ok(entries)
        }

        // ── State setup / inspection (direct, not through CLI) ──────────

        pub fn probe(&self) -> QdrantProbe {
            QdrantProbe::open(&self.collection, self.embedder_dim as u64)
        }

        /// Path to the NAS dir holding snapshots — caller can inspect retention.
        pub fn nas_dir(&self) -> PathBuf {
            self.workdir.path().join("nas")
        }
    }

    // ─── QdrantProbe ─────────────────────────────────────────────────────

    /// Direct Qdrant access for arrange/assert phases. Not part of the system
    /// under test — used to set up state and verify outcomes without going
    /// through the CLI.
    pub struct QdrantProbe {
        pub indexer: QdrantIndexer,
    }

    impl QdrantProbe {
        pub fn open(collection: &str, dim: u64) -> Self {
            Self {
                indexer: QdrantIndexer::open(&qdrant_url(), collection, dim).expect("qdrant"),
            }
        }

        pub fn point_count(&self) -> u64 {
            self.indexer.count().unwrap_or(0)
        }

        pub fn wipe_source(&self, source_id: &str) {
            let _ = self
                .indexer
                .delete_by_source_id(&SourceId(source_id.into()));
        }
    }

    // ─── McpHarness ──────────────────────────────────────────────────────

    pub struct McpHarness {
        child: Child,
        stdin: ChildStdin,
        stdout: BufReader<ChildStdout>,
    }

    #[derive(Debug)]
    #[allow(dead_code)]
    pub struct McpHit {
        pub source_id: String,
        pub chunk_index: u32,
        pub text: String,
    }

    impl McpHarness {
        pub fn spawn(config: &Path) -> Self {
            let mut child = std::process::Command::new(collection_bin())
                .arg("--config")
                .arg(config)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn librarian-collection");
            let stdin = child.stdin.take().unwrap();
            let stdout = BufReader::new(child.stdout.take().unwrap());
            Self {
                child,
                stdin,
                stdout,
            }
        }

        fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
            let req = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
            writeln!(self.stdin, "{req}").unwrap();
            self.stdin.flush().unwrap();
            let mut line = String::new();
            self.stdout.read_line(&mut line).unwrap();
            serde_json::from_str(&line).unwrap()
        }

        pub fn search(&mut self, query: &str, k: u64) -> Vec<McpHit> {
            let resp = self.request(
                1,
                "tools/call",
                json!({
                    "name":"search","arguments":{"query":query,"k":k}
                }),
            );
            let body = resp["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or("{}");
            let parsed: Value = serde_json::from_str(body).unwrap_or(Value::Null);
            parsed["hits"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|h| McpHit {
                    source_id: h["source_id"].as_str().unwrap_or("").to_string(),
                    chunk_index: h["chunk_index"].as_u64().unwrap_or(0) as u32,
                    text: h["text"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        }

        pub fn list_documents(&mut self) -> Vec<String> {
            let resp = self.request(
                2,
                "tools/call",
                json!({
                    "name":"list_documents","arguments":{}
                }),
            );
            let body = resp["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or("{}");
            let parsed: Value = serde_json::from_str(body).unwrap_or(Value::Null);
            parsed["documents"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|d| d["source_id"].as_str().unwrap_or("").to_string())
                .collect()
        }
    }

    impl Drop for McpHarness {
        fn drop(&mut self) {
            let _ = self.child.wait();
        }
    }

    // ─── FixtureBuilder ──────────────────────────────────────────────────

    /// Small test-data builder: writes plaintext fixtures to a chosen directory.
    pub struct FixtureBuilder {
        files: Vec<(String, String)>, // (filename, body)
    }

    impl FixtureBuilder {
        pub fn new() -> Self {
            Self { files: Vec::new() }
        }

        /// One file with `n` paragraphs separated by blank lines (each paragraph
        /// short and easily searchable).
        pub fn with_paragraphs(mut self, name: &str, n: usize) -> Self {
            let body = (0..n)
                .map(|i| format!("paragraph {i}"))
                .collect::<Vec<_>>()
                .join("\n\n");
            self.files.push((name.into(), body));
            self
        }

        pub fn with_file(mut self, name: &str, body: &str) -> Self {
            self.files.push((name.into(), body.into()));
            self
        }

        pub fn write_to(&self, dir: &Path) -> PathBuf {
            let target = dir.join("fixtures");
            std::fs::create_dir_all(&target).unwrap();
            for (name, body) in &self.files {
                std::fs::write(target.join(name), body).unwrap();
            }
            target
        }
    }

    // ─── Helpers ─────────────────────────────────────────────────────────

    fn parse_after(s: &str, key: &str) -> String {
        s.lines()
            .find_map(|l| {
                l.find(key).map(|i| {
                    let rest = &l[i + key.len()..];
                    rest.split_whitespace().next().unwrap_or("").to_string()
                })
            })
            .unwrap_or_default()
    }

    fn parse_kv(s: &str, key: &str) -> u64 {
        parse_after(s, key).parse().unwrap_or(0)
    }
}

use harness::*;

// ─── J1: First-time ingestion ────────────────────────────────────────────

#[test]
fn e2e_first_time_ingestion_lands_chunks_in_qdrant() {
    let Some(h) = LibrarianHarness::fresh("ingest") else {
        eprintln!("skip: no Qdrant");
        return;
    };
    let fixtures = FixtureBuilder::new()
        .with_paragraphs("a.txt", 3)
        .with_paragraphs("b.txt", 2)
        .write_to(h.workdir.path());

    let summary = h.ingest(&fixtures).expect("ingest");

    assert_eq!(summary.success_count, 2, "two files succeeded");
    assert_eq!(summary.fail_count, 0);
    assert_eq!(h.probe().point_count(), 5, "3 + 2 chunks land");

    let s = h.status().expect("status");
    assert_eq!(s.points, 5);
}

// ─── J2: Idempotent re-ingestion ─────────────────────────────────────────

#[test]
fn e2e_second_ingest_hits_cache_and_yields_no_new_points() {
    let Some(h) = LibrarianHarness::fresh("idem") else {
        eprintln!("skip: no Qdrant");
        return;
    };
    let fixtures = FixtureBuilder::new()
        .with_paragraphs("a.txt", 3)
        .write_to(h.workdir.path());

    h.ingest(&fixtures).expect("first ingest");
    let before = h.probe().point_count();

    h.ingest(&fixtures).expect("second ingest");
    let after = h.probe().point_count();

    assert_eq!(
        before, after,
        "deterministic IDs + cache hits = no new points"
    );
    let s = h.status().expect("status");
    assert!(s.cached_rows > 0, "manifest reflects cache hits");
}

// ─── J3: Update flow ─────────────────────────────────────────────────────

#[test]
fn e2e_modifying_a_file_drops_orphaned_chunks_after_re_ingest() {
    let Some(h) = LibrarianHarness::fresh("update") else {
        eprintln!("skip: no Qdrant");
        return;
    };
    let fixtures = FixtureBuilder::new()
        .with_paragraphs("a.txt", 5)
        .write_to(h.workdir.path());

    h.ingest(&fixtures).expect("first ingest");
    assert_eq!(h.probe().point_count(), 5);

    // Edit: shrink to 2 paragraphs.
    std::fs::write(fixtures.join("a.txt"), "p0\n\np1").unwrap();
    h.ingest(&fixtures).expect("second ingest");

    assert_eq!(
        h.probe().point_count(),
        2,
        "no orphaned points from prior 5-chunk version"
    );
}

// ─── J4: Explicit removal ────────────────────────────────────────────────

#[test]
fn e2e_remove_drops_chunks_for_one_source_only() {
    let Some(h) = LibrarianHarness::fresh("remove") else {
        eprintln!("skip: no Qdrant");
        return;
    };
    let fixtures = FixtureBuilder::new()
        .with_paragraphs("a.txt", 3)
        .with_paragraphs("b.txt", 2)
        .write_to(h.workdir.path());
    h.ingest(&fixtures).expect("ingest");
    assert_eq!(h.probe().point_count(), 5);

    let source_a = fixtures.join("a.txt").display().to_string();
    h.remove(&source_a).expect("remove");

    assert_eq!(h.probe().point_count(), 2, "only b.txt's chunks remain");
}

// ─── J5: Snapshot/restore round-trip ─────────────────────────────────────

#[test]
fn e2e_snapshot_then_restore_recovers_collection_state() {
    let Some(h) = LibrarianHarness::fresh("snap") else {
        eprintln!("skip: no Qdrant");
        return;
    };
    let fixtures = FixtureBuilder::new()
        .with_paragraphs("a.txt", 4)
        .write_to(h.workdir.path());
    h.ingest(&fixtures).expect("ingest");
    assert_eq!(h.probe().point_count(), 4);

    let snap_id = h.snapshot().expect("snapshot");
    assert!(
        h.nas_dir().join(&snap_id.0).exists(),
        "snapshot file landed on NAS"
    );

    // Wipe: arrange teardown via direct API, not by removing through CLI
    // (which would invoke the runner pipeline).
    h.probe()
        .wipe_source(&fixtures.join("a.txt").display().to_string());
    assert_eq!(h.probe().point_count(), 0, "collection wiped");

    h.restore(&snap_id).expect("restore");
    assert_eq!(
        h.probe().point_count(),
        4,
        "snapshot restored prior 4 chunks"
    );
}

// ─── J6: Snapshot retention ──────────────────────────────────────────────

#[test]
fn e2e_snapshot_retention_prunes_old_files_on_nas() {
    let Some(h) = LibrarianHarness::fresh("retention") else {
        eprintln!("skip: no Qdrant");
        return;
    };

    // Override default retention=5 → 2 in the existing config.
    let cfg_path = h.config.clone();
    let mut body = std::fs::read_to_string(&cfg_path).unwrap();
    body = body.replace("retention = 5", "retention = 2");
    std::fs::write(&cfg_path, body).unwrap();

    let fixtures = FixtureBuilder::new()
        .with_file("a.txt", "x")
        .write_to(h.workdir.path());
    h.ingest(&fixtures).expect("seed");

    for _ in 0..4 {
        h.snapshot().expect("snapshot");
        // Qdrant timestamps snapshots per second — nudge to keep mtimes distinct.
        std::thread::sleep(std::time::Duration::from_millis(1100));
    }

    let on_nas = std::fs::read_dir(h.nas_dir()).unwrap().count();
    assert_eq!(on_nas, 2, "retention=2 leaves 2 snapshot files");
}

// ─── J7: Fleet lifecycle ─────────────────────────────────────────────────

#[test]
fn e2e_fleet_start_status_stop_walks_lifecycle() {
    let Some(h) = LibrarianHarness::fresh("fleet") else {
        eprintln!("skip: no Qdrant");
        return;
    };

    h.fleet_start("primary").expect("start");
    let entries = h.fleet_list().expect("list");
    let primary = entries
        .iter()
        .find(|e| e.name == "primary")
        .expect("primary present");
    assert_eq!(primary.status, "running");
    assert!(primary.pid.is_some());

    h.fleet_stop("primary").expect("stop");
    let entries_after = h.fleet_list().expect("list 2");
    let primary_after = entries_after
        .iter()
        .find(|e| e.name == "primary")
        .expect("still listed");
    assert_eq!(primary_after.status, "stopped");
}

// ─── J8: MCP search ──────────────────────────────────────────────────────

#[test]
fn e2e_mcp_search_returns_relevant_hits_for_ingested_text() {
    let Some(h) = LibrarianHarness::fresh("mcp") else {
        eprintln!("skip: no Qdrant");
        return;
    };
    let fixtures = FixtureBuilder::new()
        .with_file(
            "dragon.txt",
            "the dragon was huge\n\nthe knight was brave\n\nthe king was nervous",
        )
        .write_to(h.workdir.path());
    h.ingest(&fixtures).expect("ingest");

    let mut mcp = McpHarness::spawn(&h.config);

    let hits = mcp.search("dragon", 3);
    assert!(!hits.is_empty(), "MCP search returned at least one hit");

    let docs = mcp.list_documents();
    assert!(
        docs.iter().any(|s| s.contains("dragon.txt")),
        "list_documents reflects ingest"
    );
}
