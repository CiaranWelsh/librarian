//! HTTP-status classification — same policy as the OpenAI adapter.

use librarian_domain::EmbedderError;

pub(crate) fn classify(status: u16, body: &str) -> EmbedderError {
    match status {
        408 | 429 | 500..=599 => EmbedderError::Recoverable(format!("http {status}: {}", truncate(body))),
        _ => EmbedderError::Terminal(format!("http {status}: {}", truncate(body))),
    }
}

pub(crate) fn truncate(s: &str) -> String {
    if s.len() <= 200 { s.to_string() } else { format!("{}…", &s[..200]) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_5xx_recoverable_4xx_terminal_429_recoverable() {
        assert!(matches!(classify(503, ""), EmbedderError::Recoverable(_)));
        assert!(matches!(classify(429, ""), EmbedderError::Recoverable(_)));
        assert!(matches!(classify(401, ""), EmbedderError::Terminal(_)));
        assert!(matches!(classify(400, ""), EmbedderError::Terminal(_)));
    }
}
