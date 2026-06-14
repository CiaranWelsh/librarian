//! `librarian add` preview core — the intrinsic-quality verdict for ONE new
//! resource, computed before anything is written. Mirrors `audit.rs`'s
//! `audit_docs` (a pure core + a thin command wrapper), but scoped to a single
//! resource's extracted text and chunks rather than a whole ingested collection.
//!
//! Gate 1 (this task) is the garble check: a garbled extraction aborts the add
//! (QR-2). The section decision and fragment rate are reported as advisories —
//! a whole-book slug is not boilerplate, so `Skip` does not by itself fail the
//! gate.

use librarian_domain::{
    classify_name, garble_signal, Chunk, ExtractedText, GarbleSignal, QualityConfig,
    SectionDecision,
};

use crate::commands::health::is_fragment;
use crate::commands::output::Render;

use super::plan::AddPlan;

pub struct PreviewVerdict {
    pub chunks: usize,
    /// Fraction of chunks that are fragments (0.0 when there are no chunks).
    pub fragment_rate: f64,
    pub garble: GarbleSignal,
    pub section: SectionDecision,
    /// Gate 1 passes when the extraction is not garble-flagged (QR-2).
    pub gate1_pass: bool,
}

/// Pure: compute the intrinsic-quality verdict for one resource's extracted
/// text and chunks. The section role is decided from `section_name` (the
/// file stem / slug), exactly as the ingest-time gate does.
pub fn preview_quality(
    section_name: &str,
    text: &ExtractedText,
    chunks: &[Chunk],
    q: &QualityConfig,
) -> PreviewVerdict {
    let fragment_rate = if chunks.is_empty() {
        0.0
    } else {
        let frags = chunks.iter().filter(|c| is_fragment(&c.text)).count();
        frags as f64 / chunks.len() as f64
    };
    let garble = garble_signal(text, &q.garble);
    let section = classify_name(section_name, &q.sections);
    PreviewVerdict {
        chunks: chunks.len(),
        fragment_rate,
        gate1_pass: !garble.flagged,
        garble,
        section,
    }
}

/// Render the plan + verdict for the operator. Shared between the preview (no
/// write) and the commit path (same verdict, before the write happens). The
/// `committing` flag keeps the final gate-1 line honest: a dry-run says nothing
/// was written, a commit says it is proceeding to write.
pub(crate) fn render_preview(r: Render, plan: &AddPlan, v: &PreviewVerdict, committing: bool) {
    if r.json {
        let val = serde_json::json!({
            "slug": plan.slug,
            "kind": format!("{:?}", plan.kind),
            "raw_path": plan.raw_path,
            "ingest_path": plan.ingest_path,
            "source_id_prefix": plan.source_id_prefix,
            "config_path": plan.config_path,
            "chunks": v.chunks,
            "fragment_rate": v.fragment_rate,
            "garble": {
                "value": v.garble.value,
                "ufffd_per_kc": v.garble.ufffd_per_kc,
                "letterspace_per_kc": v.garble.letterspace_per_kc,
                "flagged": v.garble.flagged,
            },
            "section": section_label(v),
            "gate1_pass": v.gate1_pass,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&val).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
        );
        return;
    }

    println!("add preview: slug={}  kind={:?}", plan.slug, plan.kind);
    println!("  raw path   : {}", plan.raw_path.display());
    println!("  ingest path: {}", plan.ingest_path.display());
    println!("  source id  : {}", plan.source_id_prefix);
    println!("  config     : {}", plan.config_path.display());
    println!(
        "  chunks     : {}  fragment-rate {:.1}%",
        v.chunks,
        v.fragment_rate * 100.0
    );
    println!(
        "  garble     : value={:.3}  flagged={}",
        v.garble.value, v.garble.flagged
    );
    println!("  section    : {}", section_label(v));

    if let SectionDecision::Skip { reason } = &v.section {
        println!(
            "  note       : section classifier says \"{reason}\" (advisory; does not block add)"
        );
    }

    let line = match (v.gate1_pass, committing) {
        (true, true) => "gate 1 (garble) PASSED -- committing",
        (true, false) => "PREVIEW: gate 1 (garble) PASSED -- nothing was written (preview only)",
        (false, _) => "PREVIEW: gate 1 (garble) FAILED -- looks garbled",
    };
    println!("{line}");
}

fn section_label(v: &PreviewVerdict) -> String {
    match &v.section {
        SectionDecision::Index => "index".to_string(),
        SectionDecision::Skip { reason } => format!("skip ({reason})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_domain::{
        BookMeta, ChunkId, ChunkPayload, GarbleConfig, Provenance, SectionConfig, SourceId,
        SpanKind, TextSpan,
    };

    fn text(s: &str) -> ExtractedText {
        ExtractedText {
            spans: vec![TextSpan {
                kind: SpanKind::Paragraph,
                text: s.into(),
                page: None,
                byte_range: 0..s.len(),
            }],
        }
    }

    fn chunk(idx: u32, body: &str) -> Chunk {
        Chunk {
            chunk_id: ChunkId(format!("c{idx}")),
            source_id: SourceId("s".into()),
            chunk_index: idx,
            text: body.into(),
            payload: ChunkPayload::Book(BookMeta {
                title: "t".into(),
                author: None,
                chapter: None,
                section: None,
                page: None,
            }),
            provenance: Provenance::default(),
        }
    }

    fn q() -> QualityConfig {
        QualityConfig {
            sections: SectionConfig {
                exclude: vec!["index".into()],
                keep: vec![],
            },
            garble: GarbleConfig { flag_above: 1.0 },
        }
    }

    #[test]
    fn clean_text_passes_gate1_with_no_fragments() {
        let body = "a".repeat(200);
        let chunks = vec![chunk(0, &body), chunk(1, &body), chunk(2, &body)];
        let v = preview_quality("programming-rust", &text("clean prose"), &chunks, &q());
        assert!(v.gate1_pass);
        assert!(!v.garble.flagged);
        assert_eq!(v.fragment_rate, 0.0);
        assert_eq!(v.chunks, 3);
    }

    #[test]
    fn garbled_text_fails_gate1() {
        let body = "a".repeat(200);
        let chunks = vec![chunk(0, &body)];
        let garbled = text("text \u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD} more");
        let v = preview_quality("some-book", &garbled, &chunks, &q());
        assert!(!v.gate1_pass);
        assert!(v.garble.flagged);
    }

    #[test]
    fn fragment_rate_is_the_fraction_of_short_chunks() {
        let long = "a".repeat(200);
        // 1 of 4 chunks is a fragment (under 80 chars) -> 0.25.
        let chunks = vec![
            chunk(0, &long),
            chunk(1, &long),
            chunk(2, &long),
            chunk(3, "short"),
        ];
        let v = preview_quality("a-book", &text("clean"), &chunks, &q());
        assert!((v.fragment_rate - 0.25).abs() < 1e-9);
    }
}
