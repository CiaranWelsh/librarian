//! Pipeline runner — orchestrates extract → chunk → embed → index.
//! Slice 002: serial, no fault catching, no cache lookup.

use librarian_domain::{
    cache_key, AdapterIdentity, Chunker, Document, Embedder, Extractor, Indexer,
    ProvenanceLink, Vector,
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

impl<E, Ch, Em, Ix> Pipeline<E, Ch, Em, Ix>
where
    E: Extractor,
    Ch: Chunker,
    Em: Embedder,
    Ix: Indexer,
{
    pub fn run(&self, doc: &Document) -> Result<RunSummary, RunError<E::Error, Ch::Error, Ix::Error>> {
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

#[derive(Debug, Clone, Copy)]
pub struct RunSummary {
    pub chunks_indexed: usize,
}

fn link<A: AdapterIdentity>(
    adapter: &A,
    source_hash: &librarian_domain::SourceHash,
) -> ProvenanceLink {
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
