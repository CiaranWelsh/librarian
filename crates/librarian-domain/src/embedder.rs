//! Embedder port. The error type is **fixed** (not associated) so the
//! `FallbackEmbedder` combinator in slice 011 can pattern-match
//! `Recoverable` vs `Terminal` without knowing the concrete adapter.

use thiserror::Error;

use crate::adapter_identity::AdapterIdentity;
use crate::chunk::Vector;

#[derive(Debug, Error)]
pub enum EmbedderError {
    #[error("recoverable: {0}")]
    Recoverable(String),
    #[error("terminal: {0}")]
    Terminal(String),
}

pub trait Embedder: AdapterIdentity {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError>;
    fn dimension(&self) -> usize;
    /// If the most recent `embed` was wrapped by a fallback combinator, the
    /// runner reads this to decide between `Success` / `RecoveredViaFallback`
    /// / multi-error `Failed`. Default: never a fallback.
    fn last_event(&self) -> Option<FallbackEvent> { None }
}

/// Communicates fallback-combinator outcomes to the runner without changing
/// the trait's success/error shape (slice 011).
#[derive(Debug, Clone)]
pub struct FallbackEvent {
    pub primary_error: String,
    /// `true` if the fallback succeeded; `false` if both primary and fallback failed.
    pub recovered: bool,
    /// Populated when the fallback also failed terminally.
    pub fallback_error: Option<String>,
}
