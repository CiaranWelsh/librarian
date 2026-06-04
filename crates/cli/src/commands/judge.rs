//! `librarian judge` (issue 028, Tier 1): the accurate, on-demand RAG quality read — an LLM
//! context-relevance judge over a query's top-k retrieved chunks. This is the RAG Triad's
//! retrieval edge (librarian is retrieval-only, so groundedness/answer-relevance don't apply).
//! Per-chunk pointwise 0-2 rubric (G-Eval / TruLens style; docs/research/rag-quality/FINDINGS.md).
//! Prompt/parse/aggregate are pure + unit-tested; the chat call is the I/O.
//!
//! Live-traffic sampling is intentionally NOT in the (stateless, ADR-0005) query daemon — the
//! monitoring home is `librarian health`; this command is the on-demand read.

use serde_json::{json, Value};

use crate::commands::query::fetch_search;

const DEFAULT_JUDGE_MODEL: &str = "gpt-4o-mini";

/// Per-chunk context-relevance rubric (pointwise 0-2). Mirrors `experiments/chunking/judge_eval.py`.
pub fn judge_prompt(query: &str, chunk: &str) -> String {
    format!(
        "You are a CONTEXT RELEVANCE grader. Given a QUESTION and a single retrieved CONTEXT \
chunk, judge ONLY whether this chunk contains information that helps answer the QUESTION. Do \
not reward length, fluency, or style, and use only what is in the chunk.\n\n\
Score:\n\
0 = irrelevant, or a heading/fragment with no real content\n\
1 = related/partial - touches the topic but does not actually answer it\n\
2 = directly answers the question with substantive content\n\n\
Reply with ONLY the single digit 0, 1, or 2.\n\n\
QUESTION:\n{query}\n\nCONTEXT CHUNK:\n{chunk}"
    )
}

/// Extract the 0/1/2 score from a judge reply (first such digit; defaults to 0).
pub fn parse_score(reply: &str) -> u8 {
    reply
        .chars()
        .find(|c| matches!(c, '0' | '1' | '2'))
        .and_then(|c| c.to_digit(10))
        .unwrap_or(0) as u8
}

#[derive(Debug, PartialEq)]
pub struct JudgeReport {
    pub n: usize,
    pub mean: f32,
    pub relevant_count: usize, // chunks scoring 2
}

pub fn aggregate(scores: &[u8]) -> JudgeReport {
    let n = scores.len();
    if n == 0 {
        return JudgeReport {
            n: 0,
            mean: 0.0,
            relevant_count: 0,
        };
    }
    let sum: u32 = scores.iter().map(|&s| s as u32).sum();
    JudgeReport {
        n,
        mean: sum as f32 / n as f32,
        relevant_count: scores.iter().filter(|&&s| s == 2).count(),
    }
}

fn call_judge(base_url: &str, key: &str, model: &str, prompt: &str) -> Result<u8, String> {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    let body = json!({
        "model": model,
        "temperature": 0,
        "max_tokens": 1,
        "messages": [{"role": "user", "content": prompt}],
    });
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(&url)
        .bearer_auth(key)
        .json(&body)
        .send()
        .map_err(|e| format!("judge request failed: {e}"))?;
    let status = resp.status();
    let v: Value = resp
        .json()
        .map_err(|e| format!("bad judge response: {e}"))?;
    if !status.is_success() {
        let msg = v["error"]["message"].as_str().unwrap_or("");
        return Err(format!("judge api {status}: {msg}"));
    }
    let content = v["choices"][0]["message"]["content"].as_str().unwrap_or("");
    Ok(parse_score(content))
}

pub fn cmd_judge(
    daemon: &str,
    collection: &str,
    query: &str,
    k: u64,
    model: Option<&str>,
) -> Result<(), String> {
    let key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| "OPENAI_API_KEY not set (the judge needs an LLM)".to_string())?;
    let base = std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com".into());
    let model = model.unwrap_or(DEFAULT_JUDGE_MODEL);

    let value = fetch_search(daemon, collection, query, k)?;
    let hits = value["hits"].as_array().cloned().unwrap_or_default();
    if hits.is_empty() {
        println!("no hits to judge for {query:?}");
        return Ok(());
    }

    let mut scores = Vec::new();
    for hit in &hits {
        let text = hit["text"].as_str().unwrap_or("");
        let sid = hit["source_id"].as_str().unwrap_or("");
        let idx = hit["chunk_index"].as_u64().unwrap_or(0);
        let score = call_judge(&base, &key, model, &judge_prompt(query, text))?;
        scores.push(score);
        println!("[{score}] {sid}#{idx}");
    }
    let report = aggregate(&scores);
    println!(
        "judge: {} chunks  mean={:.2}/2  directly-relevant={}/{}",
        report.n, report.mean, report.relevant_count, report.n
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_contains_query_chunk_and_scale() {
        let p = judge_prompt(
            "what is a saga?",
            "A saga is a sequence of local transactions.",
        );
        assert!(p.contains("what is a saga?"));
        assert!(p.contains("A saga is a sequence"));
        assert!(p.contains("0, 1, or 2"));
    }

    #[test]
    fn parse_score_extracts_first_relevant_digit() {
        assert_eq!(parse_score("2"), 2);
        assert_eq!(parse_score("Score: 1"), 1);
        assert_eq!(parse_score("0\n"), 0);
        assert_eq!(parse_score("no digit here"), 0); // default
        assert_eq!(parse_score("the answer is 2"), 2);
    }

    #[test]
    fn aggregate_mean_and_relevant_count() {
        let r = aggregate(&[2, 1, 0, 2]);
        assert_eq!(r.n, 4);
        assert!((r.mean - 1.25).abs() < 1e-6); // (2+1+0+2)/4
        assert_eq!(r.relevant_count, 2);
        assert_eq!(
            aggregate(&[]),
            JudgeReport {
                n: 0,
                mean: 0.0,
                relevant_count: 0
            }
        );
    }
}
