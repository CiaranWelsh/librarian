//! Pipeline runner — orchestrates extract → chunk → embed → index against the
//! domain's outbound ports. Lives outside `librarian-domain` only because its
//! tests pull in adapter crates as dev-dependencies; the runtime code depends
//! on nothing but the domain.
//!
//! Submodules:
//! - `pipeline` — `Pipeline`, the bare-bones runner + `RunError` / `RunSummary`.
//! - `batch`    — `BatchRunner`, the production runner with fault boundary and
//!                cache-aware re-ingest. Carries `Outcome`.
//! - `snapshot` — `SnapshotOrchestrator`, the out-of-band snapshot driver.

mod batch;
mod pipeline;
mod snapshot;

pub use batch::{BatchRunner, Outcome};
pub use pipeline::{Pipeline, RunError, RunSummary};
pub use snapshot::SnapshotOrchestrator;
