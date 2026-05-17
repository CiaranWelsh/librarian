#[derive(Debug, thiserror::Error)]
pub enum OpenAiBuildError {
    #[error("OPENAI_API_KEY missing")]
    MissingApiKey,
    #[error("http: {0}")]
    Http(#[source] reqwest::Error),
}
