# 009 — PDF text extractor

**Phase:** D · **Status:** Open · **Deps:** 008

## Goal

First real-world `Extractor` adapter: text PDFs (book / paper). Replaces the trivial UTF-8 extractor for `ContentType::Book` and `ContentType::Paper`.

## Acceptance criteria

- `adapter-extractor-pdf` crate; uses `pdfium-render` as the v1 backend (subprocess fallback deferred until a corpus actually demands it).
- Produces `ExtractedText` with usable `TextSpan`s — at minimum, headings and paragraphs distinguishable for the chunker.
- Page numbers preserved on each span.
- Populates `BookMeta` (title, author, chapter, section, page) for `ContentType::Book` and `PaperMeta` (title, authors, page_start, page_end, section) for `ContentType::Paper` per F-M.3.
- Two reference inputs: one book chapter PDF, one paper PDF, both in `tests/fixtures/`.

## Test plan

- Unit: extract both fixtures; assert non-empty text, span count > 5, every span has page number.
- Integration: end-to-end ingest of one paper through Phase B/C adapters; verify chunk count and `PaperMeta.page_start` populated.
