//! Ingest-quality measures (ADR-0006, requirements F-EQ.1 / F-EQ.2).
//!
//! Two cheap, deterministic, dependency-free measures derived from the
//! experiment study (`experiments/quality/` on turbo):
//!   * `classify_section` - decide whether a section is low-value boilerplate
//!     (index, bibliography, ...) that should be skipped before extraction.
//!   * `garble_signal` - a per-document, advisory signal that flags gross
//!     extraction artifacts (replacement characters, letter-spacing). It keys
//!     on definitive garble markers, NOT symbol density, so correctly-extracted
//!     math is never flagged.
//!
//! Both are pure functions with weak pre-conditions (any input, including the
//! empty string, is accepted - EST Ch.4). Per ADR-0004 neither earns a port:
//! there is one implementation, so they live here as domain logic.

use crate::document::{Document, ExtractedText};

// ---- F-EQ.1: low-value section filter --------------------------------------

/// Section-name patterns to exclude / keep. Matching is case-insensitive
/// substring; `keep` wins over `exclude` (so a glossary survives even if
/// "glossary" were listed to exclude).
#[derive(Debug, Clone, Default)]
pub struct SectionConfig {
    pub exclude: Vec<String>,
    pub keep: Vec<String>,
}

/// Whether a section should be indexed, or skipped with a human-readable reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionDecision {
    Index,
    Skip { reason: String },
}

/// Classify a section by its name (chapter / file stem). Testable core.
pub fn classify_name(name: &str, cfg: &SectionConfig) -> SectionDecision {
    let lname = name.to_lowercase();
    if cfg.keep.iter().any(|p| lname.contains(&p.to_lowercase())) {
        return SectionDecision::Index; // keep wins over exclude
    }
    if let Some(pat) = cfg
        .exclude
        .iter()
        .find(|p| lname.contains(&p.to_lowercase()))
    {
        return SectionDecision::Skip {
            reason: format!("low-value section: {pat}"),
        };
    }
    SectionDecision::Index
}

/// Classify a document's section from its file stem. Un-chaptered content
/// (papers) does not match the book front-matter patterns, so it is a no-op.
pub fn classify_section(doc: &Document, cfg: &SectionConfig) -> SectionDecision {
    let name = doc
        .path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    classify_name(&name, cfg)
}

// ---- F-EQ.2: advisory garble signal ----------------------------------------

#[derive(Debug, Clone, Copy, Default)]
pub struct GarbleConfig {
    /// A document is flagged when its composite `value` exceeds this.
    pub flag_above: f64,
}

