//! Directory-walk filter used by the CLI to decide which files to feed to the
//! extractor. Pure functions, no I/O.

use std::path::{Component, Path};

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

/// Should the CLI's directory walk pass `path` through to the extractor?
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
