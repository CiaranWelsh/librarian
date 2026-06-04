//! Integration: QueryService over real stub embedder + real mem searcher.
use std::sync::Arc;

use adapter_embedder_stub::StubEmbedder;
use adapter_indexer_mem::MemSearcher;
use librarian_domain::{BookMeta, Chunk, ChunkId, ChunkPayload, Embedder, Provenance, SourceId};
use query_core::QueryService;

fn book_chunk(source: &str, idx: u32, text: &str) -> Chunk {
    Chunk {
        chunk_id: ChunkId(format!("{source}#{idx}")),
        source_id: SourceId(source.into()),
        chunk_index: idx,
        text: text.into(),
        payload: ChunkPayload::Book(BookMeta {
            title: source.into(),
            author: None,
            chapter: None,
            section: None,
            page: None,
        }),
        provenance: Provenance::default(),
    }
}

#[tokio::test]
async fn search_then_documents_then_extract() {
    let stub = StubEmbedder::new();
    let mem = MemSearcher::new();
    let v0 = stub.embed(&["intro"]).unwrap().remove(0);
    let v1 = stub.embed(&["method"]).unwrap().remove(0);
    mem.add("c", book_chunk("paper", 0, "intro section"), v0);
    mem.add("c", book_chunk("paper", 1, "method section"), v1);
    let svc = QueryService::new(Arc::new(stub), mem, 4);

    let hits = svc.search("c", "intro", 5, None).await.unwrap();
    assert_eq!(hits[0].source_id.0, "paper");

    let docs = svc.list_documents("c").await.unwrap();
    assert_eq!(docs, vec![SourceId("paper".into())]);

    let chunks = svc
        .get_extract("c", &SourceId("paper".into()), 0, 2)
        .await
        .unwrap();
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].chunk_index, 0);
}
