//! PDF text extractor backed by `marker_single`.
//!
//! Marker is a vision-language model that converts PDFs to clean markdown —
//! handling multi-column layout, math (as LaTeX), tables, and figure captions
//! that bytes-level parsers fail on. We shell out to the CLI rather than
//! linking the Python library directly; one subprocess per document.
//!
//! Output: one `TextSpan` containing the full markdown body. The downstream
//! chunker splits it (recursive by default since issue 027).
//!
//! Configuration: the marker binary is found via (in order):
//! - the path given to `with_marker_bin`,
//! - the `MARKER_BIN` env var,
//! - `marker_single` in `$PATH`.
//!
//! Issue 030: the invocation is configurable via [`MarkerConfig`] — batch sizes
//! (as CLI flags; marker ignores the equivalent env vars), TORCH_DEVICE, and an
//! optional durable output dir so the markdown survives outside the cache.

use librarian_domain::{
    AdapterIdentity, ConfigHash, Document, ExtractedText, Extractor, SpanKind, StageVersion,
    TextSpan,
};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::PdfExtractError;

/// Knobs for the marker subprocess (issue 030). All optional; `Default` reproduces
/// the original hardcoded invocation. Batch sizes must be passed as CLI flags —
/// marker ignores the corresponding env vars (found the hard way on an 8GB GPU,
/// where only `--recognition_batch_size 1` lets the models fit).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MarkerConfig {
    /// Sets `TORCH_DEVICE` for the subprocess ("cuda" / "cpu"). Unset = marker's default.
    pub device: Option<String>,
    pub recognition_batch_size: Option<u32>,
    pub detection_batch_size: Option<u32>,
    pub layout_batch_size: Option<u32>,
    pub table_rec_batch_size: Option<u32>,
    /// Durable output directory. When set, marker writes `<dir>/<stem>/<stem>.md`
    /// and a pre-existing output is reused without invoking marker at all — the
    /// markdown becomes a first-class artifact (issue 029) instead of a tempdir
    /// casualty, and re-ingests resume for free. Tempdir when unset.
    pub output_dir: Option<PathBuf>,
}

pub struct PdfExtractor {
    marker_bin: PathBuf,
    config: MarkerConfig,
}

impl Default for PdfExtractor {
    fn default() -> Self {
        Self {
            marker_bin: std::env::var("MARKER_BIN")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("marker_single")),
            config: MarkerConfig::default(),
        }
    }
}

impl PdfExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_marker_bin(mut self, p: impl Into<PathBuf>) -> Self {
        self.marker_bin = p.into();
        self
    }

    pub fn with_config(mut self, config: MarkerConfig) -> Self {
        self.config = config;
        self
    }
}

/// Assemble the marker argv. Pure so the flag set is unit-testable.
fn marker_args(cfg: &MarkerConfig, pdf: &Path, out_dir: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![
        pdf.as_os_str().to_os_string(),
        "--output_dir".into(),
        out_dir.as_os_str().to_os_string(),
        "--disable_image_extraction".into(),
    ];
    let flags = [
        ("--recognition_batch_size", cfg.recognition_batch_size),
        ("--detection_batch_size", cfg.detection_batch_size),
        ("--layout_batch_size", cfg.layout_batch_size),
        ("--table_rec_batch_size", cfg.table_rec_batch_size),
    ];
    for (flag, v) in flags {
        if let Some(n) = v {
            args.push(flag.into());
            args.push(n.to_string().into());
        }
    }
    args
}

impl AdapterIdentity for PdfExtractor {
    fn name(&self) -> &str {
        "extractor-pdf"
    }
    /// Bumped from 0.1.0 (lopdf) — invalidates any prior cache entries on
    /// purpose so the corpus gets re-extracted with marker.
    fn version(&self) -> StageVersion {
        StageVersion("0.2.0-marker".into())
    }
    /// Deliberately does NOT fold `MarkerConfig` in: device and batch sizes are
    /// performance knobs that don't change the extracted content, and `output_dir`
    /// is just a location. Folding them would spuriously re-extract the whole
    /// corpus on a tuning change — the exact cost class issue 029 exists to avoid.
    fn config_hash(&self) -> ConfigHash {
        ConfigHash("marker-default".into())
    }
}

