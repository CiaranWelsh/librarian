use std::time::Duration;

#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    pub model: String,
    pub dimensions: usize,
    pub endpoint: Option<String>,
    pub batch_size: Option<usize>,
    pub timeout: Option<Duration>,
}
