//! `librarian audit` — read-only quality audit of an already-ingested
//! collection (ADR-0006). Replays the F-EQ.1 / F-EQ.2 signals over each
//! document's cached `extract` output and reports low-value sections and
//! garble-flagged documents, so an operator can decide what to `remove`.
//! Touches no external services and writes nothing.

use std::path::Path;

use adapter_cache_fs::FsCache;
use adapter_manifest_sqlite::SqliteManifest;
use librarian_domain::{
    classify_name, garble_signal, Cache, ExtractedText, QualityConfig, SectionDecision,
};

use crate::config::Config;

#[derive(Default)]
pub struct AuditReport {
    pub total: usize,
    pub low_value: Vec<(String, String)>, // (source_id, reason)
    pub flagged: Vec<(String, f64)>,      // (source_id, garble value)
    pub clean: usize,
    pub unreadable: usize, // cache miss or parse failure
}

/// Pure core: bucket a stream of `(source_id, extracted text)` against the
/// quality policy. The section role is decided from the source's file stem,
/// exactly as the ingest-time gate does.
pub fn audit_docs<I>(docs: I, q: &QualityConfig) -> AuditReport
where
    I: IntoIterator<Item = (String, Option<ExtractedText>)>,
{
    let mut r = AuditReport::default();
    for (sid, text) in docs {
        r.total += 1;
        let stem = Path::new(&sid)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| sid.clone());
        if let SectionDecision::Skip { reason } = classify_name(&stem, &q.sections) {
            r.low_value.push((sid, reason));
            continue;
        }
        match text {
            Some(t) => {
                let g = garble_signal(&t, &q.garble);
                if g.flagged {
                    r.flagged.push((sid, g.value));
                } else {
                    r.clean += 1;
                }
            }
            None => r.unreadable += 1,
        }
    }
    r
}

pub fn cmd_audit(config_path: &Path) -> Result<(), String> {
    let cfg = Config::load(config_path).map_err(|e| e.to_string())?;
    let q = cfg.quality.to_domain();
    let manifest = SqliteManifest::open(&cfg.paths.manifest).map_err(|e| e.to_string())?;
    let cache = FsCache::open(&cfg.paths.cache).map_err(|e| e.to_string())?;

    let outputs = manifest
        .list_outputs("extract")
        .map_err(|e| e.to_string())?;
    let docs = outputs.into_iter().map(|(sid, key)| {
        let text = cache
            .get(&key)
            .ok()
            .flatten()
            .and_then(|bytes| serde_json::from_slice::<ExtractedText>(&bytes).ok());
        (sid.0, text)
    });

    let report = audit_docs(docs, &q);
    let pct = |n: usize| {
        if report.total > 0 {
            100.0 * n as f64 / report.total as f64
        } else {
            0.0
        }
    };

    println!(
        "audit: collection={}  documents={}",
        cfg.collection, report.total
    );
    println!(
        "  low-value sections : {:4} ({:.1}%)  [remove candidates]",
        report.low_value.len(),
        pct(report.low_value.len())
    );
    println!(
        "  garble-flagged     : {:4} ({:.1}%)  [review]",
        report.flagged.len(),
        pct(report.flagged.len())
    );
    println!(
        "  clean              : {:4} ({:.1}%)",
        report.clean,
        pct(report.clean)
    );
    if report.unreadable > 0 {
        println!(
            "  unreadable         : {:4}  (no cache / parse failure)",
            report.unreadable
        );
    }

    if !report.low_value.is_empty() {
        println!("\n-- low-value sections (remove candidates) --");
        for (sid, reason) in &report.low_value {
            println!("  {reason}\t{sid}");
        }
    }
    if !report.flagged.is_empty() {
        println!("\n-- garble-flagged (review) --");
        let mut f = report.flagged.clone();
        f.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (sid, value) in &f {
            println!("  value={value:8.3}\t{sid}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::{GarbleConfig, SectionConfig, SpanKind, TextSpan};

    fn span(text: &str) -> ExtractedText {
        ExtractedText {
            spans: vec![TextSpan {
                kind: SpanKind::Paragraph,
                text: text.into(),
                page: None,
                byte_range: 0..text.len(),
            }],
        }
    }

    fn q() -> QualityConfig {
        QualityConfig {
            sections: SectionConfig {
                exclude: vec!["index".into()],
                keep: vec!["glossary".into()],
            },
            garble: GarbleConfig { flag_above: 1.0 },
        }
    }

    #[test]
    fn audit_buckets_docs_by_quality() {
        let docs = vec![
            ("/c/Index-of-Terms.pdf".into(), Some(span("a, 1\nb, 2"))), // low-value (name)
            (
                "/c/Chapter-01.pdf".into(),
                Some(span("text \u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD} x")),
            ), // flagged (mojibake)
            (
                "/c/Chapter-02.pdf".into(),
                Some(span("perfectly clean prose")),
            ), // clean
            (
                "/c/Glossary.pdf".into(),
                Some(span("a clean glossary entry")),
            ), // kept -> clean
            ("/c/Chapter-03.pdf".into(), None),                         // unreadable
        ];
        let r = audit_docs(docs, &q());
        assert_eq!(r.total, 5);
        assert_eq!(r.low_value.len(), 1);
        assert_eq!(r.flagged.len(), 1);
        assert_eq!(r.clean, 2); // Chapter-02 + Glossary (kept)
        assert_eq!(r.unreadable, 1);
    }
}
