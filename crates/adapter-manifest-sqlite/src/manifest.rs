//! SQLite-backed `ManifestStore`. One file per collection; one row per
//! `(source_id, stage)` — `record` upserts.

use librarian_domain::{CacheKey, ManifestStatus, ManifestStore, SourceId};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::error::SqliteManifestError;
use crate::status::status_str;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS manifest (
    source_id   TEXT NOT NULL,
    stage       TEXT NOT NULL,
    status      TEXT NOT NULL,
    attempts    INTEGER NOT NULL DEFAULT 0,
    error       TEXT,
    output_ref  TEXT,
    updated_at  INTEGER NOT NULL,
    PRIMARY KEY (source_id, stage)
);
CREATE INDEX IF NOT EXISTS idx_manifest_status ON manifest(status);
CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);
INSERT OR IGNORE INTO schema_version(version) VALUES (1);
"#;

pub struct SqliteManifest {
    pub(crate) conn: Mutex<Connection>,
    #[allow(dead_code)]
    path: PathBuf,
}

impl SqliteManifest {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, SqliteManifestError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(SqliteManifestError::Io)?;
        }
        let conn = Connection::open(&path).map_err(SqliteManifestError::Db)?;
        conn.execute_batch(SCHEMA).map_err(SqliteManifestError::Db)?;
        Ok(Self { conn: Mutex::new(conn), path })
    }

    pub fn schema_version(&self) -> Result<i64, SqliteManifestError> {
        let g = self.conn.lock().expect("poisoned");
        let v: i64 = g
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .map_err(SqliteManifestError::Db)?;
        Ok(v)
    }
}

impl ManifestStore for SqliteManifest {
    type Error = SqliteManifestError;

    fn record(
        &self,
        source_id: &SourceId,
        stage: &str,
        status: ManifestStatus,
        attempts: u32,
        error: Option<&str>,
        output_ref: Option<&CacheKey>,
    ) -> Result<(), Self::Error> {
        let g = self.conn.lock().expect("poisoned");
        let now = unix_now();
        g.execute(
            "INSERT INTO manifest(source_id, stage, status, attempts, error, output_ref, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(source_id, stage) DO UPDATE SET
               status     = excluded.status,
               attempts   = excluded.attempts,
               error      = excluded.error,
               output_ref = excluded.output_ref,
               updated_at = excluded.updated_at",
            params![
                source_id.0,
                stage,
                status_str(status),
                attempts as i64,
                error,
                output_ref.map(|k| k.0.as_str()),
                now,
            ],
        )
        .map_err(SqliteManifestError::Db)?;
        Ok(())
    }

    fn list_by_status(
        &self,
        status: ManifestStatus,
    ) -> Result<Vec<(SourceId, String)>, Self::Error> {
        let g = self.conn.lock().expect("poisoned");
        let mut stmt = g
            .prepare("SELECT source_id, stage FROM manifest WHERE status = ?1")
            .map_err(SqliteManifestError::Db)?;
        let rows = stmt
            .query_map([status_str(status)], |r| {
                Ok((SourceId(r.get::<_, String>(0)?), r.get::<_, String>(1)?))
            })
            .map_err(SqliteManifestError::Db)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(SqliteManifestError::Db)?);
        }
        Ok(out)
    }
}

/// Seconds since UNIX epoch. Avoids pulling chrono just for `updated_at`.
fn unix_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
