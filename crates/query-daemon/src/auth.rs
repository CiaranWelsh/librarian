//! Bearer-key auth + per-key token-bucket rate limit (issue 032).
//!
//! Keys live in a TOML file (default `~/.librarian/keys.toml`), hot-reloaded when the file's
//! mtime changes, so the operator can add or revoke a key with no daemon restart. **Fail-closed**:
//! once auth is wired onto a route, a request without a known key is rejected (a missing keys
//! file means an empty key set, i.e. everything is rejected). The rate limit is a runaway/abuse
//! guard — queries are cheap — not a cost gate.
//!
//! This module is pure (no axum types) so it unit-tests in isolation; the axum middleware that
//! calls `AuthState::check` lives in `lib.rs`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Instant, SystemTime};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct KeysConfig {
    #[serde(default)]
    pub defaults: Defaults,
    /// `key string -> entry`; the bearer token IS the TOML table key (`[keys.<token>]`).
    #[serde(default)]
    pub keys: HashMap<String, KeyEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Defaults {
    #[serde(default = "default_rpm")]
    pub rpm: u32,
}
impl Default for Defaults {
    fn default() -> Self {
        Self { rpm: default_rpm() }
    }
}
fn default_rpm() -> u32 {
    60
}

#[derive(Debug, Clone, Deserialize)]
pub struct KeyEntry {
    pub user: String,
    /// Per-user override of `defaults.rpm`.
    #[serde(default)]
    pub rpm: Option<u32>,
}

/// Token bucket: capacity = `rpm`, refilling at `rpm/60` tokens per second (a short burst up to
/// `rpm`, then a steady `rpm` per minute).
struct Bucket {
    tokens: f64,
    last: Instant,
    rpm: u32,
}
impl Bucket {
    fn new(rpm: u32, now: Instant) -> Self {
        Self {
            tokens: rpm as f64,
            last: now,
            rpm,
        }
    }
    /// Consume one token; when empty, return whole seconds until the next token is available.
    fn take(&mut self, now: Instant) -> Result<(), u64> {
        let rate = (self.rpm as f64 / 60.0).max(f64::MIN_POSITIVE);
        self.tokens = (self.tokens + now.saturating_duration_since(self.last).as_secs_f64() * rate)
            .min(self.rpm as f64);
        self.last = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            Err(((1.0 - self.tokens) / rate).ceil() as u64)
        }
    }
}

/// Resolved caller identity (the `user` from keys.toml), attached to the request for logging.
#[derive(Debug, Clone)]
pub struct Identity {
    pub user: String,
}

/// Why a request was rejected.
#[derive(Debug)]
pub enum AuthReject {
    /// 401 — missing or unknown key.
    Unauthorized,
    /// 429 — over the per-key rate; carries seconds-until-retry.
    RateLimited(u64),
}

struct Inner {
    mtime: Option<SystemTime>,
    cfg: KeysConfig,
    buckets: HashMap<String, Bucket>,
}

pub struct AuthState {
    path: Option<PathBuf>,
    inner: Mutex<Inner>,
}

impl AuthState {
    /// Load keys from `path`. A missing or unparseable file yields an empty key set
    /// (fail-closed: every `/v1` request 401s until the operator writes keys.toml).
    pub fn new(path: PathBuf) -> Self {
        let (mtime, cfg) = load(&path);
        Self {
            path: Some(path),
            inner: Mutex::new(Inner {
                mtime,
                cfg,
                buckets: HashMap::new(),
            }),
        }
    }

    /// In-memory config, no file, no hot-reload — for tests.
    pub fn from_config(cfg: KeysConfig) -> Self {
        Self {
            path: None,
            inner: Mutex::new(Inner {
                mtime: None,
                cfg,
                buckets: HashMap::new(),
            }),
        }
    }

    /// Single-key in-memory auth — convenience for tests.
    pub fn single_key(key: &str, user: &str) -> Self {
        let mut keys = HashMap::new();
        keys.insert(
            key.to_string(),
            KeyEntry {
                user: user.to_string(),
                rpm: None,
            },
        );
        Self::from_config(KeysConfig {
            defaults: Defaults::default(),
            keys,
        })
    }

