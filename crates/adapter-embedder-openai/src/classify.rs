//! HTTP-status → `EmbedderError` classification (slice 010 AC).
//! 5xx / 408 / 429 are recoverable so the `FallbackEmbedder` will retry;
//! everything else is terminal.

use librarian_domain::EmbedderError;

pub(crate) fn classify(status: u16, body: &str) -> EmbedderError {
    match status {
        408 | 429 | 500..=599 => {
            EmbedderError::Recoverable(format!("http {status}: {}", truncate(body)))
        }
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
    fn classify_5xx_is_recoverable() {
        assert!(matches!(classify(503, ""), EmbedderError::Recoverable(_)));
        assert!(matches!(classify(500, ""), EmbedderError::Recoverable(_)));
    }

    #[test]
    fn classify_429_is_recoverable() {
        assert!(matches!(classify(429, ""), EmbedderError::Recoverable(_)));
    }

    #[test]
    fn classify_4xx_other_is_terminal() {
        assert!(matches!(classify(401, ""), EmbedderError::Terminal(_)));
        assert!(matches!(classify(400, ""), EmbedderError::Terminal(_)));
        assert!(matches!(classify(404, ""), EmbedderError::Terminal(_)));
    }

    #[test]
    fn truncate_caps_long_bodies() {
        let long = "a".repeat(500);
        assert!(truncate(&long).len() <= 250);
        assert_eq!(truncate("short"), "short");
    }
}
