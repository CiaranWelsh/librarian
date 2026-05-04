//! SQLite-backed `ManifestStore`. One file per collection. One row per
//! `(source_id, stage)` — `record` upserts.

use librarian_domain::{CacheKey, ManifestStatus, ManifestStore, SourceId};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::Mutex;

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
    conn: Mutex<Connection>,
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

#[derive(Debug, thiserror::Error)]
pub enum SqliteManifestError {
    #[error("io: {0}")]
    Io(#[source] std::io::Error),
    #[error("db: {0}")]
    Db(#[source] rusqlite::Error),
}

fn status_str(s: ManifestStatus) -> &'static str {
    match s {
        ManifestStatus::Pending => "Pending",
        ManifestStatus::Success => "Success",
        ManifestStatus::Cached => "Cached",
        ManifestStatus::Failed => "Failed",
        ManifestStatus::RecoveredViaFallback => "RecoveredViaFallback",
        ManifestStatus::Skipped => "Skipped",
        ManifestStatus::Removed => "Removed",
    }
}

fn parse_status(s: &str) -> Option<ManifestStatus> {
    Some(match s {
        "Pending" => ManifestStatus::Pending,
        "Success" => ManifestStatus::Success,
        "Cached" => ManifestStatus::Cached,
        "Failed" => ManifestStatus::Failed,
        "RecoveredViaFallback" => ManifestStatus::RecoveredViaFallback,
        "Skipped" => ManifestStatus::Skipped,
        "Removed" => ManifestStatus::Removed,
        _ => return None,
    })
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
        let now = chrono_now();
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

/// Minimal "now" for `updated_at` — seconds since epoch. Avoids pulling chrono
/// here; manifest doesn't expose timestamps externally yet.
fn chrono_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Look up a single row by primary key — used by tests and downstream queries.
pub fn get_row(
    store: &SqliteManifest,
    source_id: &SourceId,
    stage: &str,
) -> Result<Option<Row>, SqliteManifestError> {
    let g = store.conn.lock().expect("poisoned");
    let row = g
        .query_row(
            "SELECT source_id, stage, status, attempts, error, output_ref
             FROM manifest WHERE source_id = ?1 AND stage = ?2",
            params![source_id.0, stage],
            |r| {
                Ok(Row {
                    source_id: SourceId(r.get(0)?),
                    stage: r.get(1)?,
                    status: parse_status(&r.get::<_, String>(2)?).unwrap_or(ManifestStatus::Pending),
                    attempts: r.get::<_, i64>(3)? as u32,
                    error: r.get(4)?,
                    output_ref: r.get::<_, Option<String>>(5)?.map(CacheKey),
                })
            },
        )
        .optional()
        .map_err(SqliteManifestError::Db)?;
    Ok(row)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub source_id: SourceId,
    pub stage: String,
    pub status: ManifestStatus,
    pub attempts: u32,
    pub error: Option<String>,
    pub output_ref: Option<CacheKey>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn fresh() -> (tempfile::TempDir, SqliteManifest) {
        let d = tempdir().unwrap();
        let m = SqliteManifest::open(d.path().join("m.sqlite")).unwrap();
        (d, m)
    }

    fn sid(s: &str) -> SourceId { SourceId(s.into()) }

    #[test]
    fn migration_runs_clean() {
        let (_d, m) = fresh();
        assert_eq!(m.schema_version().unwrap(), 1);
    }

    #[test]
    fn record_upserts_on_source_id_stage_pair() {
        let (_d, m) = fresh();
        m.record(&sid("a"), "extract", ManifestStatus::Pending, 0, None, None).unwrap();
        m.record(&sid("a"), "extract", ManifestStatus::Success, 1, None, None).unwrap();
        // exactly one row, with the latest values
        let row = get_row(&m, &sid("a"), "extract").unwrap().unwrap();
        assert_eq!(row.status, ManifestStatus::Success);
        assert_eq!(row.attempts, 1);
        assert_eq!(m.list_by_status(ManifestStatus::Success).unwrap().len(), 1);
        assert_eq!(m.list_by_status(ManifestStatus::Pending).unwrap().len(), 0);
    }

    #[test]
    fn list_by_status_empty_when_no_matches() {
        let (_d, m) = fresh();
        assert_eq!(m.list_by_status(ManifestStatus::Failed).unwrap(), vec![]);
    }

    #[test]
    fn round_trip_preserves_each_status_variant() {
        let (_d, m) = fresh();
        let all = [
            ManifestStatus::Pending, ManifestStatus::Success, ManifestStatus::Cached,
            ManifestStatus::Failed, ManifestStatus::RecoveredViaFallback,
            ManifestStatus::Skipped, ManifestStatus::Removed,
        ];
        for (i, s) in all.iter().enumerate() {
            m.record(&sid(&format!("d{i}")), "extract", *s, 0, None, None).unwrap();
        }
        for s in all {
            let listed = m.list_by_status(s).unwrap();
            assert_eq!(listed.len(), 1, "status {s:?}");
        }
    }

    #[test]
    fn error_and_output_ref_persist_when_set_or_null() {
        let (_d, m) = fresh();
        m.record(&sid("a"), "extract", ManifestStatus::Failed, 2, Some("boom"), None).unwrap();
        let r = get_row(&m, &sid("a"), "extract").unwrap().unwrap();
        assert_eq!(r.error.as_deref(), Some("boom"));
        assert_eq!(r.output_ref, None);

        let key = CacheKey("k".repeat(64));
        m.record(&sid("b"), "embed", ManifestStatus::Cached, 0, None, Some(&key)).unwrap();
        let r = get_row(&m, &sid("b"), "embed").unwrap().unwrap();
        assert_eq!(r.error, None);
        assert_eq!(r.output_ref, Some(key));
    }

    #[test]
    fn distinct_stages_for_same_source_coexist() {
        let (_d, m) = fresh();
        m.record(&sid("a"), "extract", ManifestStatus::Success, 1, None, None).unwrap();
        m.record(&sid("a"), "embed", ManifestStatus::Failed, 1, Some("x"), None).unwrap();
        assert_eq!(m.list_by_status(ManifestStatus::Success).unwrap().len(), 1);
        assert_eq!(m.list_by_status(ManifestStatus::Failed).unwrap().len(), 1);
    }

    #[test]
    fn open_creates_parent_directory() {
        let d = tempdir().unwrap();
        let nested = d.path().join("a/b/m.sqlite");
        let _ = SqliteManifest::open(&nested).unwrap();
        assert!(nested.exists());
    }
}