/// Bundles the F-EQ.1 / F-EQ.2 configuration the runner applies (ADR-0006).
#[derive(Debug, Clone, Default)]
pub struct QualityConfig {
    pub sections: SectionConfig,
    pub garble: GarbleConfig,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GarbleSignal {
    /// U+FFFD replacement characters per 1000 chars.
    pub ufffd_per_kc: f64,
    /// Letter-spacing runs ("P a r t") per 1000 chars.
    pub letterspace_per_kc: f64,
    /// Composite of the above, compared against the threshold.
    pub value: f64,
    pub flagged: bool,
}

/// Count maximal runs of >= 4 consecutive single-letter tokens, e.g. the
/// "P a r t O n e" produced when a PDF renders spaced display text.
fn letterspace_runs(s: &str) -> usize {
    let mut runs = 0;
    let mut streak = 0usize;
    for tok in s.split_whitespace() {
        let single_letter = {
            let mut cs = tok.chars();
            matches!((cs.next(), cs.next()), (Some(c), None) if c.is_alphabetic())
        };
        if single_letter {
            streak += 1;
        } else {
            if streak >= 4 {
                runs += 1;
            }
            streak = 0;
        }
    }
    if streak >= 4 {
        runs += 1;
    }
    runs
}

/// Garble signal over raw text. Testable core.
pub fn garble_text(s: &str, cfg: &GarbleConfig) -> GarbleSignal {
    let chars = s.chars().count().max(1) as f64;
    let ufffd = s.chars().filter(|&c| c == '\u{FFFD}').count() as f64;
    let ufffd_per_kc = 1000.0 * ufffd / chars;
    let letterspace_per_kc = 1000.0 * letterspace_runs(s) as f64 / chars;

    // Composite garble score: the sum of the two independent per-kc markers.
    // Both are zero for clean prose and for correctly-extracted (symbol-dense)
    // math, so symbol density never trips the flag; either marker alone lifts
    // `value`, and it is monotonic in both. The experiment study (E8) found this
    // sum separates garble from good content cleanly at a threshold of ~1.0.
    let value = ufffd_per_kc + letterspace_per_kc;

    GarbleSignal {
        ufffd_per_kc,
        letterspace_per_kc,
        value,
        flagged: value > cfg.flag_above,
    }
}

/// Garble signal over an extracted document (joins its spans).
pub fn garble_signal(text: &ExtractedText, cfg: &GarbleConfig) -> GarbleSignal {
    let joined = text
        .spans
        .iter()
        .map(|sp| sp.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    garble_text(&joined, cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{SpanKind, TextSpan};

    fn cfg() -> SectionConfig {
        SectionConfig {
            exclude: vec![
                "index".into(),
                "bibliography".into(),
                "contents".into(),
                "cover".into(),
                "copyright".into(),
            ],
            keep: vec!["glossary".into(), "notation".into()],
        }
    }

    // ---- F-EQ.1 partitions (EST Ch.2) -------------------------------------
    #[test]
    fn content_section_is_indexed() {
        assert_eq!(
            classify_name("Chapter-04-Gaussian-Models", &cfg()),
            SectionDecision::Index
        );
    }

    #[test]
    fn low_value_section_is_skipped() {
        assert!(matches!(
            classify_name("Index-of-Terms", &cfg()),
            SectionDecision::Skip { .. }
        ));
        assert!(matches!(
            classify_name("Bibliography", &cfg()),
            SectionDecision::Skip { .. }
        ));
    }

    #[test]
    fn keep_wins_over_exclude() {
        let c = SectionConfig {
            exclude: vec!["glossary".into()],
            keep: vec!["glossary".into()],
        };
        assert_eq!(classify_name("Glossary", &c), SectionDecision::Index);
    }

    #[test]
    fn classification_is_case_insensitive() {
        assert!(matches!(
            classify_name("BIBLIOGRAPHY", &cfg()),
            SectionDecision::Skip { .. }
        ));
    }

    #[test]
    fn unmatched_paper_name_is_indexed() {
        // un-chaptered paper names do not match book front-matter -> no-op
        assert_eq!(
            classify_name("Aglagul2022-a-simple-approach-for-characterizing", &cfg()),
            SectionDecision::Index
        );
    }

    #[test]
    fn empty_config_indexes_everything() {
        assert_eq!(
            classify_name("Index", &SectionConfig::default()),
            SectionDecision::Index
        );
    }

    // ---- letterspace_runs boundary (EST on/off point at 4) ----------------
    #[test]
    fn letterspace_boundary_at_four() {
        assert_eq!(letterspace_runs("a b c"), 0); // 3 singles: off point
        assert_eq!(letterspace_runs("a b c d"), 1); // 4 singles: on point
        assert_eq!(letterspace_runs("P a r t O n e"), 1);
        assert_eq!(letterspace_runs("the quick brown fox"), 0); // multi-letter words
    }

    // ---- F-EQ.2 measurements (pass now) -----------------------------------
    #[test]
    fn clean_prose_has_zero_signals_and_is_not_flagged() {
        let s = garble_text(
            "The quick brown fox jumps over the lazy dog.",
            &GarbleConfig { flag_above: 1.0 },
        );
        assert_eq!(s.ufffd_per_kc, 0.0);
        assert_eq!(s.letterspace_per_kc, 0.0);
        assert!(!s.flagged);
    }

    #[test]
    fn mojibake_is_measured() {
        let s = garble_text(
            "normal text with \u{FFFD}\u{FFFD}\u{FFFD} bad glyphs",
            &GarbleConfig { flag_above: 1.0 },
        );
        assert!(s.ufffd_per_kc > 0.0);
    }

    #[test]
    fn empty_input_is_safe_and_unflagged() {
        let s = garble_text("", &GarbleConfig { flag_above: 1.0 });
        assert!(!s.flagged);
    }

    #[test]
    fn signal_joins_spans() {
        let t = ExtractedText {
            spans: vec![TextSpan {
                kind: SpanKind::Paragraph,
                text: "P a r t O n e".into(),
                page: None,
                byte_range: 0..0,
            }],
        };
        let s = garble_signal(&t, &GarbleConfig { flag_above: 1.0 });
        assert!(s.letterspace_per_kc > 0.0);
    }

    // ---- F-EQ.2 flag behaviour (RED until the TODO(human) composite exists) -
    #[test]
    fn mojibake_is_flagged() {
        let s = garble_text(
            "text \u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD} more",
            &GarbleConfig { flag_above: 1.0 },
        );
        assert!(
            s.flagged,
            "documents with replacement chars must be flagged"
        );
    }

    #[test]
    fn letterspacing_is_flagged() {
        let s = garble_text(
            "P a r t O n e I n t r o d u c t i o n",
            &GarbleConfig { flag_above: 1.0 },
        );
        assert!(s.flagged, "letter-spaced text must be flagged");
    }

    #[test]
    fn math_dense_is_not_flagged() {
        // correctly-extracted math is symbol-dense but NOT garbled.
        let math = "$$\\mathcal{N}(\\mathbf{x}|\\boldsymbol{\\mu},\\boldsymbol{\\Sigma}) \
                    \\triangleq \\frac{1}{(2\\pi)^{D/2}}$$ Recall that $\\Gamma \\vdash e : \\tau$.";
        let s = garble_text(math, &GarbleConfig { flag_above: 1.0 });
        assert!(!s.flagged, "symbol/math density must NOT trigger the flag");
    }
}
