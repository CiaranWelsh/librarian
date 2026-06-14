use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum Kind {
    Pdf,
    Ebook,
    Html,
    Code,
    Markdown,
}

#[derive(Debug)]
pub struct AddPlan {
    pub kind: Kind,
    pub slug: String,
    pub raw_path: PathBuf,
    pub ingest_path: PathBuf,
    pub source_id_prefix: String,
    pub config_path: PathBuf,
}

impl AddPlan {
    /// Resolve just the kind and per-collection config path from the source name
    /// and `config_root`. Neither depends on `corpus_root`, so the caller can load
    /// the config (and thus learn `corpus_root`) before the full `derive`. This
    /// avoids deriving twice against a throwaway corpus root.
    pub fn config_path_for(
        src: &Path,
        collection: &str,
        config_root: &Path,
    ) -> Result<(Kind, PathBuf), String> {
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        let kind = classify_ext(&ext)?;
        let config_path = config_root.join(collection).join(config_file_for(&kind));
        Ok((kind, config_path))
    }

    pub fn derive(
        src: &Path,
        collection: &str,
        shelf: Option<&str>,
        slug: Option<&str>,
        corpus_root: &Path,
        config_root: &Path,
    ) -> Result<AddPlan, String> {
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();

        let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or_default();

        let base_slug = match slug {
            Some(s) => slugify(s),
            None => slugify(stem),
        };
        if base_slug.is_empty() {
            return Err(format!(
                "could not derive a slug from {:?} (only ASCII letters and digits are kept; try --slug)",
                src
            ));
        }

        let full_slug = match shelf {
            Some(s) => {
                let shelf_slug = slugify(s);
                if shelf_slug.is_empty() {
                    return Err(format!("shelf {:?} produced an empty slug", s));
                }
                format!("{}-{}", shelf_slug, base_slug)
            }
            None => base_slug,
        };

        let kind = classify_ext(&ext)?;

        let (raw_path, ingest_path, source_id_prefix) = match &kind {
            Kind::Pdf => {
                let raw = corpus_root
                    .join(collection)
                    .join("pdf")
                    .join(format!("{}.pdf", full_slug));
                let ingest = corpus_root
                    .join(collection)
                    .join("markdown")
                    .join(&full_slug);
                let prefix = format!("{}/markdown/{}", collection, full_slug);
                (raw, ingest, prefix)
            }
            Kind::Ebook => {
                let file = format!("{}.{}", full_slug, ext);
                let raw = corpus_root.join(collection).join("ebook").join(&file);
                let prefix = format!("{}/ebook/{}", collection, file);
                (raw.clone(), raw, prefix)
            }
            Kind::Html => {
                let file = format!("{}.{}", full_slug, ext);
                let raw = corpus_root.join(collection).join("html").join(&file);
                let prefix = format!("{}/html/{}", collection, file);
                (raw.clone(), raw, prefix)
            }
            Kind::Code => {
                let file = format!("{}.{}", full_slug, ext);
                let raw = corpus_root.join(collection).join("code").join(&file);
                let prefix = format!("{}/code/{}", collection, file);
                (raw.clone(), raw, prefix)
            }
            Kind::Markdown => {
                let raw = corpus_root
                    .join(collection)
                    .join("markdown")
                    .join(&full_slug);
                let prefix = format!("{}/markdown/{}", collection, full_slug);
                (raw.clone(), raw, prefix)
            }
        };

        let config_path = config_root.join(collection).join(config_file_for(&kind));

        Ok(AddPlan {
            kind,
            slug: full_slug,
            raw_path,
            ingest_path,
            source_id_prefix,
            config_path,
        })
    }
}

/// Per-collection config filename for a resource kind. PDF previews through
/// `pdf.toml`, but the decoupled PDF flow ingests the durable markdown through
/// `text.toml` (the commit path resolves that itself).
fn config_file_for(kind: &Kind) -> &'static str {
    match kind {
        Kind::Pdf => "pdf.toml",
        Kind::Ebook => "ebook.toml",
        Kind::Html => "html.toml",
        Kind::Code => "code.toml",
        Kind::Markdown => "text.toml",
    }
}

fn classify_ext(ext: &str) -> Result<Kind, String> {
    match ext {
        "pdf" => Ok(Kind::Pdf),
        "epub" | "mobi" | "azw3" | "azw" => Ok(Kind::Ebook),
        "html" | "htm" => Ok(Kind::Html),
        "md" => Ok(Kind::Markdown),
        "rs" | "py" | "c" | "cpp" | "h" | "hpp" | "go" | "js" | "ts" | "java" => Ok(Kind::Code),
        other => Err(format!("unsupported file type: .{}", other)),
    }
}

