//! `librarian` CLI binary. Composition root for v1 commands.
//!
//! Adapter dispatch uses generics, no `Box<dyn Trait>` (per memory rule).

mod commands;
mod config;
mod docs;
mod fleet;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use commands::audit::cmd_audit;
use commands::extract::cmd_extract;
use commands::health::cmd_health;
use commands::ingest::cmd_ingest;
use commands::lifecycle::{cmd_restart, cmd_start, cmd_stop};
use commands::query::cmd_query;
use commands::remove::cmd_remove;
use commands::snapshot::{cmd_restore, cmd_snapshot};
use commands::status::{cmd_fleet_status, cmd_status_collection};

#[derive(Parser, Debug)]
#[command(name = "librarian", version, about = "Vector-DB framework CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Ingest a file or directory tree into the configured collection.
    Ingest {
        #[arg(long)]
        config: PathBuf,
        input: PathBuf,
    },
    /// Read-only quality audit of an already-ingested collection (ADR-0006).
    Audit {
        #[arg(long)]
        config: PathBuf,
    },
    /// Remove all chunks for a `source_id`.
    Remove {
        #[arg(long)]
        config: PathBuf,
        #[arg(long = "source-id")]
        source_id: String,
    },
    /// Without --config: fleet status (all collections in registry).
    /// With --config: single-collection point count + manifest summary.
    Status {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Snapshot the collection.
    Snapshot {
        #[arg(long)]
        config: PathBuf,
    },
    /// Restore from snapshot id.
    Restore {
        #[arg(long)]
        config: PathBuf,
        snapshot_id: String,
    },
    /// Start a per-collection MCP server, registered under `name`.
    Start {
        name: String,
        #[arg(long)]
        config: PathBuf,
    },
    /// Stop a registered MCP server by name.
    Stop { name: String },
    /// Restart a registered MCP server by name.
    Restart {
        name: String,
        #[arg(long)]
        config: PathBuf,
    },
    /// Query a collection via the running query daemon.
    Query {
        /// Collection name.
        collection: String,
        /// Query text.
        query: String,
        /// Max hits.
        #[arg(long, default_value_t = 5)]
        limit: u64,
        /// Daemon base URL.
        #[arg(long, default_value = "http://localhost:6700")]
        daemon: String,
    },
    /// Read a contiguous chunk window from one source via the query daemon
    /// (the read half of locate-then-extract; pair with `query`).
    Extract {
        /// Collection name.
        collection: String,
        /// Source id (the `source_id` from a `query` hit).
        source_id: String,
        /// First chunk index, inclusive.
        #[arg(long, default_value_t = 0)]
        start: u32,
        /// Last chunk index, exclusive; defaults to start + 20.
        #[arg(long)]
        end: Option<u32>,
        /// Daemon base URL.
        #[arg(long, default_value = "http://localhost:6700")]
        daemon: String,
    },
    /// Run the golden probe set against a collection and report retrieval health
    /// (hit-rate@k, MRR, fragment-rate@5); append the run to a JSONL history (issue 028).
    Health {
        /// Collection name.
        collection: String,
        /// Golden probe set (JSON: `[{"q": ..., "relevant": [..]}]`).
        #[arg(long)]
        golden: PathBuf,
        /// Top-k for hit-rate / MRR.
        #[arg(long, default_value_t = 10)]
        k: u64,
        /// Optional JSONL history file to append this run to.
        #[arg(long)]
        history: Option<PathBuf>,
        /// Daemon base URL.
        #[arg(long, default_value = "http://localhost:6700")]
        daemon: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.cmd {
        Cmd::Ingest { config, input } => cmd_ingest(&config, &input),
        Cmd::Audit { config } => cmd_audit(&config),
        Cmd::Remove { config, source_id } => cmd_remove(&config, &source_id),
        Cmd::Status { config: Some(c) } => cmd_status_collection(&c),
        Cmd::Status { config: None } => cmd_fleet_status(),
        Cmd::Snapshot { config } => cmd_snapshot(&config),
        Cmd::Restore {
            config,
            snapshot_id,
        } => cmd_restore(&config, &snapshot_id),
        Cmd::Start { name, config } => cmd_start(&name, &config),
        Cmd::Stop { name } => cmd_stop(&name),
        Cmd::Restart { name, config } => cmd_restart(&name, &config),
        Cmd::Query {
            collection,
            query,
            limit,
            daemon,
        } => cmd_query(&daemon, &collection, &query, limit),
        Cmd::Extract {
            collection,
            source_id,
            start,
            end,
            daemon,
        } => cmd_extract(&daemon, &collection, &source_id, start, end),
        Cmd::Health {
            collection,
            golden,
            k,
            history,
            daemon,
        } => cmd_health(&daemon, &collection, &golden, k, history.as_deref()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
