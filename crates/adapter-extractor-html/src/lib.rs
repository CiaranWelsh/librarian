//! HTML extractor — pandoc → cleaner → markdown.

mod error;
mod extractor;

pub use error::HtmlExtractError;
pub use extractor::HtmlExtractor;

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::{AdapterIdentity, ContentType, Document, Extractor, SourceHash, SourceId};
    use std::path::PathBuf;

    fn doc(path: &str) -> Document {
        Document {
            source_id: SourceId("s".into()),
            source_hash: SourceHash("h".into()),
            content_type: ContentType::Book,
            path: PathBuf::from(path),
            work_id: None,
        }
    }

    #[test]
    fn adapter_identity_stable() {
        let e = HtmlExtractor::new();
        assert_eq!(e.name(), "extractor-html");
        assert_eq!(e.version().0, "0.1.0-pandoc-gfm");
    }

    // Non-HTML extension is rejected before pandoc is ever invoked.
    // Source: issue 026 — extractor accepts only .html/.htm.
    #[test]
    fn rejects_non_html_extension() {
        let r = HtmlExtractor::new().extract(&doc("notes.txt"));
        assert!(matches!(r, Err(HtmlExtractError::UnsupportedExtension(_))));
    }

    fn pandoc_available() -> bool {
        std::process::Command::new(
            std::env::var("PANDOC_BIN").unwrap_or_else(|_| "pandoc".into()),
        )
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    }

    // Real extraction: HTML → clean GFM markdown (heading + inline code recovered,
    // tags stripped). Self-gates when pandoc is absent so it never breaks a
    // pandoc-less CI. Source: issue 026 acceptance criterion.
    #[test]
    fn extracts_html_to_clean_markdown() {
        if !pandoc_available() {
            eprintln!("skipping: pandoc not available");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("page.html");
        std::fs::write(
            &path,
            "<h1>Title</h1>\n<p>Hello <code>world</code> here.</p>",
        )
        .unwrap();

        let d = Document {
            source_id: SourceId("s".into()),
            source_hash: SourceHash("h".into()),
            content_type: ContentType::Book,
            path,
            work_id: None,
        };
        let out = HtmlExtractor::new().extract(&d).unwrap();
        let text = &out.spans[0].text;
        assert!(text.contains("# Title"), "heading not converted: {text:?}");
        assert!(text.contains("Hello"), "body text missing: {text:?}");
        assert!(text.contains("`world`"), "inline code not preserved: {text:?}");
        assert!(!text.contains("<p>") && !text.contains("<h1>"), "raw tags leaked: {text:?}");
    }
}
