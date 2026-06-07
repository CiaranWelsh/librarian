//! `BatchRunner` — wraps a `Pipeline` with `ManifestStore` + `Cache`, catches
//! errors at the per-document fault boundary (slice 006), and consults the
//! cache before each stage to skip work on idempotent re-ingest (slice 007).

use librarian_domain::{
    cache_key, classify_section, garble_signal, AdapterIdentity, Cache, CacheKey, Chunk, Chunker,
    ConfigHash, Document, Embedder, ExtractedText, Extractor, Indexer, ManifestStatus,
    ManifestStore, QualityConfig, SectionDecision, SourceHash, SourceId, Vector,
};

use crate::pipeline::{link, Pipeline};

/// Outcome of one Document inside a batch.
#[derive(Debug, Clone)]
pub enum Outcome {
    Success {
        source_id: SourceId,
        chunks_indexed: usize,
    },
    /// Skipped before extraction as a low-value section (F-EQ.1).
    Skipped { source_id: SourceId, reason: String },
    Failed {
        source_id: SourceId,
        stage: &'static str,
        error: String,
    },
}

impl Outcome {
    pub fn source_id(&self) -> &SourceId {
        match self {
            Outcome::Success { source_id, .. }
            | Outcome::Skipped { source_id, .. }
            | Outcome::Failed { source_id, .. } => source_id,
        }
    }
    pub fn is_success(&self) -> bool {
        matches!(self, Outcome::Success { .. })
    }
    pub fn is_failed(&self) -> bool {
        matches!(self, Outcome::Failed { .. })
    }
}

pub struct BatchRunner<E, Ch, Em, Ix, M, C> {
    pub pipeline: Pipeline<E, Ch, Em, Ix>,
    pub manifest: M,
    pub cache: C,
    /// Per-collection ingest-quality policy (ADR-0006). The `Default` is a
    /// no-op section filter; the garble signal is always recorded (QA-EQ2).
    pub quality: QualityConfig,
}

