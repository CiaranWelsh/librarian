//! `librarian extract` -- read a window of chunks from one source (the read half of
//! locate-then-extract). Accepts the `source_id#idx` token that `query` prints: the `#idx`
//! centres the window, `--context N` widens it, and with no window it shows just the referenced
//! chunk. `--start/--end` remain explicit overrides. Thin client over the shared `Daemon`.

use serde_json::json;

use crate::commands::http::Daemon;
use crate::commands::output::{self, Chunk, Render};

/// Split a query token into `(source_id, Some(center))` when it ends with `#<number>`, else
/// `(token, None)`. Source ids may themselves contain `#`, so only a trailing numeric segment is
/// treated as the chunk index. Pure -- unit-tested.
pub fn parse_source_token(token: &str) -> (&str, Option<u32>) {
    match token.rsplit_once('#') {
        Some((sid, idx)) => match idx.parse::<u32>() {
            Ok(n) => (sid, Some(n)),
            Err(_) => (token, None),
        },
        None => (token, None),
    }
}

/// Resolve the `[start, end)` window. Explicit `--start`/`--end` win; else centre on the token's
/// `#idx` with `context` chunks either side; else the legacy first-20 default. Pure -- unit-tested.
pub fn resolve_window(
    start: Option<u32>,
    end: Option<u32>,
    context: u32,
    center: Option<u32>,
) -> (u32, u32) {
    if start.is_some() || end.is_some() {
        let s = start.unwrap_or(0);
        (s, end.unwrap_or(s + 20))
    } else if let Some(c) = center {
        (c.saturating_sub(context), c + context + 1)
    } else {
        (0, 20)
    }
}

pub fn cmd_extract(
    d: &Daemon,
    r: Render,
    collection: &str,
    source: &str,
    context: u32,
    start: Option<u32>,
    end: Option<u32>,
) -> Result<(), String> {
    let (source_id, center) = parse_source_token(source);
    let (start, end) = resolve_window(start, end, context, center);
    let body = json!({
        "collection": collection,
        "source_id": source_id,
        "start": start,
        "end": end,
    });
    output::progress(r, &format!("reading {source_id}..."));
    let res = d.post("/v1/extract", &body).map_err(|e| e.to_string());
    output::progress_clear(r);
    let value = res?;
    let chunks: Vec<Chunk> = serde_json::from_value(value["chunks"].clone())
        .map_err(|e| format!("unexpected chunks in daemon response: {e}"))?;
    if chunks.is_empty() {
        return Err(format!(
            "no chunks for {source_id} in [{start}, {end})\n  \
             fix:   check the source_id and range (the #index from a query hit centres the window)"
        ));
    }

    if r.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&chunks).map_err(|e| e.to_string())?
        );
        return Ok(());
    }

    println!(
        "{}\n",
        output::bold(r, &format!("{source_id}  chunks [{start}, {end})"))
    );
    for c in &chunks {
        println!(
            "{} {}\n",
            output::dim(r, &format!("#{}", c.chunk_index)),
            c.text
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_trailing_numeric_index_only() {
        assert_eq!(
            parse_source_token("book.epub#3950"),
            ("book.epub", Some(3950))
        );
        assert_eq!(parse_source_token("plain/path.md"), ("plain/path.md", None));
        // a '#' in the source id but no trailing number -> the whole thing is the id
        assert_eq!(parse_source_token("weird#name.md"), ("weird#name.md", None));
        // only the LAST segment is the index; an earlier '#' stays in the id
        assert_eq!(parse_source_token("a#b#7"), ("a#b", Some(7)));
    }

    #[test]
    fn window_resolution_precedence() {
        // explicit start/end win, even with a centre present
        assert_eq!(resolve_window(Some(10), Some(20), 5, Some(99)), (10, 20));
        assert_eq!(resolve_window(Some(10), None, 5, None), (10, 30));
        // centre on the token index with context either side
        assert_eq!(resolve_window(None, None, 3, Some(50)), (47, 54));
        // centre near 0 saturates
        assert_eq!(resolve_window(None, None, 5, Some(2)), (0, 8));
        // centre with default context 0 -> just that chunk
        assert_eq!(resolve_window(None, None, 0, Some(42)), (42, 43));
        // no window, no centre -> legacy default
        assert_eq!(resolve_window(None, None, 0, None), (0, 20));
    }
}
