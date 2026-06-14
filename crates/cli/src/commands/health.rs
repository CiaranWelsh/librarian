//! `librarian health` (issue 028, Tier 2): run the golden probe set against a collection and
//! report retrieval health (hit-rate@k, MRR, fragment-rate@5, mean top-1 score), appending each
//! run to a JSONL history so a regression — e.g. after an ingest degrades retrieval — is visible
//! over time. Thin HTTP client over the daemon; the metric math is pure and unit-tested. Mirrors
//! `eval/run_eval.py`.
//!
//! `mean_top_score` is the cheap drift signal: a sustained drop across runs flags embedding /
//! index drift (e.g. a partial re-embed mixing embedding generations). A fuller static-probe-doc
//! re-embedding check would need embedder access via a daemon endpoint (future work).

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::json;

use crate::commands::http::Daemon;
use crate::commands::output::Render;
use crate::commands::query::search;

/// Fragment heuristic, mirroring `query_core`'s and `eval/run_eval.py`: under 80 characters or
/// a bare markdown heading. Inlined to keep the CLI a thin client (no `query-core` dependency);
/// the definition is intentionally identical so online and offline scores agree.
pub(crate) fn is_fragment(text: &str) -> bool {
    text.chars().count() < 80 || text.trim_start().starts_with('#')
}

#[derive(Debug, Deserialize)]
pub struct GoldenItem {
    pub q: String,
    pub relevant: Vec<String>,
}

/// Each JSONL history line is a superset of this struct (extra `ts`/`collection` fields);
/// serde ignores unknown fields by default, so history lines deserialize cleanly.
#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HealthReport {
    pub n: usize,
    pub k: u64,
    pub hit_rate: f32,
    pub mrr: f32,
    pub fragment_rate: f32,
    /// Mean top-1 cosine over the probe set — the drift signal (track over runs).
    pub mean_top_score: f32,
}

/// 1-based rank of the first hit whose `source_id` contains any of the `relevant` substrings
/// (case-insensitive). `None` if no relevant source appears in the list.
pub fn first_relevant_rank(source_ids: &[String], relevant: &[String]) -> Option<usize> {
    source_ids
        .iter()
        .position(|s| {
            let sl = s.to_lowercase();
            relevant.iter().any(|r| sl.contains(&r.to_lowercase()))
        })
        .map(|i| i + 1)
}

/// Fraction of the first five hit texts that are fragments (shared definition with Tier 0).
pub fn fragment_rate_at5(texts: &[String]) -> f32 {
    let top5 = &texts[..texts.len().min(5)];
    if top5.is_empty() {
        return 0.0;
    }
    top5.iter().filter(|t| is_fragment(t)).count() as f32 / top5.len() as f32
}

/// Aggregate per-question `(first-relevant-rank, fragment-rate@5, top-1 score)` into a report.
pub fn aggregate(per_question: &[(Option<usize>, f32, f32)], k: u64) -> HealthReport {
    let n = per_question.len();
    if n == 0 {
        return HealthReport {
            n: 0,
            k,
            hit_rate: 0.0,
            mrr: 0.0,
            fragment_rate: 0.0,
            mean_top_score: 0.0,
        };
    }
    let nf = n as f32;
    let hits = per_question.iter().filter(|(r, _, _)| r.is_some()).count();
    let mrr = per_question
        .iter()
        .map(|(r, _, _)| r.map_or(0.0, |x| 1.0 / x as f32))
        .sum::<f32>()
        / nf;
    let frag = per_question.iter().map(|(_, f, _)| f).sum::<f32>() / nf;
    let mean_top_score = per_question.iter().map(|(_, _, s)| s).sum::<f32>() / nf;
    HealthReport {
        n,
        k,
        hit_rate: hits as f32 / nf,
        mrr,
        fragment_rate: frag,
        mean_top_score,
    }
}

/// Read and parse the golden probe set, rejecting an empty set. Shared by
/// `cmd_health` and `add`'s Gate 2.
pub(crate) fn load_golden(path: &Path) -> Result<Vec<GoldenItem>, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("read golden {}: {e}", path.display()))?;
    let golden: Vec<GoldenItem> =
        serde_json::from_str(&raw).map_err(|e| format!("parse golden: {e}"))?;
    if golden.is_empty() {
        return Err("golden probe set is empty".into());
    }
    Ok(golden)
}

