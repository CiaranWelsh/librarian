# Runbook: colleague access to the library

The reference library is served read-only at **`https://rag-librarian.com`** (Cloudflare
Tunnel → keyed query daemon on turbo). Access is keyed-and-invited: every person gets
their own bearer key, issued by the operator. There is no self-service signup, by design
(the corpus contains copyrighted excerpts).

## For colleagues — setup (2 minutes)

You need two values from the operator: the URL above and **your personal key**.

### Option A: the `librarian` CLI (recommended)

Works on Linux, macOS and Windows. Needs Rust's `cargo` (https://rustup.rs).
Clone the repo (Bitbucket with your amscins account, or the GitHub mirror) and
build the CLI:

```bash
git clone git@bitbucket.org:amscins/librarian.git   
cd librarian
cargo install --path crates/cli
```

Set two environment variables, persistently for your platform.

Linux / macOS (shell profile, e.g. `~/.bashrc` or `~/.zshrc`):

```bash
export LIBRARIAN_DAEMON=https://rag-librarian.com
export LIBRARIAN_KEY=<your key>
```

Windows (PowerShell):

```powershell
[Environment]::SetEnvironmentVariable("LIBRARIAN_DAEMON","https://rag-librarian.com","User")
[Environment]::SetEnvironmentVariable("LIBRARIAN_KEY","<your key>","User")
```

Use it:

```bash
librarian query software "how does a token bucket rate limiter work?" --limit 8
librarian query particle-physics "timepix4 time of arrival calibration"
librarian extract software "<source_id>#<chunk>" --context 5   # read around a hit
```

Collections: `software` (SE/CS books + detector manuals) and `particle-physics`
(detector/physics papers).

### Option B: plain HTTP (no install)

```bash
curl -s -H "Authorization: Bearer $LIBRARIAN_KEY" -H 'content-type: application/json' \
  -X POST https://rag-librarian.com/v1/search \
  -d '{"collection":"software","query":"...","limit":8}'
```

Endpoints: `POST /v1/search`, `GET /v1/documents?collection=`, `POST /v1/extract`,
`GET /v1/collections`.

### Optional: teach your Claude to use it (Claude Code skill)

A ready-made skill teaches Claude Code when and how to search the library (query
strategy, citing source_ids, reporting retrieval confidence). Copy
[`claude-skills/rag-librarian/SKILL.md`](../../claude-skills/rag-librarian/SKILL.md) from your clone
into your Claude Code skills folder.

Linux / macOS (from the repo root):

```bash
mkdir -p ~/.claude/skills/rag-librarian
cp claude-skills/rag-librarian/SKILL.md ~/.claude/skills/rag-librarian/
```

Windows (PowerShell, from the repo root):

```powershell
New-Item -ItemType Directory -Force "$env:USERPROFILE\.claude\skills\rag-librarian" | Out-Null
Copy-Item claude-skills\rag-librarian\SKILL.md "$env:USERPROFILE\.claude\skills\rag-librarian\"
```

Restart Claude Code; it picks the skill up automatically when reference questions
come up (or invoke it directly with `/rag-librarian`).

### Key etiquette

- The key is a bearer token: **anyone holding it is you.** Keep it in your shell profile
  or a password manager, never in committed code, scripts or shared docs.
- Don't share your key — keys are per-person so usage can be attributed and revoked
  individually.
- Rate limit is 60 requests/minute per key (HTTP 429 + Retry-After when exceeded).
- Lost or leaked? Tell the operator; revocation is instant and you get a fresh key.

## For the operator — issuing and revoking keys

Keys live in `turbo:~/.librarian/keys.toml` (mode 600). The daemon **hot-reloads** the
file on save: no restart for any of these.

**Issue a key** (run on turbo):

```bash
python3 - <<'EOF'
import secrets
print(f'\n[keys."lib_{secrets.token_hex(16)}"]\nuser = "REAL-NAME-HERE"')
EOF
# append the printed block to ~/.librarian/keys.toml, set the real name
```

Send the key over a secure channel (password-manager share or similar — not plain
email/chat if avoidable).

**Revoke**: delete that `[keys."…"]` table from keys.toml. Takes effect on next request.

**Rotate**: revoke + issue. **Per-user rate override**: add `rpm = N` inside the table.

## Security model (what protects what)

| Layer | Guarantee |
|---|---|
| Cloudflare edge | TLS to the visitor; turbo's IP never exposed |
| Tunnel ingress | Only `localhost:6701` is reachable; everything else 404s at the edge |
| Bearer auth (fail-closed) | No key → 401 before any handler runs; missing keys.toml rejects all |
| Read-only daemon | The exposed binary contains no write path — search/list/extract only. Writes happen only via the CLI on turbo itself (quality-gated `librarian add`) |
| Rate limit | 60 rpm/key token bucket: abuse/runaway guard |
| Access log | `turbo:~/.librarian/access.jsonl` — one JSONL line per request: ts, user, route, status, ms (+ collection/query/confidence for searches) |

Worst case of a leaked key: read access to the library and embedding cost per query,
until revoked. No write or delete is possible through the public surface.

## Monitoring traffic

```bash
ssh asi@turbo "tail -f ~/.librarian/access.jsonl"                                  # live
ssh asi@turbo "jq -r '.user // \"anon\"' ~/.librarian/access.jsonl | sort | uniq -c" # per-user volume
ssh asi@turbo "jq -r 'select(.status==401) | .ts' ~/.librarian/access.jsonl | wc -l" # knocking randoms
```
