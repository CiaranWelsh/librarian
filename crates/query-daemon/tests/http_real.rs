#![cfg(feature = "test-support")]
//! Real-socket HTTP tests: drive the daemon over a real TCP connection with a
//! real reqwest client. Proves the full on-the-wire serialize/deserialize path
//! (B5 in the boundary map).

use query_daemon::test_support;

#[tokio::test(flavor = "multi_thread")]
async fn search_over_real_socket() {
    let (base, _h) = test_support::spawn().await;
    let url = format!("{base}/v1/search");
    let body = serde_json::json!({"collection":"demo","query":"apple","limit":5});
    let v: serde_json::Value = tokio::task::spawn_blocking(move || {
        reqwest::blocking::Client::new()
            .post(&url)
            .json(&body)
            .send()
            .unwrap()
            .json()
            .unwrap()
    })
    .await
    .unwrap();
    assert_eq!(v["hits"][0]["source_id"], "apple");
}

#[tokio::test(flavor = "multi_thread")]
async fn unknown_collection_is_404_on_the_wire() {
    let (base, _h) = test_support::spawn().await;
    let url = format!("{base}/v1/search");
    let body = serde_json::json!({"collection":"missing","query":"apple"});
    let status = tokio::task::spawn_blocking(move || {
        reqwest::blocking::Client::new()
            .post(&url)
            .json(&body)
            .send()
            .unwrap()
            .status()
            .as_u16()
    })
    .await
    .unwrap();
    assert_eq!(status, 404);
}
