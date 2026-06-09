//! `librarian extract` -- pull a contiguous chunk window from one source via the
//! query daemon. The read half of locate-then-extract: `query` finds the chunk,
//! `extract` reads the passage around it. Thin HTTP client, no logic.

use serde_json::json;

/// Build the (url, body) for an extract request. Pure -- unit-testable.
pub fn extract_request(
    daemon: &str,
    collection: &str,
    source_id: &str,
    start: u32,
    end: u32,
) -> (String, serde_json::Value) {
    let url = format!("{}/v1/extract", daemon.trim_end_matches('/'));
    let body = json!({
        "collection": collection,
        "source_id": source_id,
        "start": start,
        "end": end,
    });
    (url, body)
}

pub fn cmd_extract(
    daemon: &str,
    collection: &str,
    source_id: &str,
    start: u32,
    end: Option<u32>,
) -> Result<(), String> {
    let end = end.unwrap_or(start + 20);
    let (url, body) = extract_request(daemon, collection, source_id, start, end);
    let client = reqwest::blocking::Client::new();
    let resp = crate::commands::query::with_auth(client.post(&url).json(&body))
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let status = resp.status();
    let value: serde_json::Value = resp.json().map_err(|e| format!("bad response: {e}"))?;
    if !status.is_success() {
        let code = value["error"]["code"].as_str().unwrap_or("error");
        let msg = value["error"]["message"].as_str().unwrap_or("");
        return Err(format!("daemon {status} [{code}]: {msg}"));
    }
    let chunks = value["chunks"].as_array().cloned().unwrap_or_default();
    if chunks.is_empty() {
        return Err(format!(
            "no chunks for {source_id} in [{start}, {end}) -- check the source_id and range"
        ));
    }
    println!("{source_id}  chunks [{start}, {end})\n");
    for c in chunks {
        let idx = c["chunk_index"].as_u64().unwrap_or(0);
        let text = c["text"].as_str().unwrap_or("");
        println!("#{idx} {text}\n");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_request_builds_url_and_body() {
        let (url, body) =
            extract_request("http://localhost:6700/", "software", "books/foo.md", 10, 20);
        assert_eq!(url, "http://localhost:6700/v1/extract");
        assert_eq!(body["collection"], "software");
        assert_eq!(body["source_id"], "books/foo.md");
        assert_eq!(body["start"], 10);
        assert_eq!(body["end"], 20);
    }
}
