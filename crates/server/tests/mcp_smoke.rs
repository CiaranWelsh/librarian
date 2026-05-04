//! Drive the MCP server via stdio. Tests treat the binary as a subprocess and
//! exchange JSON-RPC messages line-by-line. Gated on Qdrant being reachable.

use adapter_cache_fs::FsCache;
use adapter_chunker_blankline::BlankLineChunker;
use adapter_embedder_stub::StubEmbedder;
use adapter_extractor_text::TextExtractor;
use adapter_indexer_qdrant::QdrantIndexer;
use adapter_manifest_sqlite::SqliteManifest;
use librarian_domain::{ContentType, Document, Embedder, SourceHash, SourceId};
use librarian_runner::{BatchRunner, Pipeline};
use serde_json::{json, Value};
use sha2::Digest;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

fn url() -> String {
    std::env::var("LIBRARIAN_QDRANT_URL").unwrap_or_else(|_| "http://localhost:6533".into())
}

fn unique_collection(label: &str) -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    format!("librarian-mcp-{label}-{nanos}")
}

fn write_config(dir: &Path, collection: &str) -> std::path::PathBuf {
    let cfg = dir.join("librarian.toml");
    let body = format!(
        r#"collection = "{collection}"

[qdrant]
url = "{url}"

[paths]
manifest = "{manifest}"

[embedder]
kind = "stub"
"#,
        url = url(),
        manifest = dir.join("manifest.sqlite").display(),
    );
    std::fs::write(&cfg, body).unwrap();
    cfg
}

fn populate_collection(dir: &Path, collection: &str) -> bool {
    let stub = StubEmbedder::new();
    let dim = stub.dimension() as u64;
    let Ok(ix) = QdrantIndexer::open(&url(), collection, dim) else { return false; };
    let manifest = SqliteManifest::open(dir.join("manifest.sqlite")).unwrap();
    let cache = FsCache::open(dir.join("cache")).unwrap();

    let runner = BatchRunner {
        pipeline: Pipeline {
            extractor: TextExtractor::new(), chunker: BlankLineChunker::new(),
            embedder: stub, indexer: ix,
        },
        manifest, cache,
    };

    let path = dir.join("doc.txt");
    std::fs::write(&path, "hexagonal architecture\n\nvector databases\n\nlibrarian\n").unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let hash = SourceHash(hex::encode(sha2::Sha256::digest(&bytes)));
    let doc = Document {
        source_id: SourceId(path.display().to_string()),
        source_hash: hash, content_type: ContentType::Book, path, work_id: None,
    };
    let outcomes = runner.ingest_batch(&[doc]);
    outcomes.iter().all(|o| o.is_success())
}

struct McpClient { child: Child, stdin: std::process::ChildStdin, stdout: BufReader<std::process::ChildStdout> }

impl McpClient {
    fn spawn(config: &Path) -> Self {
        let path = env!("CARGO_BIN_EXE_librarian-collection");
        let mut child = Command::new(path)
            .arg("--config").arg(config)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn().expect("spawn server");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self { child, stdin, stdout }
    }
    fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
        let req = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        writeln!(self.stdin, "{req}").unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }
}
impl Drop for McpClient {
    fn drop(&mut self) {
        // Closing stdin signals EOF; the server exits.
        let _ = self.child.wait();
    }
}

#[test]
fn initialize_returns_protocol_version_and_server_info() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path(), &unique_collection("init"));
    if QdrantIndexer::open(&url(), &unique_collection("probe"), 32).is_err() {
        eprintln!("skip: no Qdrant"); return;
    }

    let mut client = McpClient::spawn(&cfg);
    let r = client.request(1, "initialize", json!({}));
    let result = r.get("result").expect("result");
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["serverInfo"]["name"], "librarian-collection");
}

#[test]
fn tools_list_advertises_search_list_documents_and_get_extract() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path(), &unique_collection("tlist"));
    if QdrantIndexer::open(&url(), &unique_collection("probe"), 32).is_err() {
        eprintln!("skip: no Qdrant"); return;
    }

    let mut client = McpClient::spawn(&cfg);
    let r = client.request(1, "tools/list", json!({}));
    let names: Vec<&str> = r["result"]["tools"].as_array().unwrap().iter()
        .map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"search"));
    assert!(names.contains(&"list_documents"));
    assert!(names.contains(&"get_extract"));

    // Each tool has an inputSchema with declared properties.
    for tool in r["result"]["tools"].as_array().unwrap() {
        assert!(tool["inputSchema"].is_object(), "tool {} has inputSchema", tool["name"]);
    }
}

#[test]
fn search_returns_hits_for_populated_collection() {
    let dir = tempfile::tempdir().unwrap();
    let collection = unique_collection("search");
    if !populate_collection(dir.path(), &collection) { eprintln!("skip: no Qdrant"); return; }
    let cfg = write_config(dir.path(), &collection);

    let mut client = McpClient::spawn(&cfg);
    let r = client.request(1, "tools/call", json!({
        "name": "search",
        "arguments": { "query": "vector databases", "k": 3 }
    }));
    let text = r["result"]["content"][0]["text"].as_str().expect("text");
    let parsed: Value = serde_json::from_str(text).expect("parse");
    let hits = parsed["hits"].as_array().expect("hits");
    assert!(!hits.is_empty(), "got at least one hit");
}

#[test]
fn list_documents_reflects_manifest_state() {
    let dir = tempfile::tempdir().unwrap();
    let collection = unique_collection("listdoc");
    if !populate_collection(dir.path(), &collection) { eprintln!("skip: no Qdrant"); return; }
    let cfg = write_config(dir.path(), &collection);

    let mut client = McpClient::spawn(&cfg);
    let r = client.request(1, "tools/call", json!({"name":"list_documents","arguments":{}}));
    let text = r["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["documents"].as_array().unwrap().len(), 1);
}

#[test]
fn get_extract_returns_chunks_in_range() {
    let dir = tempfile::tempdir().unwrap();
    let collection = unique_collection("extract");
    if !populate_collection(dir.path(), &collection) { eprintln!("skip: no Qdrant"); return; }
    let cfg = write_config(dir.path(), &collection);

    let mut client = McpClient::spawn(&cfg);
    let source_id = format!("{}", dir.path().join("doc.txt").display());
    let r = client.request(1, "tools/call", json!({
        "name": "get_extract",
        "arguments": { "source_id": source_id, "start": 0, "end": 2 }
    }));
    let text = r["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    let chunks = parsed["chunks"].as_array().unwrap();
    assert_eq!(chunks.len(), 2, "[0, 2) is two chunks");
    let indices: Vec<u64> = chunks.iter().map(|c| c["chunk_index"].as_u64().unwrap()).collect();
    assert_eq!(indices, vec![0, 1]);
}

#[test]
fn unknown_method_returns_jsonrpc_error() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path(), &unique_collection("err"));
    if QdrantIndexer::open(&url(), &unique_collection("probe"), 32).is_err() {
        eprintln!("skip: no Qdrant"); return;
    }
    let mut client = McpClient::spawn(&cfg);
    let r = client.request(1, "no/such/method", json!({}));
    assert!(r.get("error").is_some(), "error returned for unknown method: {r}");
}