/// Run every golden question against the collection, returning the per-question
/// `(first-relevant-rank, fragment-rate@5, top-1 score)` outcomes in golden order.
/// Owns the search loop and hit-list parsing; the metric math stays in `aggregate`.
/// Shared by `cmd_health` (which also derives a rank row per question) and `add`'s
/// Gate 2.
pub(crate) fn measure_per_question(
    d: &Daemon,
    collection: &str,
    golden: &[GoldenItem],
    k: u64,
) -> Result<Vec<(Option<usize>, f32, f32)>, String> {
    let mut per_question: Vec<(Option<usize>, f32, f32)> = Vec::new();
    for item in golden {
        let value = search(d, collection, &item.q, k)?;
        let hits = value["hits"].as_array().cloned().unwrap_or_default();
        let sources: Vec<String> = hits
            .iter()
            .map(|h| h["source_id"].as_str().unwrap_or("").to_string())
            .collect();
        let texts: Vec<String> = hits
            .iter()
            .map(|h| h["text"].as_str().unwrap_or("").to_string())
            .collect();
        let top_score = hits
            .first()
            .and_then(|h| h["score"].as_f64())
            .unwrap_or(0.0) as f32;
        let rank = first_relevant_rank(&sources, &item.relevant);
        per_question.push((rank, fragment_rate_at5(&texts), top_score));
    }
    Ok(per_question)
}

/// Measure retrieval health into a `HealthReport`. Shared by `add`'s Gate 2;
/// `cmd_health` aggregates its own loop output because it also builds the rows.
pub(crate) fn measure(
    d: &Daemon,
    collection: &str,
    golden: &[GoldenItem],
    k: u64,
) -> Result<HealthReport, String> {
    Ok(aggregate(
        &measure_per_question(d, collection, golden, k)?,
        k,
    ))
}

/// Append one report to the JSONL history, tagging it with the wall-clock second
/// and collection. The on-disk shape is a superset of `HealthReport`, so
/// `read_history` deserializes it back (serde ignores the extra fields). Shared by
/// `cmd_health` and `add`'s Gate 2; prints the "appended to" line itself so both
/// callers report consistently.
pub(crate) fn append_history(
    path: &Path,
    collection: &str,
    report: &HealthReport,
) -> Result<(), String> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let record = json!({
        "ts": ts,
        "collection": collection,
        "k": report.k,
        "n": report.n,
        "hit_rate": report.hit_rate,
        "mrr": report.mrr,
        "fragment_rate": report.fragment_rate,
        "mean_top_score": report.mean_top_score,
    });
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("open history {}: {e}", path.display()))?;
    f.write_all(format!("{record}\n").as_bytes())
        .map_err(|e| format!("write history: {e}"))?;
    println!("appended to {}", path.display());
    Ok(())
}

/// Read the JSONL history into `HealthReport`s, newest entries last. Robust over
/// strict: a missing file is an empty history, blank lines are skipped, and a
/// malformed line is skipped rather than aborting the read (a single bad append
/// must not blind the regression gate to the rest of the baseline).
pub(crate) fn read_history(path: &Path) -> Result<Vec<HealthReport>, String> {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("read history {}: {e}", path.display())),
    };
    let mut reports = Vec::new();
    let mut skipped = 0usize;
    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        match serde_json::from_str::<HealthReport>(line) {
            Ok(r) => reports.push(r),
            Err(_) => skipped += 1,
        }
    }
    if skipped > 0 {
        eprintln!(
            "WARNING: skipped {skipped} malformed line(s) in history {}",
            path.display()
        );
    }
    Ok(reports)
}