impl Extractor for PdfExtractor {
    type Error = PdfExtractError;

    fn extract(&self, doc: &Document) -> Result<ExtractedText, Self::Error> {
        let stem = doc
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| PdfExtractError::Marker("non-UTF-8 pdf filename".into()))?;

        // Durable mode writes where configured; otherwise a tempdir that must
        // outlive the read below.
        let mut _tmp_guard: Option<tempfile::TempDir> = None;
        let out_dir: PathBuf = match &self.config.output_dir {
            Some(d) => {
                std::fs::create_dir_all(d).map_err(PdfExtractError::Io)?;
                d.clone()
            }
            None => {
                let t = tempfile::tempdir().map_err(PdfExtractError::Io)?;
                let p = t.path().to_path_buf();
                _tmp_guard = Some(t);
                p
            }
        };

        // marker writes <out_dir>/<stem>/<stem>.md
        let md_path = out_dir.join(stem).join(format!("{stem}.md"));

        // In durable mode a previous extraction is reused — marker is never the
        // hot path twice for the same file (issue 029).
        let reuse = self.config.output_dir.is_some() && md_path.exists();
        if !reuse {
            let mut cmd = Command::new(&self.marker_bin);
            cmd.args(marker_args(&self.config, &doc.path, &out_dir));
            if let Some(dev) = &self.config.device {
                cmd.env("TORCH_DEVICE", dev);
            }
            let output = cmd.output().map_err(PdfExtractError::Io)?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Tail of stderr — marker logs are noisy on the way down.
                let tail: String = stderr
                    .lines()
                    .rev()
                    .take(8)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
                    .join(" | ");
                return Err(PdfExtractError::Marker(format!(
                    "exit {:?}: {}",
                    output.status.code(),
                    tail
                )));
            }
        }

        let body = std::fs::read_to_string(&md_path).map_err(PdfExtractError::Io)?;
        if body.trim().is_empty() {
            return Err(PdfExtractError::Marker("empty markdown output".into()));
        }

        let len = body.len();
        Ok(ExtractedText {
            spans: vec![TextSpan {
                kind: SpanKind::Paragraph,
                text: body,
                page: None,
                byte_range: 0..len,
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn marker_args_defaults_are_minimal() {
        let args = s(&marker_args(
            &MarkerConfig::default(),
            Path::new("/x/Book.pdf"),
            Path::new("/out"),
        ));
        assert_eq!(
            args,
            vec![
                "/x/Book.pdf",
                "--output_dir",
                "/out",
                "--disable_image_extraction"
            ]
        );
    }

    #[test]
    fn marker_args_full_config_emits_all_flags() {
        let cfg = MarkerConfig {
            device: Some("cuda".into()), // device is env, not argv
            recognition_batch_size: Some(1),
            detection_batch_size: Some(2),
            layout_batch_size: Some(3),
            table_rec_batch_size: Some(4),
            output_dir: None,
        };
        let args = s(&marker_args(&cfg, Path::new("/x/B.pdf"), Path::new("/o")));
        for pair in [
            ["--recognition_batch_size", "1"],
            ["--detection_batch_size", "2"],
            ["--layout_batch_size", "3"],
            ["--table_rec_batch_size", "4"],
        ] {
            let i = args.iter().position(|a| a == pair[0]).expect(pair[0]);
            assert_eq!(args[i + 1], pair[1]);
        }
        assert!(!args.iter().any(|a| a == "cuda"), "device must not be argv");
    }

    #[test]
    fn config_hash_ignores_marker_config() {
        // Perf knobs don't change output content; the extract cache must not
        // invalidate on a tuning change (see AdapterIdentity::config_hash doc).
        let tuned = PdfExtractor::new().with_config(MarkerConfig {
            recognition_batch_size: Some(1),
            device: Some("cpu".into()),
            ..MarkerConfig::default()
        });
        assert_eq!(PdfExtractor::new().config_hash(), tuned.config_hash());
    }
}
