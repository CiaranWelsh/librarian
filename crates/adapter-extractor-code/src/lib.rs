//! Code-file extractor: reads the file as UTF-8 and emits a single Code span.
//! Walking + filtering happens in the CLI; `should_include` is the helper.

use librarian_domain::{
    AdapterIdentity, ConfigHash, Document, ExtractedText, Extractor, SpanKind, StageVersion,
    TextSpan,
};
use std::path::{Component, Path};

#[derive(Default)]
pub struct CodeExtractor;

impl CodeExtractor {
    pub fn new() -> Self { Self }
}

#[derive(Debug, thiserror::Error)]
pub enum CodeExtractError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("not utf-8: {0}")]
    Encoding(String),
}

impl AdapterIdentity for CodeExtractor {
    fn name(&self) -> &str { "extractor-code" }
    fn version(&self) -> StageVersion { StageVersion("0.1.0".into()) }
    fn config_hash(&self) -> ConfigHash { ConfigHash("default".into()) }
}

impl Extractor for CodeExtractor {
    type Error = CodeExtractError;
    fn extract(&self, doc: &Document) -> Result<ExtractedText, Self::Error> {
        let bytes = std::fs::read(&doc.path)?;
        let text = String::from_utf8(bytes).map_err(|e| CodeExtractError::Encoding(e.to_string()))?;
        let len = text.len();
        Ok(ExtractedText { spans: vec![TextSpan {
            kind: SpanKind::Code,
            text,
            page: None,
            byte_range: 0..len,
        }]})
    }
}

/// Default skip list for vendored / output / VCS dirs.
pub const DEFAULT_SKIP_DIRS: &[&str] = &[
    ".git", "target", "node_modules", "vendor", "dist", "build", ".venv", "__pycache__", ".tox",
];

/// Default supported source extensions. Anything outside is treated as binary
/// or otherwise out-of-scope and skipped.
pub const DEFAULT_INCLUDE_EXTS: &[&str] = &[
    "rs", "py", "js", "jsx", "ts", "tsx", "go", "java", "kt", "swift",
    "c", "h", "cc", "cpp", "hpp", "cs", "rb", "sh", "bash", "zsh",
    "toml", "yaml", "yml", "json", "md", "txt",
];

/// Filter: should the CLI's directory walk pass `path` through to the extractor?
/// Pure function — no I/O.
pub fn should_include(path: &Path, skip_dirs: &[&str], include_exts: &[&str]) -> bool {
    for c in path.components() {
        if let Component::Normal(s) = c {
            if let Some(name) = s.to_str() {
                if skip_dirs.iter().any(|d| name == *d) { return false; }
            }
        }
    }
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    include_exts.iter().any(|e| e.eq_ignore_ascii_case(ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::{ContentType, SourceHash, SourceId};

    fn doc(p: &Path) -> Document {
        Document {
            source_id: SourceId(p.display().to_string()),
            source_hash: SourceHash("h".into()),
            content_type: ContentType::Code,
            path: p.to_path_buf(),
            work_id: None,
        }
    }

    #[test]
    fn reads_utf8_source_file_as_single_code_span() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("hello.rs");
        std::fs::write(&p, "fn main() {}\n").unwrap();
        let r = CodeExtractor.extract(&doc(&p)).unwrap();
        assert_eq!(r.spans.len(), 1);
        assert!(matches!(r.spans[0].kind, SpanKind::Code));
        assert_eq!(r.spans[0].text, "fn main() {}\n");
    }

    #[test]
    fn missing_file_yields_io_error() {
        let r = CodeExtractor.extract(&doc(Path::new("/no/such/file.rs")));
        assert!(matches!(r, Err(CodeExtractError::Io(_))));
    }

    #[test]
    fn non_utf8_is_classified_as_encoding_error() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("binary.rs");
        std::fs::write(&p, [0xff, 0xfe, 0x00, 0x00]).unwrap();
        let r = CodeExtractor.extract(&doc(&p));
        assert!(matches!(r, Err(CodeExtractError::Encoding(_))));
    }

    #[test]
    fn skip_dirs_filter_removes_target_node_modules_git() {
        for skipped in &["target", ".git", "node_modules", "vendor"] {
            let p: std::path::PathBuf = format!("/repo/{skipped}/sub/foo.rs").into();
            assert!(!should_include(&p, DEFAULT_SKIP_DIRS, DEFAULT_INCLUDE_EXTS),
                    "{skipped} should be skipped");
        }
    }

    #[test]
    fn binary_extension_is_skipped() {
        let p = Path::new("/repo/src/binary.dat");
        assert!(!should_include(p, DEFAULT_SKIP_DIRS, DEFAULT_INCLUDE_EXTS));
    }

    #[test]
    fn known_source_extensions_are_included() {
        for ext in &["rs", "py", "ts", "go"] {
            let p: std::path::PathBuf = format!("/repo/src/file.{ext}").into();
            assert!(should_include(&p, DEFAULT_SKIP_DIRS, DEFAULT_INCLUDE_EXTS),
                    ".{ext} should be included");
        }
    }
}
