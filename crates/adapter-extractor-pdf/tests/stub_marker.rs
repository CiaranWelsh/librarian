//! Issue 030: PdfExtractor invokes marker with configurable flags/device and an optional
//! durable output dir. Tested against a stub `marker_single` shell script that records its
//! argv + TORCH_DEVICE and writes fixture markdown — no real marker needed.

use adapter_extractor_pdf::{MarkerConfig, PdfExtractor};
use librarian_domain::{ContentType, Document, Extractor, SourceHash, SourceId};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Write the stub marker script into `dir` and return its path. The stub:
/// - records its argv to `<dir>/argv.txt` and `$TORCH_DEVICE` to `<dir>/device.txt`
/// - writes `# stub markdown for <stem>` to `<output_dir>/<stem>/<stem>.md`
fn write_stub(dir: &Path) -> PathBuf {
    let script = r##"#!/bin/sh
in="$1"; shift
out=""
prev=""
for a in "$@"; do
  [ "$prev" = "--output_dir" ] && out="$a"
  prev="$a"
done
stem=$(basename "$in" .pdf)
mkdir -p "$out/$stem"
echo "# stub markdown for $stem" > "$out/$stem/$stem.md"
{ printf '%s\n' "$in"; printf '%s\n' "$@"; } > "$(dirname "$0")/argv.txt"
echo "$TORCH_DEVICE" > "$(dirname "$0")/device.txt"
exit 0
"##;
    let path = dir.join("marker_single");
    std::fs::write(&path, script).unwrap();
    let mut perm = std::fs::metadata(&path).unwrap().permissions();
    perm.set_mode(0o755);
    std::fs::set_permissions(&path, perm).unwrap();
    path
}

fn doc(pdf: &str) -> Document {
    Document {
        source_id: SourceId(pdf.into()),
        source_hash: SourceHash("h".into()),
        content_type: ContentType::Book,
        path: PathBuf::from(pdf),
        work_id: None,
    }
}

#[test]
fn passes_batch_flags_and_device_to_marker() {
    let stub_dir = tempfile::tempdir().unwrap();
    let stub = write_stub(stub_dir.path());
    let cfg = MarkerConfig {
        device: Some("cuda".into()),
        recognition_batch_size: Some(1),
        detection_batch_size: Some(2),
        layout_batch_size: Some(3),
        table_rec_batch_size: Some(4),
        output_dir: None,
    };
    let e = PdfExtractor::new().with_marker_bin(&stub).with_config(cfg);

    let text = e.extract(&doc("/nonexistent/MyBook.pdf")).unwrap();
    assert_eq!(text.spans[0].text.trim(), "# stub markdown for MyBook");

    let argv = std::fs::read_to_string(stub_dir.path().join("argv.txt")).unwrap();
    for needle in [
        "--recognition_batch_size\n1",
        "--detection_batch_size\n2",
        "--layout_batch_size\n3",
        "--table_rec_batch_size\n4",
        "--disable_image_extraction",
    ] {
        assert!(argv.contains(needle), "argv missing {needle:?}:\n{argv}");
    }
    let device = std::fs::read_to_string(stub_dir.path().join("device.txt")).unwrap();
    assert_eq!(device.trim(), "cuda");
}

#[test]
fn durable_output_dir_persists_markdown_after_extract() {
    let stub_dir = tempfile::tempdir().unwrap();
    let stub = write_stub(stub_dir.path());
    let out = tempfile::tempdir().unwrap();
    let cfg = MarkerConfig {
        output_dir: Some(out.path().to_path_buf()),
        ..MarkerConfig::default()
    };
    let e = PdfExtractor::new().with_marker_bin(&stub).with_config(cfg);

    let text = e.extract(&doc("/nonexistent/Durable.pdf")).unwrap();
    assert_eq!(text.spans[0].text.trim(), "# stub markdown for Durable");
    // the markdown survives outside any tempdir — this is the issue-029 durability fix
    let kept = out.path().join("Durable").join("Durable.md");
    assert!(kept.exists(), "markdown not persisted at {kept:?}");
}

#[test]
fn pre_existing_durable_output_is_reused_without_running_marker() {
    let stub_dir = tempfile::tempdir().unwrap();
    let stub = write_stub(stub_dir.path());
    let out = tempfile::tempdir().unwrap();
    let stem_dir = out.path().join("Cached");
    std::fs::create_dir_all(&stem_dir).unwrap();
    std::fs::write(stem_dir.join("Cached.md"), "# pre-existing extraction\n").unwrap();

    let cfg = MarkerConfig {
        output_dir: Some(out.path().to_path_buf()),
        ..MarkerConfig::default()
    };
    let e = PdfExtractor::new().with_marker_bin(&stub).with_config(cfg);

    let text = e.extract(&doc("/nonexistent/Cached.pdf")).unwrap();
    assert_eq!(text.spans[0].text.trim(), "# pre-existing extraction");
    // marker must NOT have been invoked — the stub would have left argv.txt
    assert!(
        !stub_dir.path().join("argv.txt").exists(),
        "marker was invoked despite existing durable output"
    );
}

#[test]
fn default_config_adds_no_batch_flags() {
    let stub_dir = tempfile::tempdir().unwrap();
    let stub = write_stub(stub_dir.path());
    let e = PdfExtractor::new().with_marker_bin(&stub); // no config at all

    e.extract(&doc("/nonexistent/Plain.pdf")).unwrap();
    let argv = std::fs::read_to_string(stub_dir.path().join("argv.txt")).unwrap();
    assert!(
        !argv.contains("batch_size"),
        "unexpected batch flags:\n{argv}"
    );
    let device = std::fs::read_to_string(stub_dir.path().join("device.txt")).unwrap();
    assert_eq!(device.trim(), "", "TORCH_DEVICE should be unset by default");
}
