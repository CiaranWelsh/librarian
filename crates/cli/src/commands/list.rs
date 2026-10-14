//! `librarian list` -- the collections the daemon serves, with document and chunk counts.
//! Thin client over `/v1/collections` (name, points) and `/v1/documents` (distinct sources).

use serde::{Deserialize, Serialize};

use crate::commands::http::Daemon;
use crate::commands::output::{self, Render};

#[derive(Debug, Serialize, PartialEq)]
pub struct CollectionRow {
    pub name: String,
    pub documents: usize,
    pub chunks: u64,
}

#[derive(Deserialize)]
struct Collection {
    name: String,
    points: u64,
}

pub fn fetch_collections(d: &Daemon) -> Result<Vec<CollectionRow>, String> {
    let v = d.get("/v1/collections", &[]).map_err(|e| e.to_string())?;
    let cols: Vec<Collection> = serde_json::from_value(v["collections"].clone())
        .map_err(|e| format!("unexpected collections in daemon response: {e}"))?;
    cols.into_iter()
        .map(|c| {
            // ponytail: counts documents by fetching the whole source list (~1s for software);
            // add a count to /v1/collections if this gets slow.
            let v = d
                .get("/v1/documents", &[("collection", &c.name)])
                .map_err(|e| e.to_string())?;
            let docs: Vec<String> = serde_json::from_value(v["documents"].clone())
                .map_err(|e| format!("unexpected documents in daemon response: {e}"))?;
            Ok(CollectionRow {
                name: c.name,
                documents: docs.len(),
                chunks: c.points,
            })
        })
        .collect()
}

pub fn cmd_list(d: &Daemon, r: Render) -> Result<(), String> {
    output::progress(r, "listing collections...");
    let rows = fetch_collections(d);
    output::progress_clear(r);
    let rows = rows?;

    if r.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&rows).map_err(|e| e.to_string())?
        );
        return Ok(());
    }

    let w = rows
        .iter()
        .map(|c| c.name.len())
        .chain(["COLLECTION".len()])
        .max()
        .unwrap_or(0);
    println!(
        "{}",
        output::bold(
            r,
            &format!("{:<w$}  {:>9}  {:>9}", "COLLECTION", "DOCUMENTS", "CHUNKS")
        )
    );
    for c in &rows {
        println!("{:<w$}  {:>9}  {:>9}", c.name, c.documents, c.chunks);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_combine_point_counts_with_document_counts() {
        let mut server = mockito::Server::new();
        server
            .mock("GET", "/v1/collections")
            .with_body(r#"{"collections":[{"name":"software","points":120},{"name":"trading","points":7}]}"#)
            .create();
        server
            .mock("GET", "/v1/documents?collection=software")
            .with_body(r#"{"documents":["a.epub","b.pdf"]}"#)
            .create();
        server
            .mock("GET", "/v1/documents?collection=trading")
            .with_body(r#"{"documents":["c.pdf"]}"#)
            .create();
        let d = Daemon::new(&server.url(), 5);
        assert_eq!(
            fetch_collections(&d).unwrap(),
            vec![
                CollectionRow {
                    name: "software".into(),
                    documents: 2,
                    chunks: 120
                },
                CollectionRow {
                    name: "trading".into(),
                    documents: 1,
                    chunks: 7
                },
            ]
        );
    }
}
