//! Free functions over `SqliteManifest` that are useful outside the
//! `ManifestStore` trait — e.g. the MCP server's `list_documents` and tests.

use librarian_domain::{CacheKey, ManifestStatus, SourceId};
use rusqlite::{params, OptionalExtension};

use crate::error::SqliteManifestError;
use crate::manifest::SqliteManifest;
use crate::status::parse_status;

/// Distinct source_ids that have at least one Success / Cached /
/// RecoveredViaFallback row. Used by the MCP server's `list_documents` (slice 013).
pub fn distinct_ingested_sources(
    store: &SqliteManifest,
) -> Result<Vec<SourceId>, SqliteManifestError> {
    let g = store.conn.lock().expect("poisoned");
    let mut stmt = g
        .prepare(
            "SELECT DISTINCT source_id FROM manifest \
             WHERE status IN ('Success', 'Cached', 'RecoveredViaFallback')",
        )
        .map_err(SqliteManifestError::Db)?;
    let rows = stmt
        .query_map([], |r| Ok(SourceId(r.get::<_, String>(0)?)))
        .map_err(SqliteManifestError::Db)?;
    let mut out = Vec::new();
    for r in rows { out.push(r.map_err(SqliteManifestError::Db)?); }
    Ok(out)
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
