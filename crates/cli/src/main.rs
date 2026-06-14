//! `librarian` CLI binary. Composition root for v1 commands.
//!
//! Adapter dispatch uses generics, no `Box<dyn Trait>` (per memory rule). Global flags
//! (`--json`, `--no-color`, `-q`, `-v`) are resolved once into a `Render` and threaded into the
//! output commands; the daemon URL / timeout / limit follow flag > env > config > default
//! precedence (docs/research/cli-ux/findings.md).

mod client_config;
mod commands;
mod config;
mod docs;
mod fleet;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use std::path::PathBuf;
use std::process::ExitCode;

use commands::audit::cmd_audit;
use commands::extract::cmd_extract;
use commands::health::cmd_health;
use commands::http::Daemon;
use commands::ingest::cmd_ingest;
use commands::judge::cmd_judge;
use commands::lifecycle::{cmd_restart, cmd_start, cmd_stop};
use commands::output::Render;
use commands::query::cmd_query;
use commands::remove::cmd_remove;
use commands::snapshot::{cmd_restore, cmd_snapshot};
use commands::status::{cmd_fleet_status, cmd_status_collection};

const ABOUT: &str =
    "Search and read the librarian reference corpus (a vector-DB RAG over your books and papers).";

const AFTER_HELP: &str = "\
Examples:
  librarian query software \"how does hexagonal architecture work?\"
  librarian extract software \"book.epub#3950\" --context 5
  librarian query particle-physics \"time of arrival calibration\" --json | jq