fn slugify(s: &str) -> String {
    let lower = s.to_ascii_lowercase();
    let mut slug = String::with_capacity(lower.len());
    let mut last_was_sep = false;
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_sep = false;
        } else if !last_was_sep {
            slug.push('-');
            last_was_sep = true;
        }
    }
    slug.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS: &str = "/corpus";
    const CFG: &str = "/cfg";

    fn corpus() -> &'static Path {
        Path::new(CORPUS)
    }
    fn cfg() -> &'static Path {
        Path::new(CFG)
    }

    #[test]
    fn pdf_basic() {
        let plan = AddPlan::derive(
            Path::new("/downloads/Programming Rust.pdf"),
            "software",
            None,
            None,
            corpus(),
            cfg(),
        )
        .unwrap();

        assert_eq!(plan.kind, Kind::Pdf);
        assert_eq!(plan.slug, "programming-rust");
        assert_eq!(
            plan.raw_path,
            PathBuf::from("/corpus/software/pdf/programming-rust.pdf")
        );
        assert_eq!(
            plan.ingest_path,
            PathBuf::from("/corpus/software/markdown/programming-rust")
        );
        assert_eq!(plan.source_id_prefix, "software/markdown/programming-rust");
        assert_eq!(plan.config_path, PathBuf::from("/cfg/software/pdf.toml"));
    }

    #[test]
    fn epub_in_place() {
        let plan = AddPlan::derive(
            Path::new("/downloads/Async Rust.epub"),
            "software",
            None,
            None,
            corpus(),
            cfg(),
        )
        .unwrap();

        assert_eq!(plan.kind, Kind::Ebook);
        assert_eq!(plan.slug, "async-rust");
        assert_eq!(
            plan.raw_path,
            PathBuf::from("/corpus/software/ebook/async-rust.epub")
        );
        assert_eq!(plan.ingest_path, plan.raw_path);
        assert_eq!(plan.source_id_prefix, "software/ebook/async-rust.epub");
        assert_eq!(plan.config_path, PathBuf::from("/cfg/software/ebook.toml"));
    }

    #[test]
    fn markdown_with_shelf_and_custom_slug() {
        let plan = AddPlan::derive(
            Path::new("/downloads/foo.md"),
            "software",
            Some("architecture"),
            Some("my-book"),
            corpus(),
            cfg(),
        )
        .unwrap();

        assert_eq!(plan.slug, "architecture-my-book");
        assert_eq!(
            plan.ingest_path,
            PathBuf::from("/corpus/software/markdown/architecture-my-book")
        );
        assert_eq!(plan.raw_path, plan.ingest_path);
        assert_eq!(
            plan.source_id_prefix,
            "software/markdown/architecture-my-book"
        );
        assert_eq!(plan.config_path, PathBuf::from("/cfg/software/text.toml"));
    }

    #[test]
    fn unsupported_extension_errors() {
        let result = AddPlan::derive(
            Path::new("/downloads/photo.png"),
            "software",
            None,
            None,
            corpus(),
            cfg(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsupported file type"));
    }

    #[test]
    fn html_in_place() {
        let plan = AddPlan::derive(
            Path::new("/downloads/the-rust-reference.html"),
            "software",
            None,
            None,
            corpus(),
            cfg(),
        )
        .unwrap();

        assert_eq!(plan.kind, Kind::Html);
        assert_eq!(
            plan.raw_path,
            PathBuf::from("/corpus/software/html/the-rust-reference.html")
        );
        assert_eq!(plan.ingest_path, plan.raw_path);
        assert_eq!(
            plan.source_id_prefix,
            "software/html/the-rust-reference.html"
        );
        assert_eq!(plan.config_path, PathBuf::from("/cfg/software/html.toml"));
    }

    #[test]
    fn code_in_place() {
        let plan = AddPlan::derive(
            Path::new("/downloads/main.rs"),
            "software",
            None,
            None,
            corpus(),
            cfg(),
        )
        .unwrap();

        assert_eq!(plan.kind, Kind::Code);
        assert_eq!(
            plan.raw_path,
            PathBuf::from("/corpus/software/code/main.rs")
        );
        assert_eq!(plan.ingest_path, plan.raw_path);
        assert_eq!(plan.source_id_prefix, "software/code/main.rs");
        assert_eq!(plan.config_path, PathBuf::from("/cfg/software/code.toml"));
    }
}
