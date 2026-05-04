# 011 — Fallback adapter chain

**Phase:** D · **Status:** Open · **Deps:** 010

## Goal

Implements **F-1.6, QA-F2**: a recoverable failure on the primary embedder retries on a configured fallback; manifest records both attempts.

## Acceptance criteria

- Combinator type `FallbackEmbedder<P, F>` where `P: Embedder` and `F: Embedder`. Itself implements `Embedder`. Generic, not `dyn`.
- On `Embedder::Error::Recoverable` from primary → calls fallback. On `Terminal` → propagates without retry.
- Both attempts surface in the manifest row: status `RecoveredViaFallback` on success, `Failed` with both error messages on terminal failure of the fallback.
- v1 chain length: primary + single fallback. Deeper chains and other-stage fallbacks are deferred until a corpus needs them.

## Test plan

- Stub primary that returns `Recoverable` once + stub fallback that succeeds → status `RecoveredViaFallback`, both error messages preserved.
- Stub primary `Recoverable` + stub fallback `Terminal` → status `Failed`, both messages preserved.
- Stub primary `Terminal` → fallback never called.