Workflow (locate, then read):
  1. `query` a collection to find relevant chunks (each hit is <source_id>#<index>)
  2. paste a hit token into `extract` to read the passage around it

Tip: set LIBRARIAN_DAEMON=http://turbo:6700 once to skip --daemon on every call.";

#[derive(Parser, Debug)]
#[command(name = "librarian", version, about = ABOUT, after_help = AFTER_HELP, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
    /// Output machine-readable JSON (stable schema) instead of human text.
    #[arg(long, global = true)]
    json: bool,
    /// Disable color (also auto-off when NO_COLOR is set or stdout is not a terminal).
    #[arg(long = "no-color", global = true)]
    no_color: bool,
    /// Suppress tips and progress; show only results and errors.
    #[arg(short = 'q', long, global = true)]
    quiet: bool,
    /// Extra detail on stderr.
    #[arg(short = 'v', long, global = true)]
    verbose: bool,
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
    /// Search a collection for the most relevant chunks.
    ///
    /// Each hit is `<source_id>#<index>`; paste a hit into `extract` to read its passage.
    #[command(
        after_help = "Example:\n  librarian query software \"what is a saga pattern?\" --limit 8"
    )]
    Query {
        /// Collection name (e.g. software, particle-physics).
        collection: String,
        /// What to search for (a question works better than bare keywords).
        query: String,
        /// Max hits to return [default: 5].
        #[arg(short = 'n', long)]
        limit: Option<u64>,
        /// Daemon base URL (overrides $LIBRARIAN_DAEMON and config; default http://localhost:6700).
        #[arg(long)]
        daemon: Option<String>,
        /// Request timeout in seconds [default: 30].
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// Read a window of chunks from one source (the read half of locate-then-extract).
    ///
    /// Accepts the `<source_id>#<index>` token that `query` prints; the index centres the window.
    #[command(
        after_help = "Examples:\n  librarian extract software \"book.epub#3950\"             # just that chunk\n  librarian extract software \"book.epub#3950\" --context 5  # 5 chunks either side"
    )]
    Extract {
        /// Collection name.
        collection: String,
        /// A source token from a query hit, e.g. "book.epub#3950" (the #index centres the window).
        source: String,
        /// Chunks to include either side of the #index.
        #[arg(short = 'c', long, default_value_t = 0)]
        context: u32,
        /// Explicit window start (inclusive); overrides --context.
        #[arg(long)]
        start: Option<u32>,
        /// Explicit window end (exclusive); defaults to start + 20.
        #[arg(long)]
        end: Option<u32>,
        /// Daemon base URL (overrides $LIBRARIAN_DAEMON and config).
        #[arg(long)]
        daemon: Option<String>,
        /// Request timeout in seconds [default: 30].
        #[arg(long)]
        timeout: Option<u64>,
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
        /// Daemon base URL (overrides $LIBRARIAN_DAEMON and config).
        #[arg(long)]
        daemon: Option<String>,
        /// Request timeout in seconds [default: 30].
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// LLM context-relevance judge over a query's top-k chunks (issue 028, Tier 1).
    /// The accurate on-demand RAG quality read. Operator-only: needs `OPENAI_API_KEY`.
    Judge {
        /// Collection name.
        collection: String,
        /// Query text.
        query: String,
        /// Number of top chunks to judge.
        #[arg(long, default_value_t = 5)]
        k: u64,
        /// Judge model (default gpt-4o-mini).
        #[arg(long)]
        model: Option<String>,
        /// Daemon base URL (overrides $LIBRARIAN_DAEMON and config).
        #[arg(long)]
        daemon: Option<String>,
        /// Request timeout in seconds [default: 30].
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// Add a resource to a collection (quality-gated; preview by default).
    Add {
        /// Path to the resource file to add (omit only with --undo).
        path: Option<PathBuf>,
        /// Collection to add the resource to.
        #[arg(long)]
        to: String,
        /// Write to the collection (omit for dry-run preview).
        #[arg(long)]
        commit: bool,
        /// Override the shelf segment of the canonical path.
        #[arg(long)]
        shelf: Option<String>,
        /// Override the slug derived from the filename.
        #[arg(long)]
        slug: Option<String>,
        /// Move the source file into the corpus root instead of copying.
        #[arg(long = "move")]
        r#move: bool,
        /// Skip quality gates and ingest regardless of score.
        #[arg(long)]
        force: bool,
        /// Run the LLM judge after ingest and report scores.
        #[arg(long)]
        judge: bool,
        /// Remove a previously-added resource by source_id.
        #[arg(long)]
        undo: Option<String>,
    },
    /// Print a shell completion script (bash, zsh, fish, ...) to stdout.
    #[command(after_help = "Example:\n  librarian completions zsh > ~/.zfunc/_librarian")]
    Completions {
        /// Target shell.
        shell: Shell,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let render = Render::resolve(cli.json, cli.no_color, cli.quiet, cli.verbose);
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
            timeout,
        } => {
            let cfg = client_config::ClientConfig::load();
            let d = Daemon::new(
                &cfg.resolve_daemon(daemon.as_deref()),
                cfg.resolve_timeout(timeout),
            );
            cmd_query(&d, render, &collection, &query, cfg.resolve_limit(limit, 5))
        }
        Cmd::Extract {
            collection,
            source,
            context,
            start,
            end,
            daemon,
            timeout,
        } => {
            let cfg = client_config::ClientConfig::load();
            let d = Daemon::new(
                &cfg.resolve_daemon(daemon.as_deref()),
                cfg.resolve_timeout(timeout),
            );
            cmd_extract(&d, render, &collection, &source, context, start, end)
        }
        Cmd::Health {
            collection,
            golden,
            k,
            history,
            daemon,
            timeout,
        } => {
            let cfg = client_config::ClientConfig::load();
            let d = Daemon::new(
                &cfg.resolve_daemon(daemon.as_deref()),
                cfg.resolve_timeout(timeout),
            );
            cmd_health(&d, render, &collection, &golden, k, history.as_deref())
        }
        Cmd::Judge {
            collection,
            query,
            k,
            model,
            daemon,
            timeout,
        } => {
            let cfg = client_config::ClientConfig::load();
            let d = Daemon::new(
                &cfg.resolve_daemon(daemon.as_deref()),
                cfg.resolve_timeout(timeout),
            );
            cmd_judge(&d, render, &collection, &query, k, model.as_deref())
        }
        Cmd::Add {
            path,
            to,
            commit,
            shelf,
            slug,
            r#move,
            force,
            judge,
            undo,
        } => commands::add::cmd_add(
            commands::add::AddArgs {
                path,
                to,
                commit,
                shelf,
                slug,
                move_: r#move,
                force,
                judge,
                undo,
            },
            render,
        ),
        Cmd::Completions { shell } => {
            generate(
                shell,
                &mut Cli::command(),
                "librarian",
                &mut std::io::stdout(),
            );
            Ok(())
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
