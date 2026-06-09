//! Optional client-side config (XDG): persistent defaults a daily user shouldn't retype, e.g. the
//! daemon URL. Precedence is resolved here: flag > `$LIBRARIAN_DAEMON` > config file > built-in
//! default (docs/research/cli-ux/findings.md #1). The file is `$XDG_CONFIG_HOME/librarian/
//! config.toml` (default `~/.config/librarian/config.toml`); absence is fine.

use std::path::PathBuf;

use serde::Deserialize;

use crate::commands::http::DEFAULT_TIMEOUT_SECS;

const DEFAULT_DAEMON: &str = "http://localhost:6700";

#[derive(Debug, Default, Deserialize)]
pub struct ClientConfig {
    pub daemon: Option<String>,
    pub timeout: Option<u64>,
    pub limit: Option<u64>,
}

impl ClientConfig {
    /// Load the config file, or an empty config if it is missing or unparseable.
    pub fn load() -> Self {
        match config_path().and_then(|p| std::fs::read_to_string(p).ok()) {
            Some(text) => toml::from_str(&text).unwrap_or_default(),
            None => Self::default(),
        }
    }

    /// Resolve the daemon URL: explicit flag > `$LIBRARIAN_DAEMON` > config > default.
    pub fn resolve_daemon(&self, flag: Option<&str>) -> String {
        flag.map(str::to_string)
            .or_else(|| {
                std::env::var("LIBRARIAN_DAEMON")
                    .ok()
                    .filter(|s| !s.is_empty())
            })
            .or_else(|| self.daemon.clone())
            .unwrap_or_else(|| DEFAULT_DAEMON.to_string())
    }

    /// Resolve the request timeout (seconds): flag > config > default.
    pub fn resolve_timeout(&self, flag: Option<u64>) -> u64 {
        flag.or(self.timeout).unwrap_or(DEFAULT_TIMEOUT_SECS)
    }

    /// Resolve the hit limit: flag > config > `fallback`.
    pub fn resolve_limit(&self, flag: Option<u64>, fallback: u64) -> u64 {
        flag.or(self.limit).unwrap_or(fallback)
    }
}

fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("librarian").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(daemon: Option<&str>) -> ClientConfig {
        ClientConfig {
            daemon: daemon.map(str::to_string),
            timeout: None,
            limit: None,
        }
    }

    #[test]
    fn flag_beats_config() {
        // An explicit flag wins and short-circuits before the env var, so this is deterministic
        // regardless of whether LIBRARIAN_DAEMON happens to be set in the test environment.
        assert_eq!(
            cfg(Some("http://from-config:6700")).resolve_daemon(Some("http://from-flag:6700")),
            "http://from-flag:6700"
        );
    }

    #[test]
    fn timeout_and_limit_fall_back() {
        assert_eq!(cfg(None).resolve_timeout(None), DEFAULT_TIMEOUT_SECS);
        assert_eq!(cfg(None).resolve_timeout(Some(7)), 7);
        assert_eq!(cfg(None).resolve_limit(None, 5), 5);
        assert_eq!(cfg(None).resolve_limit(Some(12), 5), 12);
    }
}
