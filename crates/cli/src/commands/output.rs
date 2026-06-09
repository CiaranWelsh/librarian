//! Rendering: typed daemon responses, the audience decision (human / JSON / color), and small
//! style helpers. The audience is resolved once from the global flags + environment and threaded
//! through as `Render`, so each command has a single branch rather than scattered `isatty` checks
//! (docs/research/cli-ux/findings.md #3).

use std::io::{IsTerminal, Write};

use serde::{Deserialize, Serialize};

/// One search hit. These field names are the stable `--json` schema — a public contract.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Hit {
    pub score: f64,
    pub source_id: String,
    #[serde(default)]
    pub chunk_index: u64,
    #[serde(default)]
    pub text: String,
}

/// One extracted chunk.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Chunk {
    #[serde(default)]
    pub chunk_index: u64,
    #[serde(default)]
    pub text: String,
}

/// How to render output, resolved once from the global flags + environment.
#[derive(Debug, Clone, Copy)]
pub struct Render {
    pub json: bool,
    pub color: bool,
    pub quiet: bool,
    pub verbose: bool,
    /// Whether stdout is a terminal (gates tips/progress that would pollute a pipe).
    pub tty: bool,
}

impl Render {
    /// Color is on only when not `--json`, not `--no-color`, `NO_COLOR` is unset, and stdout is a
    /// terminal. `tty` is tracked separately so non-color terminals still get tips.
    pub fn resolve(json: bool, no_color: bool, quiet: bool, verbose: bool) -> Self {
        let tty = std::io::stdout().is_terminal();
        let color = !json && !no_color && std::env::var_os("NO_COLOR").is_none() && tty;
        Self {
            json,
            color,
            quiet,
            verbose,
            tty,
        }
    }
}

const RESET: &str = "\x1b[0m";

pub fn dim(r: Render, s: &str) -> String {
    if r.color {
        format!("\x1b[2m{s}{RESET}")
    } else {
        s.to_string()
    }
}

pub fn bold(r: Render, s: &str) -> String {
    if r.color {
        format!("\x1b[1m{s}{RESET}")
    } else {
        s.to_string()
    }
}

/// Bold the query's words where they appear in `text` (case-insensitive, words of >=3 chars).
/// No-op when color is off; skipped on non-ASCII text to keep byte-offset slicing safe.
pub fn highlight(r: Render, text: &str, query: &str) -> String {
    if !r.color {
        return text.to_string();
    }
    let mut out = text.to_string();
    for term in query.split_whitespace().filter(|w| w.chars().count() >= 3) {
        out = highlight_term(&out, term);
    }
    out
}

fn highlight_term(text: &str, term: &str) -> String {
    if !text.is_ascii() {
        return text.to_string();
    }
    let lower = text.to_lowercase();
    let needle = term.to_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut idx = 0;
    while let Some(rel) = lower[idx..].find(&needle) {
        let start = idx + rel;
        let end = start + needle.len();
        out.push_str(&text[idx..start]);
        out.push_str("\x1b[1m");
        out.push_str(&text[start..end]);
        out.push_str(RESET);
        idx = end;
    }
    out.push_str(&text[idx..]);
    out
}

/// Print a tip/note to stderr unless `--quiet`. Messages go to stderr so stdout stays data-only.
pub fn note(r: Render, msg: &str) {
    if !r.quiet {
        eprintln!("{msg}");
    }
}

/// Show an in-place progress line on stderr while a network call is in flight (terminal only,
/// never under --quiet/--json/pipe). Pair with `progress_clear` so it's erased before output
/// (findings.md #8: feedback within ~100ms, but it must not survive into the result).
pub fn progress(r: Render, msg: &str) {
    if r.tty && !r.quiet && !r.json {
        eprint!("{msg}\r");
        let _ = std::io::stderr().flush();
    }
}

/// Erase the progress line written by `progress`.
pub fn progress_clear(r: Render) {
    if r.tty && !r.quiet && !r.json {
        eprint!("\x1b[2K"); // clear line; the cursor is already at column 0 after the `\r`
        let _ = std::io::stderr().flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(color: bool) -> Render {
        Render {
            json: false,
            color,
            quiet: false,
            verbose: false,
            tty: color,
        }
    }

    #[test]
    fn highlight_wraps_matches_only_when_color_on() {
        let plain = highlight(r(false), "the saga pattern", "saga");
        assert_eq!(plain, "the saga pattern"); // untouched without color

        let colored = highlight(r(true), "the Saga pattern", "saga");
        assert!(colored.contains("\x1b[1mSaga\x1b[0m"), "got {colored:?}"); // case-insensitive, original case kept
    }

    #[test]
    fn highlight_ignores_short_words_and_non_ascii() {
        // short query words (<3) are not highlighted
        assert_eq!(highlight(r(true), "a b cd", "a b"), "a b cd");
        // non-ascii text is returned unchanged (no panic)
        assert_eq!(highlight(r(true), "café résumé", "cafe"), "café résumé");
    }
}
