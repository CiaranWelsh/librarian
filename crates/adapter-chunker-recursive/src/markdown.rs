//! Markdown-header pass + breadcrumb pipeline (issue 027). Port of LangChain's
//! `MarkdownHeaderTextSplitter` (markdown.py: header-stack state machine, code-fence aware,
//! `strip_headers=True`, `aggregate_lines_to_chunks`) composed with the recursive splitter,
//! reproducing `experiments/chunking/sweep.py::chunk_rcts_md`.
//!
//! Each output chunk is `Book > Chapter > h1 > h2 > h3\n\n<recursive chunk of the section>`.

use crate::recursive::{split_text, DEFAULT_SEPARATORS};

/// Headers we split on, ordered longest-separator-first (LangChain sorts by length desc so
/// `###` is tested before `#`).
const HEADERS: &[(&str, &str)] = &[("###", "h3"), ("##", "h2"), ("#", "h1")];

struct Section {
    /// Active header path as ordered (name, text) pairs — the breadcrumb tail.
    meta: Vec<(String, String)>,
    body: String,
}

/// Python `str.isprintable()` approximation: keep visible characters and the space, drop
/// control/other whitespace (tabs etc.).
fn is_printable(c: char) -> bool {
    c == ' ' || (!c.is_whitespace() && !c.is_control())
}

/// Insertion-ordered upsert, mirroring `initial_metadata[name] = data` on a Python dict.
fn upsert(meta: &mut Vec<(String, String)>, name: &str, data: &str) {
    if let Some(slot) = meta.iter_mut().find(|(n, _)| n == name) {
        slot.1 = data.to_string();
    } else {
        meta.push((name.to_string(), data.to_string()));
    }
}

/// Split markdown into sections carrying their cumulative header path. Port of
/// `MarkdownHeaderTextSplitter.split_text` + `aggregate_lines_to_chunks` (strip_headers=True).
fn split_into_sections(text: &str) -> Vec<Section> {
    let mut lines_with_meta: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut current_content: Vec<String> = Vec::new();
    let mut current_metadata: Vec<(String, String)> = Vec::new();
    let mut initial_metadata: Vec<(String, String)> = Vec::new();
    let mut header_stack: Vec<(usize, String)> = Vec::new(); // (level, name)
    let mut in_code_block = false;
    let mut opening_fence = "";

    for raw_line in text.split('\n') {
        let stripped: String = raw_line
            .trim()
            .chars()
            .filter(|c| is_printable(*c))
            .collect();

        if !in_code_block {
            if stripped.starts_with("```") && stripped.matches("```").count() == 1 {
                in_code_block = true;
                opening_fence = "```";
            } else if stripped.starts_with("~~~") {
                in_code_block = true;
                opening_fence = "~~~";
            }
        } else if stripped.starts_with(opening_fence) {
            in_code_block = false;
            opening_fence = "";
        }

        if in_code_block {
            current_content.push(stripped);
            continue; // matches Python: skips the current_metadata sync below
        }

        let mut matched = false;
        for (sep, name) in HEADERS {
            let is_header = stripped.starts_with(sep)
                && (stripped.chars().count() == sep.len()
                    || stripped[sep.len()..].starts_with(' '));
            if is_header {
                matched = true;
                let level = sep.matches('#').count();
                // Pop headers of equal-or-deeper level, dropping their metadata.
                while let Some((lvl, _)) = header_stack.last() {
                    if *lvl >= level {
                        let (_, popped_name) = header_stack.pop().unwrap();
                        initial_metadata.retain(|(n, _)| n != &popped_name);
                    } else {
                        break;
                    }
                }
                let header_text = stripped[sep.len()..].trim().to_string();
                header_stack.push((level, name.to_string()));
                upsert(&mut initial_metadata, name, &header_text);
                // Flush the preceding block with its (parent) metadata.
                if !current_content.is_empty() {
                    lines_with_meta.push((current_content.join("\n"), current_metadata.clone()));
                    current_content.clear();
                }
                // strip_headers=True → the header line itself is dropped.
                break;
            }
        }
        if !matched {
            if !stripped.is_empty() {
                current_content.push(stripped);
            } else if !current_content.is_empty() {
                lines_with_meta.push((current_content.join("\n"), current_metadata.clone()));
                current_content.clear();
            }
        }
        current_metadata = initial_metadata.clone();
    }
    if !current_content.is_empty() {
        lines_with_meta.push((current_content.join("\n"), current_metadata.clone()));
    }

    // aggregate_lines_to_chunks: merge consecutive blocks with identical metadata.
    let mut sections: Vec<Section> = Vec::new();
    for (content, meta) in lines_with_meta {
        if let Some(last) = sections.last_mut() {
            if last.meta == meta {
                last.body.push_str("  \n");
                last.body.push_str(&content);
                continue;
            }
        }
        sections.push(Section {
            meta,
            body: content,
        });
    }
    sections
}

/// Breadcrumb base from a `<category>_<Book>__<Chapter>.md` filename, e.g.
/// `Effective-Software-Testing > Chapter-09-Integration`. Mirrors `sweep.book_breadcrumb`.
pub fn book_breadcrumb(file: &str) -> String {
    let name = std::path::Path::new(file)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(file);
    let stem = name.strip_suffix(".md").unwrap_or(name);
    let parts: Vec<&str> = stem.split("__").collect();
    let p0 = parts.first().copied().unwrap_or("");
    let chapter = parts.get(1).copied().unwrap_or("");
    let book = p0.splitn(2, '_').last().unwrap_or(p0); // text after the first '_'
    format!("{book} > {chapter}")
        .trim_matches(|c| c == ' ' || c == '>')
        .to_string()
}

/// Chunk a markdown document into breadcrumb-prefixed, recursively-sized chunks.
/// Reproduces `sweep.chunk_rcts_md`: header split → recursive pack within each section →
/// prepend `Book > Chapter > h1 > h2 > h3`. Falls back to plain recursive (book breadcrumb
/// only) for documents with no detectable sections.
pub fn chunk_markdown(
    text: &str,
    file: &str,
    chunk_size: usize,
    chunk_overlap: usize,
) -> Vec<String> {
    let base = book_breadcrumb(file);
    let sections = split_into_sections(text);
    if sections.is_empty() {
        return split_text(text, DEFAULT_SEPARATORS, chunk_size, chunk_overlap)
            .into_iter()
            .map(|c| format!("{base}\n\n{c}"))
            .collect();
    }
    let mut out = Vec::new();
    for sec in &sections {
        let mut crumb = base.clone();
        for (_, data) in &sec.meta {
            crumb.push_str(" > ");
            crumb.push_str(data);
        }
        for piece in split_text(&sec.body, DEFAULT_SEPARATORS, chunk_size, chunk_overlap) {
            out.push(format!("{crumb}\n\n{piece}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct MdCase {
        name: String,
        file: String,
        text: String,
        chunk_size: usize,
        chunk_overlap: usize,
        expected: Vec<String>,
    }

    #[derive(Deserialize)]
    struct Golden {
        md: Vec<MdCase>,
    }

    // The breadcrumb pipeline must reproduce experiments/chunking/sweep.py::chunk_rcts_md.
    #[test]
    fn md_matches_langchain_golden() {
        let raw = include_str!("../fixtures/golden_vectors.json");
        let golden: Golden = serde_json::from_str(raw).expect("parse golden fixture");
        for c in &golden.md {
            let got = chunk_markdown(&c.text, &c.file, c.chunk_size, c.chunk_overlap);
            assert_eq!(
                got, c.expected,
                "markdown golden mismatch on case '{}'",
                c.name
            );
        }
    }
}