impl<E, Ch, Em, Ix, M, C> BatchRunner<E, Ch, Em, Ix, M, C>
where
    E: Extractor,
    Ch: Chunker,
    Em: Embedder,
    Ix: Indexer,
    M: ManifestStore,
    C: Cache,
{
    /// Ingest a batch of documents, applying the runner's ingest-quality policy
    /// (ADR-0006): low-value sections are skipped (F-EQ.1) and a per-document
    /// garble signal is recorded (F-EQ.2).
    pub fn ingest_batch(&self, docs: &[Document]) -> Vec<Outcome> {
        docs.iter().map(|d| self.ingest_one(d)).collect()
    }

    fn ingest_one(&self, doc: &Document) -> Outcome {
        let sid = &doc.source_id;
        let sh = &doc.source_hash;

        // ── section filter (F-EQ.1) ── classify by section identity before any
        // work; low-value boilerplate is skipped end-to-end and recorded.
        if let SectionDecision::Skip { reason } = classify_section(doc, &self.quality.sections) {
            self.record(sid, "section", ManifestStatus::Skipped, Some(&reason), None);
            return Outcome::Skipped {
                source_id: sid.clone(),
                reason,
            };
        }

        // ── extract ──
        let ext_key = key_for(sh, &self.pipeline.extractor);
        let extracted: ExtractedText = match self.lookup(&ext_key) {
            Some(t) => {
                self.record(sid, "extract", ManifestStatus::Cached, None, Some(&ext_key));
                t
            }
            None => match self.pipeline.extractor.extract(doc) {
                Ok(t) => {
                    self.store(&ext_key, &t);
                    self.record(
                        sid,
                        "extract",
                        ManifestStatus::Success,
                        None,
                        Some(&ext_key),
                    );
                    t
                }
                Err(e) => return self.fail(sid, "extract", e.to_string()),
            },
        };

        // ── garble signal (F-EQ.2) ── advisory; always recorded (QA-EQ2),
        // never blocks ingest.
        let g = garble_signal(&extracted, &self.quality.garble);
        let status = if g.flagged {
            ManifestStatus::Flagged
        } else {
            ManifestStatus::Success
        };
        let msg = format!(
            "ufffd/kc={:.3} lspace/kc={:.3} value={:.3}",
            g.ufffd_per_kc, g.letterspace_per_kc, g.value
        );
        self.record(sid, "quality", status, Some(&msg), None);

        // ── chunk ──
        let chunk_key = key_for(sh, &self.pipeline.chunker);
        let mut chunks: Vec<Chunk> = match self.lookup(&chunk_key) {
            Some(c) => {
                self.record(sid, "chunk", ManifestStatus::Cached, None, Some(&chunk_key));
                c
            }
            None => match self.pipeline.chunker.chunk(doc, extracted) {
                Ok(c) => {
                    self.store(&chunk_key, &c);
                    self.record(
                        sid,
                        "chunk",
                        ManifestStatus::Success,
                        None,
                        Some(&chunk_key),
                    );
                    c
                }
                Err(e) => return self.fail(sid, "chunk", e.to_string()),
            },
        };

        // Provenance — appended in fixed order regardless of cache hits, since
        // the *content* of a chunk doesn't change when its inputs are cached.
        let p_ext = link(&self.pipeline.extractor, sh);
        let p_chunk = link(&self.pipeline.chunker, sh);
        let p_embed = link(&self.pipeline.embedder, sh);
        for c in &mut chunks {
            c.provenance.0.push(p_ext.clone());
            c.provenance.0.push(p_chunk.clone());
            c.provenance.0.push(p_embed.clone());
        }

        // ── embed ──
        // The embed cache must reflect the chunk-set it embeds, not just the source +
        // embedder. `chunk_key` identifies (source, chunker), so folding it into the embed
        // key means a chunker change invalidates embed too — otherwise re-chunking serves
        // stale vectors and the index stage rejects the count mismatch (issue 028 cache bug).
        let embed_key = cache_key::derive(
            sh,
            self.pipeline.embedder.name(),
            &self.pipeline.embedder.version(),
            &ConfigHash(format!(
                "{};chunks={}",
                self.pipeline.embedder.config_hash().0,
                chunk_key.0
            )),
        );
        let vectors: Vec<Vector> = match self.lookup(&embed_key) {
            Some(v) => {
                self.record(sid, "embed", ManifestStatus::Cached, None, Some(&embed_key));
                v
            }
            None => {
                let texts: Vec<&str> = chunks.iter().map(|c| c.text.as_str()).collect();
                match self.pipeline.embedder.embed(&texts) {
                    Ok(v) => {
                        self.store(&embed_key, &v);
                        // Slice 011: if the embedder is a fallback combinator and
                        // recovered, record RecoveredViaFallback with the primary error.
                        match self.pipeline.embedder.last_event() {
                            Some(ev) if ev.recovered => {
                                let msg = format!("primary recoverable: {}", ev.primary_error);
                                self.record(
                                    sid,
                                    "embed",
                                    ManifestStatus::RecoveredViaFallback,
                                    Some(&msg),
                                    Some(&embed_key),
                                );
                            }
                            _ => self.record(
                                sid,
                                "embed",
                                ManifestStatus::Success,
                                None,
                                Some(&embed_key),
                            ),
                        }
                        v
                    }
                    Err(e) => {
                        // Slice 011: if a fallback combinator left an event with both
                        // errors, record the combined Failed message.
                        let combined = match self.pipeline.embedder.last_event() {
                            Some(ev) => format!(
                                "primary: {}; fallback: {}",
                                ev.primary_error,
                                ev.fallback_error.unwrap_or_else(|| "n/a".into()),
                            ),
                            None => e.to_string(),
                        };
                        return self.fail(sid, "embed", combined);
                    }
                }
            }
        };

        // ── index ── always `replace`, not `upsert`. This makes the runner
        // naturally handle the F-1.8 update-with-fewer-chunks case: any
        // chunk_index that no longer exists for `source_id` is dropped.
        // Deterministic point IDs keep this idempotent on unchanged input.
        match self.pipeline.indexer.replace(sid, &chunks, &vectors) {
            Ok(()) => self.record(sid, "index", ManifestStatus::Success, None, None),
            Err(e) => return self.fail(sid, "index", e.to_string()),
        }

        Outcome::Success {
            source_id: sid.clone(),
            chunks_indexed: chunks.len(),
        }
    }

    /// Explicit removal (F-1.9). Drops every chunk for `source_id` from the
    /// indexer and records `ManifestStatus::Removed` for each pipeline stage.
    /// Removing a missing source is a no-op.
    pub fn remove(&self, source_id: &SourceId) -> Result<(), String> {
        if let Err(e) = self.pipeline.indexer.delete_by_source_id(source_id) {
            self.record(
                source_id,
                "index",
                ManifestStatus::Failed,
                Some(&e.to_string()),
                None,
            );
            return Err(e.to_string());
        }
        for stage in ["extract", "chunk", "embed", "index"] {
            self.record(source_id, stage, ManifestStatus::Removed, None, None);
        }
        Ok(())
    }

    fn lookup<T: serde::de::DeserializeOwned>(&self, key: &CacheKey) -> Option<T> {
        match self.cache.get(key) {
            Ok(Some(bytes)) => serde_json::from_slice(&bytes).ok(),
            _ => None,
        }
    }

    fn store<T: serde::Serialize>(&self, key: &CacheKey, value: &T) {
        if let Ok(bytes) = serde_json::to_vec(value) {
            let _ = self.cache.put(key, &bytes);
        }
    }

    fn record(
        &self,
        sid: &SourceId,
        stage: &'static str,
        status: ManifestStatus,
        error: Option<&str>,
        output_ref: Option<&CacheKey>,
    ) {
        let _ = self
            .manifest
            .record(sid, stage, status, 1, error, output_ref);
    }

    fn fail(&self, sid: &SourceId, stage: &'static str, msg: String) -> Outcome {
        self.record(sid, stage, ManifestStatus::Failed, Some(&msg), None);
        Outcome::Failed {
            source_id: sid.clone(),
            stage,
            error: msg,
        }
    }
}

fn key_for<A: AdapterIdentity>(source_hash: &SourceHash, adapter: &A) -> CacheKey {
    cache_key::derive(
        source_hash,
        adapter.name(),
        &adapter.version(),
        &adapter.config_hash(),
    )
}
