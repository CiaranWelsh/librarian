//! Shared HTTP transport for the daemon-facing commands (query, extract, health, judge).
//!
//! One place owns: the blocking client and its timeout, the optional bearer auth, the daemon
//! base URL, and the decode of the daemon's error envelope into a `ClientError`. The error's
//! human message lives in one `Display` impl so it can be tested and kept consistent
//! (docs/research/cli-ux/findings.md #5 errors, #7 network robustness).

use std::time::Duration;

use serde_json::Value;

/// Default request timeout. A wedged daemon must never hang the CLI forever (the OS connect
/// backoff alone can reach ~127s). Overridable per command via `--timeout`.
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// A daemon request that failed. The variants carry exactly what the human message needs; the
/// `Display` impl turns each into a "what failed / why / how to fix" message.
#[derive(Debug)]
pub enum ClientError {
    /// Never opened a connection (refused, DNS failure, reset).
    Connect { url: String, cause: String },
    /// Connected, but no response within the timeout.
    Timeout { url: String, secs: u64 },
    /// The daemon answered with a non-success status and an error envelope.
    Daemon {
        status: u16,
        code: String,
        message: String,
    },
    /// The response body was not the JSON we expected.
    BadResponse { cause: String },
}

impl std::error::Error for ClientError {}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Each message states what failed, why, and how to fix it (findings.md #5). Multi-line
        // with indented cause/fix reads well in a terminal; `main` prepends "error: ".
        match self {
            ClientError::Connect { url, cause } => write!(
                f,
                "cannot reach the daemon at {url}\n  cause: {cause}\n  \
                 fix:   is it running? set LIBRARIAN_DAEMON or pass --daemon <url>"
            ),
            ClientError::Timeout { url, secs } => write!(
                f,
                "no response from the daemon at {url} within {secs}s\n  cause: request timed out\n  \
                 fix:   is the host reachable? raise --timeout, or check the daemon"
            ),
            ClientError::Daemon {
                status,
                code,
                message,
            } => {
                write!(f, "daemon returned {status} [{code}]")?;
                if !message.is_empty() {
                    write!(f, ": {message}")?;
                }
                Ok(())
            }
            ClientError::BadResponse { cause } => write!(
                f,
                "unexpected response from the daemon\n  cause: {cause}\n  \
                 fix:   the CLI and daemon versions may differ"
            ),
        }
    }
}

/// A handle to the query daemon: base URL plus a timeout-bounded blocking client, built once and
/// reused (so a multi-request command like `health` does not rebuild the client per call).
pub struct Daemon {
    base: String,
    timeout_secs: u64,
    client: reqwest::blocking::Client,
}

impl Daemon {
    pub fn new(base: &str, timeout_secs: u64) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("build blocking http client");
        Self {
            base: base.trim_end_matches('/').to_string(),
            timeout_secs,
            client,
        }
    }

    /// POST `body` to `<base><path>` and return the parsed JSON, mapping transport failures and
    /// the daemon error envelope to a typed `ClientError`.
    pub fn post(&self, path: &str, body: &Value) -> Result<Value, ClientError> {
        let url = format!("{}{}", self.base, path);
        let resp = with_auth(self.client.post(&url).json(body))
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    ClientError::Timeout {
                        url: url.clone(),
                        secs: self.timeout_secs,
                    }
                } else {
                    ClientError::Connect {
                        url: url.clone(),
                        cause: e.to_string(),
                    }
                }
            })?;
        let status = resp.status();
        let value: Value = resp.json().map_err(|e| ClientError::BadResponse {
            cause: e.to_string(),
        })?;
        if !status.is_success() {
            return Err(ClientError::Daemon {
                status: status.as_u16(),
                code: value["error"]["code"].as_str().unwrap_or("error").into(),
                message: value["error"]["message"].as_str().unwrap_or("").into(),
            });
        }
        Ok(value)
    }
}

/// Attach `Authorization: Bearer $LIBRARIAN_KEY` when set. Inert on the keyless tailnet; used once
/// the daemon is gated (issue 032 / feat/serving).
pub fn with_auth(req: reqwest::blocking::RequestBuilder) -> reqwest::blocking::RequestBuilder {
    match std::env::var("LIBRARIAN_KEY") {
        Ok(k) if !k.is_empty() => req.bearer_auth(k),
        _ => req,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn post_returns_value_on_success() {
        let mut server = mockito::Server::new();
        let m = server
            .mock("POST", "/v1/search")
            .with_status(200)
            .with_body(r#"{"hits":[],"confidence":{}}"#)
            .create();
        let d = Daemon::new(&server.url(), 5);
        let v = d.post("/v1/search", &json!({"q":"x"})).unwrap();
        assert!(v["hits"].is_array());
        m.assert();
    }

    #[test]
    fn non_success_maps_to_daemon_error_with_envelope() {
        let mut server = mockito::Server::new();
        server
            .mock("POST", "/v1/search")
            .with_status(404)
            .with_body(r#"{"error":{"code":"not_found","message":"no such collection"}}"#)
            .create();
        let d = Daemon::new(&server.url(), 5);
        match d.post("/v1/search", &json!({})).unwrap_err() {
            ClientError::Daemon {
                status,
                code,
                message,
            } => {
                assert_eq!(status, 404);
                assert_eq!(code, "not_found");
                assert!(message.contains("no such collection"));
            }
            other => panic!("expected Daemon error, got {other:?}"),
        }
    }

    #[test]
    fn unreachable_host_is_a_connect_error() {
        // Port 1 is not listening -> connection refused, fast (not a timeout).
        let d = Daemon::new("http://127.0.0.1:1", 2);
        let err = d.post("/v1/search", &json!({})).unwrap_err();
        assert!(matches!(err, ClientError::Connect { .. }), "got {err:?}");
    }

    // --- The messages themselves (RED until the Display impl is written) ---

    #[test]
    fn connect_error_message_names_target_and_cause() {
        let s = ClientError::Connect {
            url: "http://turbo:6700".into(),
            cause: "connection refused".into(),
        }
        .to_string();
        assert!(
            s.contains("http://turbo:6700"),
            "should name what we tried to reach"
        );
        assert!(s.contains("connection refused"), "should state the cause");
    }

    #[test]
    fn timeout_message_reports_url_and_seconds() {
        let s = ClientError::Timeout {
            url: "http://turbo:6700".into(),
            secs: 30,
        }
        .to_string();
        assert!(s.contains("http://turbo:6700"));
        assert!(s.contains("30"));
    }

    #[test]
    fn daemon_error_message_includes_status_and_code() {
        let s = ClientError::Daemon {
            status: 401,
            code: "unauthorized".into(),
            message: "missing key".into(),
        }
        .to_string();
        assert!(s.contains("401"));
        assert!(s.contains("unauthorized"));
    }
}
