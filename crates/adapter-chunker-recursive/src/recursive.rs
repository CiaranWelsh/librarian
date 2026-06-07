//! Faithful port of LangChain's `RecursiveCharacterTextSplitter` (literal separators,
//! `keep_separator=True`, character length). Mirrors `character.py::_split_text` +
//! `base.py::_merge_splits`/`_join_docs` (archived in `docs/research/chunking/langchain-reference/`).
//!
//! Lengths are counted in characters (`chars().count()`), matching Python `len(str)`.

/// LangChain's default separator hierarchy: paragraph, line, word, character.
pub const DEFAULT_SEPARATORS: &[&str] = &["\n\n", "\n", " ", ""];

fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Split a chunk into chunks of at most `chunk_size` characters, with `chunk_overlap`
/// characters carried between consecutive chunks. Reproduces `RecursiveCharacterTextSplitter`.
pub fn split_text(
    text: &str,
    separators: &[&str],
    chunk_size: usize,
    chunk_overlap: usize,
) -> Vec<String> {
    split_recursive(text, separators, chunk_size, chunk_overlap)
}

fn split_recursive(
    text: &str,
    separators: &[&str],
    chunk_size: usize,
    chunk_overlap: usize,
) -> Vec<String> {
    let mut final_chunks: Vec<String> = Vec::new();

    // Pick the first separator that occurs in `text` (else the last one), and keep the
    // finer separators to recurse into over-budget pieces. An empty separator stops here.
    let mut separator: &str = separators.last().copied().unwrap_or("");
    let mut new_separators: &[&str] = &[];
    for (i, s) in separators.iter().enumerate() {
        if s.is_empty() {
            separator = s;
            break;
        }
        if text.contains(s) {
            separator = s;
            new_separators = &separators[i + 1..];
            break;
        }
    }

    let splits = split_with_separator(text, separator);
    // keep_separator=True → the separator is already attached to each piece, so pieces are
    // merged with an empty join separator.
    let mut good: Vec<String> = Vec::new();
    for s in splits {
        if char_len(&s) < chunk_size {
            good.push(s);
        } else {
            if !good.is_empty() {
                final_chunks.extend(merge_splits(&good, "", chunk_size, chunk_overlap));
                good.clear();
            }
            if new_separators.is_empty() {
                final_chunks.push(s);
            } else {
                final_chunks.extend(split_recursive(
                    &s,
                    new_separators,
                    chunk_size,
                    chunk_overlap,
                ));
            }
        }
    }
    if !good.is_empty() {
        final_chunks.extend(merge_splits(&good, "", chunk_size, chunk_overlap));
    }
    final_chunks
}

/// Split `text` on `separator`, keeping the separator attached to the *start* of each
/// following piece (LangChain `keep_separator="start"`). Empty separator → individual
/// characters. Empty pieces are dropped.
fn split_with_separator(text: &str, separator: &str) -> Vec<String> {
    if separator.is_empty() {
        return text.chars().map(|c| c.to_string()).collect();
    }
    let parts: Vec<&str> = text.split(separator).collect();
    let mut out: Vec<String> = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        let piece = if i == 0 {
            part.to_string()
        } else {
            format!("{separator}{part}")
        };
        if !piece.is_empty() {
            out.push(piece);
        }
    }
    out
}

/// Join `docs` with `separator` and strip leading/trailing whitespace
/// (`strip_whitespace=True`). Returns `None` for an empty result.
fn join_docs(docs: &[&str], separator: &str) -> Option<String> {
    let text = docs.join(separator);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Pack `splits` into chunks of at most `chunk_size`, carrying `chunk_overlap` characters
/// of trailing pieces into the next chunk. Direct port of `base.py::_merge_splits`.
fn merge_splits(
    splits: &[String],
    separator: &str,
    chunk_size: usize,
    chunk_overlap: usize,
) -> Vec<String> {
    let sep_len = char_len(separator);
    let mut docs: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut total: usize = 0;

    for d in splits {
        let len = char_len(d);
        let sep_if_nonempty = if !current.is_empty() { sep_len } else { 0 };
        if total + len + sep_if_nonempty > chunk_size {
            if !current.is_empty() {
                if let Some(doc) = join_docs(&current, separator) {
                    docs.push(doc);
                }
                // Pop from the front until we're within the overlap budget (and back under
                // chunk_size). The separator term re-checks `current` each iteration.
                while total > chunk_overlap
                    || (total + len + (if !current.is_empty() { sep_len } else { 0 }) > chunk_size
                        && total > 0)
                {
                    if current.is_empty() {
                        break;
                    }
                    total -= char_len(current[0]) + (if current.len() > 1 { sep_len } else { 0 });
                    current.remove(0);
                }
            }
        }
        current.push(d);
        total += len + (if current.len() > 1 { sep_len } else { 0 });
    }
    if let Some(doc) = join_docs(&current, separator) {
        docs.push(doc);
    }
    docs
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct RctsCase {
        name: String,
        text: String,
        chunk_size: usize,
        chunk_overlap: usize,
        expected: Vec<String>,
    }

    #[derive(Deserialize)]
    struct Golden {
        rcts: Vec<RctsCase>,
    }

    // The Rust port must reproduce LangChain's RecursiveCharacterTextSplitter byte-for-byte.
    // Source of `expected`: experiments/chunking/gen_golden.py (issue 027).
    #[test]
    fn rcts_matches_langchain_golden() {
        let raw = include_str!("../fixtures/golden_vectors.json");
        let golden: Golden = serde_json::from_str(raw).expect("parse golden fixture");
        for c in &golden.rcts {
            let got = split_text(&c.text, DEFAULT_SEPARATORS, c.chunk_size, c.chunk_overlap);
            assert_eq!(got, c.expected, "RCTS golden mismatch on case '{}'", c.name);
        }
    }
}
