//! `librarian query` -- search a collection via the query daemon. Thin client: build the body,
//! POST it through the shared `Daemon`, render the hits for the resolved audience.

use serde_json::{json, Value};

use crate::commands::http::Daemon;
use crate::commands::output::{self, Hit, Render};

/// Build the search request body. Pure -- unit-testable.
pub fn search_body(collection: &str, query: &str, limit: u64) -> Value {
    json!({ "collection": collection, "query": query, "limit": limit })
}

/// POST a search and return the parsed response. Shared by `query`, `health`, and `judge`.
pub fn search(d: &Daemon, collection: &str, query: &str, limit: u64) -> Result<Value, String> {
    d.post("/v1/search", &search_body(collection, query, limit))
        .map_err(|e| e.to_string())
}

pub fn cmd_query(
    d: &Daemon,
    r: Render,
    collection: &str,
    query: &str,
    limit: u64,
) -> Result<(), String> {
    output::progress(r, &format!("searching {collection}..."));
    let started = std::time::Instant::now();
    let res = search(d, collection, query, limit);
    output::progress_clear(r);
    let value = res?;
    let hits: Vec<Hit> = serde_json::from_value(value["hits"].clone())
        .map_err(|e| format!("unexpected hits in daemon response: {e}"))?;
    if r.verbose {
        let line = format!(
            "{} hits in {} ms",
            hits.len(),
            started.elapsed().as_millis()
        );
        output::note(r, &output::dim(r, &line));
    }

    if r.json {
        let out = json!({
            "hits": hits,
            "confidence": value.get("confidence").cloned().unwrap_or(Value::Null),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?
        );
        return Ok(());
    }

    for h in &hits {
        let head = output::bold(
            r,
            &format!("[{:.3}] {}#{}", h.score, h.source_id, h.chunk_index),
        );
        let preview: String = h.text.chars().take(200).collect();
        let preview = output::highlight(r, &preview, query);
        println!("{head}\n  {preview}\n");
    }

    // Tier-0 retrieval-confidence (issue 028): the triage signal the librarian's job is to
    // surface. Dim so it doesn't compete with the hits, but always shown -- it is part of the answer.
    let c = &value["confidence"];
    if c.is_object() {
        let label = c["label"].as_str().unwrap_or("?").to_uppercase();
        let val = c["value"].as_f64().unwrap_or(0.0);
        let top = c["top_score"].as_f64().unwrap_or(0.0);
        let margin = c["margin"].as_f64().unwrap_or(0.0);
        let frag = c["fragment_rate"].as_f64().unwrap_or(0.0);
        let line = format!(
            "confidence: {label} ({val:.2})  [top {top:.3}, margin {margin:.3}, fragments {:.0}%]",
            frag * 100.0
        );
        println!("{}", output::dim(r, &line));
    }

    // Follow-up hint (terminal only): a paste-ready `extract` for the top hit, so locate->read
    // needs no hand-assembly. To stderr, so it never pollutes piped output.
    if r.tty {
        if let Some(top) = hits.first() {
            let tip = format!(
                "tip: read around it -> librarian extract {} \"{}#{}\" --context 3",
                collection, top.source_id, top.chunk_index
            );
            output::note(r, &output::dim(r, &tip));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_body_has_collection_query_limit() {
        let b = search_body("software", "rust async", 3);
        assert_eq!(b["collection"], "software");
        assert_eq!(b["query"], "rust async");
        assert_eq!(b["limit"], 3);
    }
}
