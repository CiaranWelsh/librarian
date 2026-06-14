//! E2E for `librarian add --commit` (L-069): place the resource under the corpus
//! and ingest it. One focused round trip on the in-place markdown flow — the
//! decoupled PDF flow shares the same place-and-ingest tail, so a markdown
//! fixture exercises the write side without a Marker dependency.
//!
//! Self-skipping like `cli_integration.rs`: probe `status` first and bail with a
//! printed note if the test Qdrant is unreachable.

use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;
use std::path::Path;
use tempfile::TempDir;

fn qdrant_url() -> String {
    std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6533".into())
}

fn unique_collection() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("librarian-cli-addcommit-{nanos}")
}

/// Write a per-collection `text.toml` under `<config_root>/<collection>/`, the
/// layout `add` resolves the config from. `corpus_root` is a temp dir so placed
/// files land under it (ADR-0007).
fn write_text_config(config_root: &Path, collection: &str, corpus_root: &Path, dir: &Path) {
    let col_dir = config_root.join(collection);
    std::fs::create_dir_all(&col_dir).unwrap();
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
        corpus_root = corpus_root.display(),
    );
    std::fs::write(col_dir.join("text.toml"), body).unwrap();
}

#[test]
fn add_commit_places_and_ingests_markdown() {
    let collection = unique_collection();
    let dir = TempDir::new().unwrap();
    let config_root = dir.path().join("config");
    let corpus_root = dir.path().join("corpus");
    std::fs::create_dir_all(&corpus_root).unwrap();
    write_text_config(&config_root, &collection, &corpus_root, dir.path());

    let cfg = config_root.join(&collection).join("text.toml");

    // Probe Qdrant via `status`; skip the rest if it is unreachable.
    let probe = Command::cargo_bin("librarian")
        .unwrap()
        .args(["status", "--config"])
        .arg(&cfg)
        .assert();
    if !probe.get_output().status.success() {
        eprintln!("skip: Qdrant not reachable at {}", qdrant_url());
        return;
    }

    // A small markdown source outside the corpus; `add` should place it under it.
    let src = dir.path().join("hexagons.md");
    std::fs::write(
        &src,
        "# Hexagonal Architecture\n\nPorts and adapters keep the core pure.\n",
    )
    .unwrap();

    Command::cargo_bin("librarian")
        .unwrap()
        .env("LIBRARIAN_CONFIG_ROOT", &config_root)
        .args(["add"])
        .arg(&src)
        .args(["--to", &collection, "--commit"])
        .assert()
        .success()
        .stdout(contains(format!(
            "committed {collection}/markdown/hexagons"
        )));

    // The source was placed at <corpus>/<col>/markdown/hexagons/hexagons.md.
    let placed = corpus_root
        .join(&collection)
        .join("markdown")
        .join("hexagons")
        .join("hexagons.md");
    assert!(
        placed.exists(),
        "expected placed markdown at {}",
        placed.display()
    );

    // Ingest indexed points into the collection.
    Command::cargo_bin("librarian")
        .unwrap()
        .args(["status", "--config"])
        .arg(&cfg)
        .assert()
        .success()
        .stdout(contains("points: ").and(predicates::str::contains("points: 0").not()));
}