pub fn cmd_health(
    d: &Daemon,
    r: Render,
    collection: &str,
    golden_path: &Path,
    k: u64,
    history_path: Option<&Path>,
) -> Result<(), String> {
    let golden = load_golden(golden_path)?;

    // One shared search loop (`measure_per_question`); the human table additionally
    // wants a per-question rank row, derived by zipping the outcomes with the golden
    // questions in order.
    let per_question = measure_per_question(d, collection, &golden, k)?;
    let rows: Vec<(Option<usize>, String)> = golden
        .iter()
        .zip(&per_question)
        .map(|(item, (rank, _, _))| (*rank, item.q.clone()))
        .collect();

    let report = aggregate(&per_question, k);
    if r.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
        );
    } else {
        println!(
            "health[{collection}]  n={}  hit-rate@{}={:.0}%  MRR={:.3}  fragment-rate@5={:.0}%  mean-top={:.3}",
            report.n,
            report.k,
            report.hit_rate * 100.0,
            report.mrr,
            report.fragment_rate * 100.0,
            report.mean_top_score,
        );
        println!("rank | question");
        for (rank, q) in &rows {
            let rk = rank.map_or("-".to_string(), |x| x.to_string());
            let preview: String = q.chars().take(60).collect();
            println!("  {rk:>3} | {preview}");
        }
    }

    if let Some(hp) = history_path {
        append_history(hp, collection, &report)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_finds_first_relevant_source_case_insensitive() {
        let sources = vec![
            "zebra#0".to_string(),
            "testing_Effective-Software-Testing#3".to_string(),
        ];
        assert_eq!(
            first_relevant_rank(&sources, &["effective-software-testing".into()]),
            Some(2)
        );
        assert_eq!(first_relevant_rank(&sources, &["nonexistent".into()]), None);
    }

    #[test]
    fn fragment_rate_over_top5() {
        let body = "x".repeat(200);
        let texts = vec!["# Heading".to_string(), body.clone(), body];
        assert!((fragment_rate_at5(&texts) - 1.0 / 3.0).abs() < 1e-6);
        assert_eq!(fragment_rate_at5(&[]), 0.0);
    }

    #[test]
    fn aggregate_computes_metrics_and_mean_top() {
        // q1 rank1 frag0 top0.6, q2 rank2 frag0.5 top0.5, q3 miss frag1 top0.1
        let per_q = vec![
            (Some(1), 0.0_f32, 0.6_f32),
            (Some(2), 0.5, 0.5),
            (None, 1.0, 0.1),
        ];
        let r = aggregate(&per_q, 10);
        assert_eq!(r.n, 3);
        assert!((r.hit_rate - 2.0 / 3.0).abs() < 1e-6);
        assert!((r.mrr - 0.5).abs() < 1e-6); // (1 + 0.5 + 0)/3
        assert!((r.fragment_rate - 0.5).abs() < 1e-6); // (0 + 0.5 + 1)/3
        assert!((r.mean_top_score - 0.4).abs() < 1e-6); // (0.6 + 0.5 + 0.1)/3
    }

    #[test]
    fn read_history_parses_jsonl_skipping_blanks() {
        // Two real history lines (one carrying the extra ts/collection tags serde
        // ignores) plus a blank line; expect exactly two reports back.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("health_software.jsonl");
        let body = "{\"ts\":1700000000,\"collection\":\"software\",\"k\":10,\"n\":50,\"hit_rate\":0.98,\"mrr\":0.77,\"fragment_rate\":0.02,\"mean_top_score\":0.61}\n\
            \n\
            {\"k\":10,\"n\":50,\"hit_rate\":1.0,\"mrr\":0.80,\"fragment_rate\":0.0,\"mean_top_score\":0.65}\n";
        std::fs::write(&path, body).unwrap();

        let reports = read_history(&path).unwrap();
        assert_eq!(reports.len(), 2);
        assert!((reports[0].hit_rate - 0.98).abs() < 1e-6);
        assert_eq!(reports[0].n, 50);
        assert!((reports[1].mrr - 0.80).abs() < 1e-6);
    }

    #[test]
    fn read_history_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.jsonl");
        assert!(read_history(&missing).unwrap().is_empty());
    }

    #[test]
    fn empty_set_is_zeroed() {
        let r = aggregate(&[], 5);
        assert_eq!(
            r,
            HealthReport {
                n: 0,
                k: 5,
                hit_rate: 0.0,
                mrr: 0.0,
                fragment_rate: 0.0,
                mean_top_score: 0.0,
            }
        );
    }
}
