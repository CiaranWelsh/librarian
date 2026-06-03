//! `ManifestStatus` ↔ SQL string conversion. Kept in one place so the SQL
//! literals in `queries.rs` agree with the writer path.

use librarian_domain::ManifestStatus;

pub(crate) fn status_str(s: ManifestStatus) -> &'static str {
    match s {
        ManifestStatus::Pending => "Pending",
        ManifestStatus::Success => "Success",
        ManifestStatus::Cached => "Cached",
        ManifestStatus::Failed => "Failed",
        ManifestStatus::RecoveredViaFallback => "RecoveredViaFallback",
        ManifestStatus::Skipped => "Skipped",
        ManifestStatus::Flagged => "Flagged",
        ManifestStatus::Removed => "Removed",
    }
}

pub(crate) fn parse_status(s: &str) -> Option<ManifestStatus> {
    Some(match s {
        "Pending" => ManifestStatus::Pending,
        "Success" => ManifestStatus::Success,
        "Cached" => ManifestStatus::Cached,
        "Failed" => ManifestStatus::Failed,
        "RecoveredViaFallback" => ManifestStatus::RecoveredViaFallback,
        "Skipped" => ManifestStatus::Skipped,
        "Flagged" => ManifestStatus::Flagged,
        "Removed" => ManifestStatus::Removed,
        _ => return None,
    })
}
