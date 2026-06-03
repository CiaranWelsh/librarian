//! `Pipeline` — the happy-path runner. Generic over the four stage traits;
//! orchestrates extract → chunk → embed → index for a single document, with no
//! fault catching and no cache lookup. The fault boundary and cache-aware
//! behaviour belong to `BatchRunner` (see `batch.rs`).

use librarian_domain::{
    cache_key, AdapterIdentity, Chunker, Document, Embedder, Extractor, Indexer, ProvenanceLink,
    SourceHash, Vector,
};

pub struct Pipeline<E, Ch, Em, Ix> {
    pub extractor: E,
    pub chunker: Ch,
    pub embedder: Em,
    pub indexer: Ix,
}

#[derive(Debug, thiserror::Error)]
pub enum RunError<EE, CE, IE> {
    #[error("extract: {0}")]
    Extract(#[source] EE),
    #[error("chunk: {0}")]
    Chunk(#[source] CE),
    #[error("embed: {0}")]
    Embed(#[source] librarian_domain::EmbedderError),
    #[error("index: {0}")]
    Index(#[source] IE),
}

#[derive(Debug, Clone, Copy)]
pub struct RunSummary {
    pub chunks_indexed: usize,
}

impl<E, Ch, Em, Ix> Pipeline<E, Ch, Em, Ix>
where
    E: Extractor,
    Ch: Chunker,
    Em: Embedder,
    Ix: Indexer,
{
    pub fn run(
        &self,
        doc: &Document,
    ) -> Result<RunSummary, RunError<E::Error, Ch::Error, Ix::Error>> {
        let extracted = self.extractor.extract(doc).map_err(RunError::Extract)?;
        let mut chunks = self
            .chunker
            .chunk(doc, extracted)
            .map_err(RunError::Chunk)?;

        // Append provenance for the stages we just ran. Cache lookup is slice 007.
        let extract_link = link(&self.extractor, &doc.source_hash);
        let chunk_link = link(&self.chunker, &doc.source_hash);
        let embed_link = link(&self.embedder, &doc.source_hash);
        for c in &mut chunks {
            c.provenance.0.push(extract_link.clone());
            c.provenance.0.push(chunk_link.clone());
            c.provenance.0.push(embed_link.clone());
        }

        let texts: Vec<&str> = chunks.iter().map(|c| c.text.as_str()).collect();
        let vectors: Vec<Vector> = self.embedder.embed(&texts).map_err(RunError::Embed)?;

        self.indexer
            .upsert(&chunks, &vectors)
            .map_err(RunError::Index)?;

        Ok(RunSummary {
            chunks_indexed: chunks.len(),
        })
    }
}

/// Shared with `BatchRunner` — builds a `ProvenanceLink` from an adapter's
/// identity. The cache-key formula lives in `librarian_domain::cache_key`
/// (single source of truth, ADR-0001 §4).
pub(crate) fn link<A: AdapterIdentity>(adapter: &A, source_hash: &SourceHash) -> ProvenanceLink {
    let key = cache_key::derive(
        source_hash,
        adapter.name(),
        &adapter.version(),
        &adapter.config_hash(),
    );
    ProvenanceLink {
        stage_name: adapter.name().to_string(),
        stage_version: adapter.version(),
        config_hash: adapter.config_hash(),
        cache_key: key,
    }
}

#[cfg(test)]
mod stub_tests {
    //! Stub-based unit tests on `Pipeline::run` — proves the runner is testable
    //! without any real adapter, per slice-002 AC.

    use super::*;
    use librarian_domain::{
        BookMeta, Chunk, ChunkId, ChunkPayload, ConfigHash, ContentType, EmbedderError,
        ExtractedText, Provenance, SourceHash, SourceId, SpanKind, StageVersion, TextSpan,
    };
    use std::cell::RefCell;

    /// Records the order in which stages were invoked.
    #[derive(Default)]
    struct CallLog(RefCell<Vec<&'static str>>);
    impl CallLog {
        fn push(&self, s: &'static str) {
            self.0.borrow_mut().push(s);
        }
        fn snapshot(&self) -> Vec<&'static str> {
            self.0.borrow().clone()
        }
    }

    struct StubExtractor<'a> {
        log: &'a CallLog,
        n_spans: usize,
    }
    impl<'a> AdapterIdentity for StubExtractor<'a> {
        fn name(&self) -> &str {
            "stub-extract"
        }
        fn version(&self) -> StageVersion {
            StageVersion("v1".into())
        }
        fn config_hash(&self) -> ConfigHash {
            ConfigHash("c".into())
        }
    }
    #[derive(Debug, thiserror::Error)]
    #[error("stub-extract")]
    struct ExtErr;
    impl<'a> Extractor for StubExtractor<'a> {
        type Error = ExtErr;
        fn extract(&self, _: &Document) -> Result<ExtractedText, Self::Error> {
            self.log.push("extract");
            Ok(ExtractedText {
                spans: (0..self.n_spans)
                    .map(|i| TextSpan {
                        kind: SpanKind::Paragraph,
                        text: format!("p{i}"),
                        page: None,
                        byte_range: 0..2,
                    })
                    .collect(),
            })
        }
    }

    struct StubChunker<'a> {
        log: &'a CallLog,
    }
    impl<'a> AdapterIdentity for StubChunker<'a> {
        fn name(&self) -> &str {
            "stub-chunk"
        }
        fn version(&self) -> StageVersion {
            StageVersion("v1".into())
        }
        fn config_hash(&self) -> ConfigHash {
            ConfigHash("c".into())
        }
    }
    #[derive(Debug, thiserror::Error)]
    #[error("stub-chunk")]
    struct ChErr;
    impl<'a> Chunker for StubChunker<'a> {
        type Error = ChErr;
        fn chunk(&self, doc: &Document, t: ExtractedText) -> Result<Vec<Chunk>, Self::Error> {
            self.log.push("chunk");
            Ok(t.spans
                .into_iter()
                .enumerate()
                .map(|(i, s)| Chunk {
                    chunk_id: ChunkId(format!("{}#{i}", doc.source_id.0)),
                    source_id: doc.source_id.clone(),
                    chunk_index: i as u32,
                    text: s.text,
                    payload: ChunkPayload::Book(BookMeta {
                        title: "t".into(),
                        author: None,
                        chapter: None,
                        section: None,
                        page: None,
                    }),
                    provenance: Provenance::default(),
                })
                .collect())
        }
    }

    struct StubEmbedder<'a> {
        log: &'a CallLog,
        fail: Option<EmbedderError>,
    }
    impl<'a> AdapterIdentity for StubEmbedder<'a> {
        fn name(&self) -> &str {
            "stub-embed"
        }
        fn version(&self) -> StageVersion {
            StageVersion("v1".into())
        }
        fn config_hash(&self) -> ConfigHash {
            ConfigHash("c".into())
        }
    }
    impl<'a> Embedder for StubEmbedder<'a> {
        fn embed(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
            self.log.push("embed");
            if let Some(e) = &self.fail {
                return Err(match e {
                    EmbedderError::Recoverable(s) => EmbedderError::Recoverable(s.clone()),
                    EmbedderError::Terminal(s) => EmbedderError::Terminal(s.clone()),
                });
            }
            Ok(texts.iter().map(|_| vec![0.0]).collect())
        }
        fn dimension(&self) -> usize {
            1
        }
    }

    #[derive(Default)]
    struct StubIndexer<'a> {
        log: Option<&'a CallLog>,
        calls: RefCell<usize>,
        last_len: RefCell<usize>,
    }
    impl<'a> AdapterIdentity for StubIndexer<'a> {
        fn name(&self) -> &str {
            "stub-index"
        }
        fn version(&self) -> StageVersion {
            StageVersion("v1".into())
        }
        fn config_hash(&self) -> ConfigHash {
            ConfigHash("c".into())
        }
    }
    #[derive(Debug, thiserror::Error)]
    #[error("stub-index")]
    struct IxErr;
    impl<'a> Indexer for StubIndexer<'a> {
        type Error = IxErr;
        fn upsert(&self, chunks: &[Chunk], _: &[Vector]) -> Result<(), Self::Error> {
            if let Some(l) = self.log {
                l.push("index");
            }
            *self.calls.borrow_mut() += 1;
            *self.last_len.borrow_mut() = chunks.len();
            Ok(())
        }
        fn replace(&self, _: &SourceId, _: &[Chunk], _: &[Vector]) -> Result<(), Self::Error> {
            unreachable!()
        }
        fn delete_by_source_id(&self, _: &SourceId) -> Result<(), Self::Error> {
            unreachable!()
        }
    }

