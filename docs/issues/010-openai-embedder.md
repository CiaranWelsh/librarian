# 010 — OpenAI embedder

**Phase:** D · **Status:** Open · **Deps:** 009

## Goal

`adapter-embedder-openai` implementing `Embedder` against OpenAI's `text-embedding-3-large`. First real network adapter.

## Acceptance criteria

- Crate uses `reqwest` (blocking) — sync trait per F-1.5.
- Batching: configurable batch size; respects OpenAI's per-request limits.
- Errors classified: HTTP 5xx / timeouts → `Embedder::Error::Recoverable` (so the fallback chain in 011 can act); 4xx auth/quota → `Terminal`.
- API key from env (`OPENAI_API_KEY`); never logged.
- `AdapterIdentity` reports model name + version + a config hash including model + dimensions.

## Test plan

- Unit with mocked HTTP (`mockito` or similar): batching boundary, error classification, retry-disabled (retries are the runner's job).
- Manual smoke: small ingest against real API to confirm the wire format.
- Cost gate: skip the live test by default; gated behind `OPENAI_LIVE=1`.
