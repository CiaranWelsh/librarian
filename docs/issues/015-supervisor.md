# 015 — Supervisor + fleet registry

**Phase:** F · **Status:** Open · **Deps:** 014

## Goal

Implements **F-9.1 – F-9.4, QA-O1**: a supervisor process on Turbo manages the fleet of `librarian-collection` servers. CLI `start` / `stop` / `restart` / `status` route through it.

## Acceptance criteria

- Fleet registry: SQLite file under `/var/lib/librarian/`. Schema: name, config path, MCP port, child PID, status, uptime, last-ingest timestamp.
- Supervisor is a pure orchestration binary — no dependency on `librarian-domain` traits.
- `librarian-supervisor` binary (or a `--supervisor` mode of `librarian`) — long-lived, owns the registry.
- `start <collection>` allocates a non-conflicting port, spawns `librarian-collection` with the config, registers it.
- `stop <collection>` signals the child, waits for clean exit, updates registry.
- `status` lists registered collections with running/stopped state, port, uptime.
- Idempotent: starting a running collection is a no-op; stopping a stopped one is a no-op.

## Test plan

- Integration: start two mock collections, `status` lists both with distinct ports; stop one, `status` reflects it; restart, port may change but registry is consistent.
- Crash test: kill child PID externally; next `status` invocation PID-checks and marks the entry stopped. No background heartbeat in v1.
