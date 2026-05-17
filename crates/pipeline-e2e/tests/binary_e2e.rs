//! E2E: spawn the actual binary, point it at a fixture, assert stdout.

use assert_cmd::Command;
use predicates::str::contains;

fn fixture_path() -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.push("tests/fixtures/sample.txt");
    p
}

#[test]
fn ingests_three_paragraph_fixture() {
    Command::cargo_bin("pipeline-e2e")
        .unwrap()
        .arg(fixture_path())
        .assert()
        .success()
        .stdout(contains("indexed 3 chunks"));
}

#[test]
fn missing_argument_fails() {
    Command::cargo_bin("pipeline-e2e")
        .unwrap()
        .assert()
        .failure();
}
