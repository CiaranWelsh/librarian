//! Append-only JSONL access log — traffic monitoring without a telemetry stack.
//!
//! One line per `/v1` request: ts (unix secs), route, status, ms, user (when auth resolved
//! one), and for searches the collection + confidence label (+ query text unless disabled).
//! Analysis is `jq` over the file; anything fancier (issue 033) can consume the same lines
//! later. Writes are best-effort — logging must never fail a request — and the file is
//! opened per write, so plain `mv` rotation works with no signal handling.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use serde_json::json;

use crate::auth::Identity;

pub struct AccessLog {
    path: PathBuf,
    /// Log the search query text. Off = shape-only lines (route/user/collection/status).
    queries: bool,
}

/// Per-request detail a handler attaches to its response extensions for the logger.
#[derive(Clone)]
pub struct LogFields {
    pub collection: String,
    pub query: String,
    pub confidence: &'static str,
}

impl AccessLog {
    pub fn new(path: PathBuf, queries: bool) -> Self {
        Self { path, queries }
    }

    /// Best-effort append; a failed write drops the line, never the request.
    fn append(&self, line: &serde_json::Value) {
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let mut s = line.to_string();
            s.push('\n');
            let _ = f.write_all(s.as_bytes());
        }
    }
}

/// Outermost `/v1` middleware: times the request, then writes one JSONL line built from the
/// response status plus whatever `auth_mw` (Identity) and the handler (LogFields) attached
/// to the response extensions. `None` state = logging disabled, pure pass-through.
pub async fn access_mw(
    State(log): State<Option<Arc<AccessLog>>>,
    req: Request,
    next: Next,
) -> Response {
    let Some(log) = log else {
        return next.run(req).await;
    };
    let route = req.uri().path().to_string();
    let start = Instant::now();
    let resp = next.run(req).await;

    let mut line = json!({
        "ts": SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        "route": route,
        "status": resp.status().as_u16(),
        "ms": start.elapsed().as_millis() as u64,
    });
    if let Some(id) = resp.extensions().get::<Identity>() {
        line["user"] = id.user.clone().into();
    }
    if let Some(f) = resp.extensions().get::<LogFields>() {
        line["collection"] = f.collection.clone().into();
        line["confidence"] = f.confidence.into();
        if log.queries {
            line["query"] = f.query.clone().into();
        }
    }
    log.append(&line);
    resp
}
