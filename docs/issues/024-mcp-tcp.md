# 024 — MCP-over-TCP transport

**Phase:** J (post-v1) · **Status:** Open · **Deps:** 015

## Goal

Make the supervisor's port allocation actually do something. Today `librarian-collection` ignores the `--port` argument that slice 015's supervisor passes; it serves MCP over stdio only. Stdio works for daily use via SSH-piped Claude Code (`command = "ssh turbo librarian-collection --config …"`), so this isn't blocking — but the `--port` allocation is decorative until this lands.

## Acceptance criteria

- `librarian-collection` accepts `--port <u16>` and, when set, binds a TCP listener on that port instead of (or alongside) stdio.
- Transport: HTTP+SSE per the MCP spec, or raw newline-delimited JSON-RPC. Pick whichever the canonical MCP client libraries on macOS prefer; HTTP+SSE is the safer bet.
- No auth in v1 (per ADR / deployment view — local-network trust on Turbo).
- Existing MCP integration tests still pass — stdio remains the default when `--port` is not given.

## Test plan

- Unit: argument parsing (`--port` optional).
- Integration: spawn server with `--port`, connect a TCP client, exercise `initialize` + `tools/call search` against a populated collection. Mirrors the existing stdio MCP smoke test.
- Fleet test: `librarian start <name> --config …` then connect from a separate process to the supervised port; `librarian stop <name>` closes the listener.
