#[derive(Debug, thiserror::Error)]
pub enum VoyageBuildError {
    #[error("VOYAGE_API_KEY missing")]
    MissingApiKey,
    #[error("http: {0}")]
    Http(#[source] reqwest::Error),
}
