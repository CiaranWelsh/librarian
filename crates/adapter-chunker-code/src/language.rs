use std::path::Path;

/// Map a file extension to a language tag for `CodeMeta.language`.
/// Pure function; no I/O.
pub fn detect_language(path: &Path) -> Option<String> {
    let ext = path.extension().and_then(|s| s.to_str())?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" | "jsx" => "javascript",
        "ts" | "tsx" => "typescript",
        "go" => "go",
        "java" => "java",
        "kt" => "kotlin",
        "swift" => "swift",
        "c" | "h" => "c",
        "cc" | "cpp" | "hpp" => "cpp",
        "sip" => "sip",
        "cs" => "csharp",
        "rb" => "ruby",
        "sh" | "bash" | "zsh" => "shell",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "md" => "markdown",
        _ => return None,
    }.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_language_from_extension() {
        assert_eq!(detect_language(Path::new("foo.rs")).as_deref(), Some("rust"));
        assert_eq!(detect_language(Path::new("foo.py")).as_deref(), Some("python"));
        assert_eq!(detect_language(Path::new("foo.unknown")), None);
    }
}
