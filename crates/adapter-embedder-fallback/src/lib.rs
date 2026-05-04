//! `FallbackEmbedder<P, F>` — try `P`, on `EmbedderError::Recoverable` try `F`.
//! Generic over both, no `dyn`.
//!
//! Outcome surfaces to the runner via `Embedder::last_event()` (slice 011).

use librarian_domain::{
    AdapterIdentity, ConfigHash, Embedder, EmbedderError, FallbackEvent, StageVersion, Vector,
};
use std::cell::RefCell;

pub struct FallbackEmbedder<P, F> {
    pub primary: P,
    pub fallback: F,
    last: RefCell<Option<FallbackEvent>>,
}

impl<P, F> FallbackEmbedder<P, F>
where
    P: Embedder,
    F: Embedder,
{
    pub fn new(primary: P, fallback: F) -> Self {
        assert_eq!(
            primary.dimension(), fallback.dimension(),
            "fallback embedder dimension mismatch: primary {}, fallback {}",
            primary.dimension(), fallback.dimension(),
        );
        Self { primary, fallback, last: RefCell::new(None) }
    }
}

impl<P, F> AdapterIdentity for FallbackEmbedder<P, F>
where
    P: Embedder,
    F: Embedder,
{
    fn name(&self) -> &str { "fallback-embedder" }
    fn version(&self) -> StageVersion {
        StageVersion(format!("p={};f={}", self.primary.name(), self.fallback.name()))
    }
    fn config_hash(&self) -> ConfigHash {
        ConfigHash(format!(
            "p={}-{}-{};f={}-{}-{}",
            self.primary.name(), self.primary.version().0, self.primary.config_hash().0,
            self.fallback.name(), self.fallback.version().0, self.fallback.config_hash().0,
        ))
    }
}

impl<P, F> Embedder for FallbackEmbedder<P, F>
where
    P: Embedder,
    F: Embedder,
{
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        *self.last.borrow_mut() = None;
        match self.primary.embed(texts) {
            Ok(v) => Ok(v),
            Err(EmbedderError::Terminal(t)) => Err(EmbedderError::Terminal(t)), // no fallback
            Err(EmbedderError::Recoverable(pe)) => match self.fallback.embed(texts) {
                Ok(v) => {
                    *self.last.borrow_mut() = Some(FallbackEvent {
                        primary_error: pe, recovered: true, fallback_error: None,
                    });
                    Ok(v)
                }
                Err(fe) => {
                    let fe_msg = fe.to_string();
                    *self.last.borrow_mut() = Some(FallbackEvent {
                        primary_error: pe.clone(),
                        recovered: false,
                        fallback_error: Some(fe_msg.clone()),
                    });
                    Err(EmbedderError::Terminal(format!(
                        "primary recoverable: {pe}; fallback terminal: {fe_msg}"
                    )))
                }
            },
        }
    }

    fn dimension(&self) -> usize { self.primary.dimension() }

    fn last_event(&self) -> Option<FallbackEvent> {
        self.last.borrow_mut().take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct Stub {
        result: Cell<Option<Result<Vec<Vector>, EmbedderError>>>,
        calls: Cell<usize>,
        name: &'static str,
    }
    impl Stub {
        fn ok(vec: Vec<Vector>) -> Self {
            Self { result: Cell::new(Some(Ok(vec))), calls: Cell::new(0), name: "stub" }
        }
        fn err(e: EmbedderError) -> Self {
            Self { result: Cell::new(Some(Err(e))), calls: Cell::new(0), name: "stub" }
        }
    }
    impl AdapterIdentity for Stub {
        fn name(&self) -> &str { self.name }
        fn version(&self) -> StageVersion { StageVersion("v".into()) }
        fn config_hash(&self) -> ConfigHash { ConfigHash("c".into()) }
    }
    impl Embedder for Stub {
        fn embed(&self, _: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
            self.calls.set(self.calls.get() + 1);
            self.result.replace(None).expect("stub configured for one call")
        }
        fn dimension(&self) -> usize { 4 }
    }

    #[test]
    fn primary_ok_short_circuits_fallback() {
        let primary = Stub::ok(vec![vec![1.0; 4]]);
        let fallback = Stub::ok(vec![vec![9.0; 4]]);
        let f = FallbackEmbedder::new(primary, fallback);
        let v = f.embed(&["x"]).unwrap();
        assert_eq!(v, vec![vec![1.0; 4]]);
        assert_eq!(f.fallback.calls.get(), 0);
        assert!(f.last_event().is_none());
    }

    #[test]
    fn primary_recoverable_then_fallback_succeeds_records_event() {
        let primary = Stub::err(EmbedderError::Recoverable("net blip".into()));
        let fallback = Stub::ok(vec![vec![9.0; 4]]);
        let f = FallbackEmbedder::new(primary, fallback);
        let v = f.embed(&["x"]).unwrap();
        assert_eq!(v, vec![vec![9.0; 4]]);
        assert_eq!(f.fallback.calls.get(), 1);
        let ev = f.last_event().expect("event recorded");
        assert!(ev.recovered);
        assert_eq!(ev.primary_error, "net blip");
        assert!(ev.fallback_error.is_none());
    }

    #[test]
    fn primary_recoverable_fallback_terminal_returns_terminal_with_both_messages() {
        let primary = Stub::err(EmbedderError::Recoverable("net blip".into()));
        let fallback = Stub::err(EmbedderError::Terminal("auth fail".into()));
        let f = FallbackEmbedder::new(primary, fallback);
        let err = f.embed(&["x"]).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("net blip"));
        assert!(msg.contains("auth fail"));
        let ev = f.last_event().expect("event recorded");
        assert!(!ev.recovered);
        assert_eq!(ev.primary_error, "net blip");
        assert_eq!(ev.fallback_error.as_deref(), Some("terminal: auth fail"));
    }

    #[test]
    fn primary_terminal_does_not_call_fallback() {
        let primary = Stub::err(EmbedderError::Terminal("auth fail".into()));
        let fallback = Stub::ok(vec![vec![9.0; 4]]);
        let f = FallbackEmbedder::new(primary, fallback);
        let _ = f.embed(&["x"]).unwrap_err();
        assert_eq!(f.fallback.calls.get(), 0);
        assert!(f.last_event().is_none());
    }

    #[test]
    fn last_event_is_consumed_on_read() {
        let primary = Stub::err(EmbedderError::Recoverable("p".into()));
        let fallback = Stub::ok(vec![vec![0.0; 4]]);
        let f = FallbackEmbedder::new(primary, fallback);
        let _ = f.embed(&["x"]).unwrap();
        assert!(f.last_event().is_some());
        assert!(f.last_event().is_none(), "consumed");
    }

    #[test]
    #[should_panic(expected = "dimension mismatch")]
    fn mismatched_dimensions_panic_on_construction() {
        struct D8;
        impl AdapterIdentity for D8 {
            fn name(&self) -> &str { "d8" }
            fn version(&self) -> StageVersion { StageVersion("v".into()) }
            fn config_hash(&self) -> ConfigHash { ConfigHash("c".into()) }
        }
        impl Embedder for D8 {
            fn embed(&self, _: &[&str]) -> Result<Vec<Vector>, EmbedderError> { Ok(vec![]) }
            fn dimension(&self) -> usize { 8 }
        }
        let primary = Stub::ok(vec![]);
        let fallback = D8;
        let _ = FallbackEmbedder::new(primary, fallback);
    }
}
