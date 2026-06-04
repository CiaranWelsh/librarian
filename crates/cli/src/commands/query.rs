//! `librarian query` -- thin HTTP client over the query daemon. No query logic.

use serde_json::json;

/// Build the (url, body) for a search request. Pure -- unit-testable.
pub fn search_request(
    daemon: &str,
    collection: &str,
    query: &str,
    limit: u64,
) -> (String, serde_json::Value) {
    let url = format!("{}/v1/search", daemon.trim_end_matches('/'));
    let body = json!({ "collection": collection, "query": query, "limit": limit });
    (url, body)
}

pub fn cmd_query(daemon: &str, collection: &str, query: &str, limit: u64) -> Result<(), String> {
    let (url, body) = search_request(daemon, collection, query, limit);
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let status = resp.status();
    let value: serde_json::Value = resp.json().map_err(|e| format!("bad response: {e}"))?;
    if !status.is_success() {
        let code = value["error"]["code"].as_str().unwrap_or("error");
        let msg = value["error"]["message"].as_str().unwrap_or("");
        return Err(format!("daemon {status} [{code}]: {msg}"));
    }
    for hit in value["hits"].as_array().cloned().unwrap_or_default() {
        let score = hit["score"].as_f64().unwrap_or(0.0);
        let sid = hit["source_id"].as_str().unwrap_or("");
        let idx = hit["chunk_index"].as_u64().unwrap_or(0);
        let text = hit["text"].as_str().unwrap_or("");
        let preview: String = text.chars().take(200).collect();
        println!("[{score:.3}] {sid}#{idx}\n  {preview}\n");
    }
    // Tier 0 retrieval-confidence (issue 028): a triage signal, not a precise grade.
    let c = &value["confidence"];
    if c.is_object() {
        let label = c["label"].as_str().unwrap_or("?").to_uppercase();
        let val = c["value"].as_f64().unwrap_or(0.0);
        let top = c["top_score"].as_f64().unwrap_or(0.0);
        let margin = c["margin"].as_f64().unwrap_or(0.0);
        let frag = c["fragment_rate"].as_f64().unwrap_or(0.0);
        println!(
            "confidence: {label} ({val:.2})  [top {top:.3}, margin {margin:.3}, fragments {:.0}%]",
            frag * 100.0
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_request_builds_url_and_body() {
        let (url, body) = search_request("http://localhost:6700/", "software", "rust async", 3);
        assert_eq!(url, "http://localhost:6700/v1/search");
        assert_eq!(body["collection"], "software");
        assert_eq!(body["query"], "rust async");
        assert_eq!(body["limit"], 3);
    }
}
