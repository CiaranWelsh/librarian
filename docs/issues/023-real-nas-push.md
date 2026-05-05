# 023 — Real NAS push (HTTPS or SCP)

**Phase:** J (post-v1) · **Status:** Open · **Deps:** 014

## Goal

Slice 014's AC said: "pushes the resulting file to NAS via HTTPS or SCP (configurable). No NFS mount." The current implementation copies to a local directory path. That path *can* be an NFS mount, which contradicts the slice's stated discipline. When Turbo and the NAS are different hosts without a mount, the snapshotter doesn't push anything across the network.

## Acceptance criteria

- `QdrantNasSnapshotter::open` gains a transport selector: local-dir (current), HTTPS PUT, or SCP. Drop a `kind = "local" | "https" | "scp"` field into the snapshot config; defaults to `local` for back-compat.
- HTTPS variant: PUT the snapshot file to a configurable URL (basic auth or bearer token via env). Tests use mockito.
- SCP variant: shell out to `scp -B`. Tests use a local SSH listener (or are gated on env). Config: target user@host:path, identity file path.
- `restore` is symmetric: HTTPS GET / SCP pull.
- `list` and `prune` work over HTTPS where the destination supports directory-style listing (otherwise fall back to a local mirror tracked in the manifest).

## Test plan

- HTTPS: mockito-driven PUT/GET tests.
- SCP: integration test gated on `LIBRARIAN_SCP_TARGET=user@host:/tmp/test` env var; skip if absent.
- Config parsing: invalid kind → human-readable error.

## Notes

For v1.0 the local-dir path is fine if the user has an NFS mount on Turbo (mounted at, e.g., `/mnt/nas/librarian/`). This slice exists to honour the slice-014 contract once the user wants the no-NFS deployment.