/// Adding the same resource twice is a no-op the second time: the first commit
/// ingests, the second skips (printing `skip:`) and leaves the point count
/// unchanged, and a third run with `--force` re-ingests (no `skip:`). Guards the
/// idempotency pre-check and that `--force` overrides it.
#[test]
fn add_commit_is_idempotent_and_force_overrides() {
    let collection = unique_collection();
    let dir = TempDir::new().unwrap();
    let config_root = dir.path().join("config");
    let corpus_root = dir.path().join("corpus");
    std::fs::create_dir_all(&corpus_root).unwrap();
    write_text_config(&config_root, &collection, &corpus_root, dir.path());

    let cfg = config_root.join(&collection).join("text.toml");

    let probe = Command::cargo_bin("librarian")
        .unwrap()
        .args(["status", "--config"])
        .arg(&cfg)
        .assert();
    if !probe.get_output().status.success() {
        eprintln!("skip: Qdrant not reachable at {}", qdrant_url());
        return;
    }

    let src = dir.path().join("idempotent.md");
    std::fs::write(&src, "# Idempotency\n\nAdding twice should be a no-op.\n").unwrap();

    let add = |force: bool| {
        let mut c = Command::cargo_bin("librarian").unwrap();
        c.env("LIBRARIAN_CONFIG_ROOT", &config_root)
            .args(["add"])
            .arg(&src)
            .args(["--to", &collection, "--commit"]);
        if force {
            c.arg("--force");
        }
        c
    };

    // First commit ingests; no skip.
    add(false)
        .assert()
        .success()
        .stdout(contains("skip:").not());

    let points_after_first = collection_points(&cfg);

    // Second commit skips and leaves the count unchanged.
    add(false)
        .assert()
        .success()
        .stdout(contains("skip:").and(contains("already present")));
    assert_eq!(
        collection_points(&cfg),
        points_after_first,
        "skip should not change the point count"
    );

    // Third commit with --force re-ingests (overwrites in place); no skip.
    add(true).assert().success().stdout(contains("skip:").not());
}

/// After a commit, `add --undo <source_id>` removes the points and the placed
/// `.md`. Source_id is the markdown file id `<col>/markdown/<slug>/<slug>.md`.
#[test]
fn add_undo_removes_points_and_placed_file() {
    let collection = unique_collection();
    let dir = TempDir::new().unwrap();
    let config_root = dir.path().join("config");
    let corpus_root = dir.path().join("corpus");
    std::fs::create_dir_all(&corpus_root).unwrap();
    write_text_config(&config_root, &collection, &corpus_root, dir.path());

    let cfg = config_root.join(&collection).join("text.toml");

    let probe = Command::cargo_bin("librarian")
        .unwrap()
        .args(["status", "--config"])
        .arg(&cfg)
        .assert();
    if !probe.get_output().status.success() {
        eprintln!("skip: Qdrant not reachable at {}", qdrant_url());
        return;
    }

    let src = dir.path().join("undo-me.md");
    std::fs::write(&src, "# Undo\n\nThis add should be reversible.\n").unwrap();

    Command::cargo_bin("librarian")
        .unwrap()
        .env("LIBRARIAN_CONFIG_ROOT", &config_root)
        .args(["add"])
        .arg(&src)
        .args(["--to", &collection, "--commit"])
        .assert()
        .success();

    let placed = corpus_root
        .join(&collection)
        .join("markdown")
        .join("undo-me")
        .join("undo-me.md");
    assert!(placed.exists(), "expected placed markdown before undo");

    let source_id = format!("{collection}/markdown/undo-me/undo-me.md");
    Command::cargo_bin("librarian")
        .unwrap()
        .env("LIBRARIAN_CONFIG_ROOT", &config_root)
        .args(["add", "--undo", &source_id, "--to", &collection])
        .assert()
        .success()
        .stdout(contains(format!("undone {source_id}")));

    assert!(
        !placed.exists(),
        "placed markdown should be gone after undo"
    );
    Command::cargo_bin("librarian")
        .unwrap()
        .args(["status", "--config"])
        .arg(&cfg)
        .assert()
        .success()
        .stdout(contains("points: 0"));
}

/// Parse the `points: N` line from `status --config` into a count.
fn collection_points(cfg: &Path) -> u64 {
    let out = Command::cargo_bin("librarian")
        .unwrap()
        .args(["status", "--config"])
        .arg(cfg)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix("points: "))
        .and_then(|n| n.trim().parse().ok())
        .unwrap_or_else(|| panic!("no `points:` line in status output:\n{stdout}"))
}
