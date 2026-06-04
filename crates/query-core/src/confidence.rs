//! Tier 0 RAG quality (issue 028): a cheap, reference-free per-query retrieval-confidence
//! signal derived from the top-k cosine score distribution (Query Performance Prediction:
//! NQC-style spread, top-1, top1-top2 margin) plus chunk substance (fragment-rate).
//!
//! Honest scope: for a dense vector store, score-based QPP correlates only ~0.2-0.4 with true
//! quality (see `docs/research/rag-quality/FINDINGS.md`), so this is a **triage / abstain**
//! signal, not a precise grade. The raw signals are exposed so callers aren't reliant on the
//! composite, and the label thresholds are documented defaults to calibrate against the golden
//! set (Tier 2).

use librarian_domain::Hit;

/// Coarse triage label for a query's retrieval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceLabel {
    /// A close, distinguishable, substantial top hit — trust it.
    Strong,
    /// Something matched, but weakly or only as fragments — read with care.
    Weak,
    /// Nothing close — the query is likely out-of-corpus; consider abstaining.
    LikelyNoAnswer,
}

/// Per-query retrieval confidence and the raw signals behind it.
#[derive(Debug, Clone)]
pub struct RetrievalConfidence {
    /// 0-1 composite: top closeness discounted by fragment-rate. Triage, not a precise grade.
    pub value: f32,
    pub label: ConfidenceLabel,
    /// Top-1 cosine score — absolute closeness; doubles as the out-of-corpus signal.
    pub top_score: f32,
    /// s1 - s2: distinguishability of the best hit (0 if fewer than two hits).
    pub margin: f32,
    /// Std-dev of the top-k scores (NQC-style spread; higher = more distinguishable).
    pub score_spread: f32,
    /// Fraction of the top-5 hits that are fragments (<80 chars or a bare heading).
    pub fragment_rate: f32,
}

/// Tunable thresholds for the triage label. Defaults are starting points for OpenAI
/// `text-embedding-3-large` cosine scores; calibrate against the golden set (Tier 2).
#[derive(Debug, Clone, Copy)]
pub struct ConfidenceThresholds {
    /// Top-1 below this → `LikelyNoAnswer` (out-of-corpus).
    pub no_answer_below: f32,
    /// Top-1 at/above this (with margin + low fragments) → `Strong`.
    pub strong_top: f32,
    pub strong_margin: f32,
    pub max_fragment_rate: f32,
}

impl Default for ConfidenceThresholds {
    fn default() -> Self {
        Self {
            no_answer_below: 0.25,
            strong_top: 0.40,
            strong_margin: 0.03,
            max_fragment_rate: 0.40,
        }
    }
}

/// Same fragment heuristic the offline eval uses (`eval/run_eval.py`): a hit is a "fragment"
/// if it's under 80 characters or a bare markdown heading. Tier 2 (`librarian health`) mirrors
/// this exact definition so online and offline measurements agree.
fn is_fragment(text: &str) -> bool {
    text.chars().count() < 80 || text.trim_start().starts_with('#')
}

fn std_dev(xs: &[f32]) -> f32 {
    let n = xs.len();
    if n == 0 {
        return 0.0;
    }
    let mean = xs.iter().sum::<f32>() / n as f32;
    let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / n as f32;
    var.sqrt()
}

/// Compute retrieval confidence with the default thresholds.
pub fn retrieval_confidence(hits: &[Hit]) -> RetrievalConfidence {
    retrieval_confidence_with(hits, &ConfidenceThresholds::default())
}

/// Compute retrieval confidence with explicit thresholds.
pub fn retrieval_confidence_with(hits: &[Hit], t: &ConfidenceThresholds) -> RetrievalConfidence {
    if hits.is_empty() {
        return RetrievalConfidence {
            value: 0.0,
            label: ConfidenceLabel::LikelyNoAnswer,
            top_score: 0.0,
            margin: 0.0,
            score_spread: 0.0,
            fragment_rate: 0.0,
        };
    }
    let scores: Vec<f32> = hits.iter().map(|h| h.score).collect();
    let top_score = scores[0];
    let margin = if scores.len() >= 2 {
        scores[0] - scores[1]
    } else {
        0.0
    };
    let score_spread = std_dev(&scores);
    let top5 = &hits[..hits.len().min(5)];
    let fragment_rate =
        top5.iter().filter(|h| is_fragment(&h.text)).count() as f32 / top5.len() as f32;

    // Composite (documented heuristic): top closeness, discounted up to 50% by junk hits.
    let value = (top_score * (1.0 - 0.5 * fragment_rate)).clamp(0.0, 1.0);

    let label = if top_score < t.no_answer_below {
        ConfidenceLabel::LikelyNoAnswer
    } else if top_score >= t.strong_top
        && margin >= t.strong_margin
        && fragment_rate <= t.max_fragment_rate
    {
        ConfidenceLabel::Strong
    } else {
        ConfidenceLabel::Weak
    };

    RetrievalConfidence {
        value,
        label,
        top_score,
        margin,
        score_spread,
        fragment_rate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::SourceId;

    fn hit(score: f32, text: &str) -> Hit {
        Hit {
            score,
            source_id: SourceId("s".into()),
            content_type: "book".into(),
            chunk_index: 0,
            text: text.into(),
        }
    }

    #[test]
    fn signals_match_known_score_vector() {
        let body = "x".repeat(200);
        let hits = vec![hit(0.5, &body), hit(0.4, &body), hit(0.3, &body)];
        let c = retrieval_confidence(&hits);
        assert!((c.top_score - 0.5).abs() < 1e-6);
        assert!((c.margin - 0.1).abs() < 1e-6, "margin {}", c.margin);
        assert!(
            (c.score_spread - 0.081_649_66).abs() < 1e-3,
            "std {}",
            c.score_spread
        );
        assert_eq!(c.fragment_rate, 0.0);
    }

    #[test]
    fn strong_when_close_distinct_and_substantial() {
        let body = "x".repeat(200);
        let hits = vec![hit(0.55, &body), hit(0.45, &body)];
        assert_eq!(retrieval_confidence(&hits).label, ConfidenceLabel::Strong);
    }

    #[test]
    fn likely_no_answer_when_top_score_low() {
        let body = "x".repeat(200);
        let hits = vec![hit(0.18, &body), hit(0.17, &body)];
        assert_eq!(
            retrieval_confidence(&hits).label,
            ConfidenceLabel::LikelyNoAnswer
        );
    }

    #[test]
    fn weak_and_discounted_when_hits_are_fragments() {
        // Close top score, but the hits are bare headings → high fragment-rate → Weak, value down.
        let hits = vec![hit(0.55, "# Heading"), hit(0.50, "## Another heading")];
        let c = retrieval_confidence(&hits);
        assert!(c.fragment_rate > 0.9, "frag {}", c.fragment_rate);
        assert_eq!(c.label, ConfidenceLabel::Weak);
        assert!(c.value < 0.55, "value {} should be discounted", c.value);
    }

    #[test]
    fn empty_hits_is_no_answer() {
        let c = retrieval_confidence(&[]);
        assert_eq!(c.label, ConfidenceLabel::LikelyNoAnswer);
        assert_eq!(c.value, 0.0);
    }
}
