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

use commands::ingest::cmd_ingest;
use commands::lifecycle::{cmd_restart, cmd_start, cmd_stop};
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
        #[arg(long)] config: PathBuf,
        input: PathBuf,
    },
    /// Remove all chunks for a `source_id`.
    Remove {
        #[arg(long)] config: PathBuf,
        #[arg(long = "source-id")] source_id: String,
    },
    /// Without --config: fleet status (all collections in registry).
    /// With --config: single-collection point count + manifest summary.
    Status {
        #[arg(long)] config: Option<PathBuf>,
    },
    /// Snapshot the collection.
    Snapshot { #[arg(long)] config: PathBuf },
    /// Restore from snapshot id.
    Restore { #[arg(long)] config: PathBuf, snapshot_id: String },
    /// Start a per-collection MCP server, registered under `name`.
    Start   { name: String, #[arg(long)] config: PathBuf },
    /// Stop a registered MCP server by name.
    Stop    { name: String },
    /// Restart a registered MCP server by name.
    Restart { name: String, #[arg(long)] config: PathBuf },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.cmd {
        Cmd::Ingest { config, input } => cmd_ingest(&config, &input),
        Cmd::Remove { config, source_id } => cmd_remove(&config, &source_id),
        Cmd::Status { config: Some(c) } => cmd_status_collection(&c),
        Cmd::Status { config: None } => cmd_fleet_status(),
        Cmd::Snapshot { config } => cmd_snapshot(&config),
        Cmd::Restore { config, snapshot_id } => cmd_restore(&config, &snapshot_id),
        Cmd::Start { name, config } => cmd_start(&name, &config),
        Cmd::Stop { name } => cmd_stop(&name),
        Cmd::Restart { name, config } => cmd_restart(&name, &config),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