    /// Validate `key` and apply its rate limit, reloading keys.toml first if it changed on disk.
    pub fn check(&self, key: Option<&str>) -> Result<Identity, AuthReject> {
        self.check_at(key, Instant::now())
    }

    fn check_at(&self, key: Option<&str>, now: Instant) -> Result<Identity, AuthReject> {
        let mut g = self.inner.lock().expect("auth lock poisoned");
        if let Some(path) = &self.path {
            let cur = mtime_of(path);
            if cur != g.mtime {
                let (m, c) = load(path);
                g.mtime = m;
                g.cfg = c;
                g.buckets.clear(); // a key-set change resets buckets — cheap and rare
            }
        }
        let key = key.ok_or(AuthReject::Unauthorized)?;
        let entry = match g.cfg.keys.get(key) {
            Some(e) => e.clone(),
            None => return Err(AuthReject::Unauthorized),
        };
        let rpm = entry.rpm.unwrap_or(g.cfg.defaults.rpm);
        let bucket = g
            .buckets
            .entry(key.to_string())
            .or_insert_with(|| Bucket::new(rpm, now));
        bucket.rpm = rpm;
        bucket.take(now).map_err(AuthReject::RateLimited)?;
        Ok(Identity { user: entry.user })
    }
}

fn mtime_of(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

fn load(p: &Path) -> (Option<SystemTime>, KeysConfig) {
    match std::fs::read_to_string(p) {
        Ok(t) => (mtime_of(p), toml::from_str(&t).unwrap_or_default()),
        Err(_) => (None, KeysConfig::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::Duration;

    fn write_keys(body: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("keys.toml");
        std::fs::File::create(&p)
            .unwrap()
            .write_all(body.as_bytes())
            .unwrap();
        (dir, p)
    }

    #[test]
    fn missing_or_unknown_key_is_unauthorized() {
        let (_d, p) = write_keys("[keys.abc]\nuser = \"alice\"\n");
        let a = AuthState::new(p);
        assert!(matches!(a.check(None), Err(AuthReject::Unauthorized)));
        assert!(matches!(
            a.check(Some("nope")),
            Err(AuthReject::Unauthorized)
        ));
    }

    #[test]
    fn known_key_yields_identity() {
        let (_d, p) = write_keys("[keys.abc]\nuser = \"alice\"\n");
        let a = AuthState::new(p);
        assert_eq!(a.check(Some("abc")).unwrap().user, "alice");
    }

    #[test]
    fn rate_limited_after_rpm_in_a_burst() {
        let (_d, p) = write_keys("[keys.abc]\nuser = \"a\"\nrpm = 2\n");
        let a = AuthState::new(p);
        let t = Instant::now();
        assert!(a.check_at(Some("abc"), t).is_ok());
        assert!(a.check_at(Some("abc"), t).is_ok());
        assert!(matches!(
            a.check_at(Some("abc"), t),
            Err(AuthReject::RateLimited(s)) if s >= 1
        ));
    }

    #[test]
    fn bucket_refills_over_time() {
        let (_d, p) = write_keys("[keys.abc]\nuser = \"a\"\nrpm = 60\n");
        let a = AuthState::new(p);
        let t = Instant::now();
        for _ in 0..60 {
            assert!(a.check_at(Some("abc"), t).is_ok());
        }
        assert!(matches!(
            a.check_at(Some("abc"), t),
            Err(AuthReject::RateLimited(_))
        ));
        assert!(a.check_at(Some("abc"), t + Duration::from_secs(1)).is_ok());
    }

    #[test]
    fn defaults_apply_without_override() {
        let (_d, p) = write_keys("[defaults]\nrpm = 1\n[keys.abc]\nuser = \"a\"\n");
        let a = AuthState::new(p);
        let t = Instant::now();
        assert!(a.check_at(Some("abc"), t).is_ok());
        assert!(matches!(
            a.check_at(Some("abc"), t),
            Err(AuthReject::RateLimited(_))
        ));
    }

    #[test]
    fn empty_file_rejects_everything() {
        let (_d, p) = write_keys("");
        let a = AuthState::new(p);
        assert!(matches!(
            a.check(Some("anything")),
            Err(AuthReject::Unauthorized)
        ));
    }
}