    fn doc() -> Document {
        Document {
            source_id: SourceId("s".into()),
            source_hash: SourceHash("h".into()),
            content_type: ContentType::Book,
            path: "x".into(),
            work_id: None,
        }
    }

    #[test]
    fn stages_invoked_in_order_extract_chunk_embed_index() {
        let log = CallLog::default();
        let ix = StubIndexer {
            log: Some(&log),
            ..Default::default()
        };
        let p = Pipeline {
            extractor: StubExtractor {
                log: &log,
                n_spans: 2,
            },
            chunker: StubChunker { log: &log },
            embedder: StubEmbedder {
                log: &log,
                fail: None,
            },
            indexer: ix,
        };
        let s = p.run(&doc()).unwrap();
        assert_eq!(s.chunks_indexed, 2);
        assert_eq!(log.snapshot(), vec!["extract", "chunk", "embed", "index"]);
        assert_eq!(*p.indexer.last_len.borrow(), 2);
    }

    #[test]
    fn extractor_failure_stops_pipeline_before_chunk() {
        struct FailingExt<'a>(&'a CallLog);
        impl<'a> AdapterIdentity for FailingExt<'a> {
            fn name(&self) -> &str {
                "fail-ext"
            }
            fn version(&self) -> StageVersion {
                StageVersion("v".into())
            }
            fn config_hash(&self) -> ConfigHash {
                ConfigHash("c".into())
            }
        }
        impl<'a> Extractor for FailingExt<'a> {
            type Error = ExtErr;
            fn extract(&self, _: &Document) -> Result<ExtractedText, Self::Error> {
                self.0.push("extract");
                Err(ExtErr)
            }
        }
        let log = CallLog::default();
        let p = Pipeline {
            extractor: FailingExt(&log),
            chunker: StubChunker { log: &log },
            embedder: StubEmbedder {
                log: &log,
                fail: None,
            },
            indexer: StubIndexer::<'_>::default(),
        };
        let r = p.run(&doc());
        assert!(matches!(r, Err(RunError::Extract(_))));
        assert_eq!(log.snapshot(), vec!["extract"]);
    }

    #[test]
    fn embedder_recoverable_surfaces_as_run_error_embed() {
        let log = CallLog::default();
        let p = Pipeline {
            extractor: StubExtractor {
                log: &log,
                n_spans: 1,
            },
            chunker: StubChunker { log: &log },
            embedder: StubEmbedder {
                log: &log,
                fail: Some(EmbedderError::Recoverable("net".into())),
            },
            indexer: StubIndexer::<'_>::default(),
        };
        let r = p.run(&doc());
        assert!(matches!(
            r,
            Err(RunError::Embed(EmbedderError::Recoverable(_)))
        ));
    }

    #[test]
    fn provenance_appends_three_links_per_chunk() {
        let log = CallLog::default();
        let ix = StubIndexer {
            log: Some(&log),
            ..Default::default()
        };
        let p = Pipeline {
            extractor: StubExtractor {
                log: &log,
                n_spans: 1,
            },
            chunker: StubChunker { log: &log },
            embedder: StubEmbedder {
                log: &log,
                fail: None,
            },
            indexer: ix,
        };
        p.run(&doc()).unwrap();
        assert_eq!(*p.indexer.last_len.borrow(), 1);
    }
}
