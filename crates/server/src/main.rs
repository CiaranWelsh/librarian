//! `librarian-collection` — read-only MCP server. Hand-rolled JSON-RPC 2.0
//! over line-delimited stdio. Three tools: search, list_documents, get_extract.
//!
//! No auth in v1 — local-network trust on Turbo per ADR-0004 / deployment view.

mod config;
mod server;
mod tools;

use clap::Parser;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use crate::server::Server;

#[derive(Parser, Debug)]
#[command(name = "librarian-collection", about = "Per-collection MCP server")]
struct Cli {
    #[arg(long)] config: PathBuf,
    /// Allocated by the supervisor (slice 015). Currently unused — server runs over stdio.
    #[arg(long)] port: Option<u16>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let server = match Server::open(&cli.config) {
        Ok(s) => s,
        Err(e) => { eprintln!("error: {e}"); return ExitCode::FAILURE; }
    };

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        if line.trim().is_empty() { continue; }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let err = json!({ "jsonrpc":"2.0","id":null,"error":{"code":-32700,"message": format!("parse: {e}")}});
                let _ = writeln!(out, "{err}");
                continue;
            }
        };
        if let Some(reply) = server.handle(&msg) {
            let _ = writeln!(out, "{reply}");
            let _ = out.flush();
        }
    }
    ExitCode::SUCCESS
}
