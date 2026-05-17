#[derive(Debug, thiserror::Error)]
pub enum SqliteManifestError {
    #[error("io: {0}")]
    Io(#[source] std::io::Error),
    #[error("db: {0}")]
    Db(#[source] rusqlite::Error),
}
