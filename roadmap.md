# Librarian roadmap

Status at **v1.1.0** (2026-06-07). Production = `main`, served by `librarian-serve` on turbo
(currently a `nohup` process; the running binary is the production tool).

## Done (v1.1.0)
- Canonical corpus at `/data/corpus` (issue 029); Marker decoupled from ingest (030)
- Recursive markdown chunker (027); both collections rebuilt and quality-scored
- Quality instrumentation (028): Tier-0 confidence, Tier-2 health, Tier-1 judge; ingest gate
  (`docs/quality-standard.md`)
- Validated task-conditioned usage recipe (031), deployed in the `cw:librarian` skill
- Retrieval confidence reported to the user (skill duty)

## Lines of development (parallel, branched off `main`)

### Line 0 — Protect production
- [x] Merge to `main` + release **v1.1.0**
- [ ] Supervise the daemon (systemd `Restart=always`) — deferred; running code is production for now
- [ ] Budget isolation: a separate OpenAI key/cap for experiments so a batch job cannot drain
      the live query quota (lesson from the 2026-06-07 quota incident)
- [ ] Drop `pp_test_*` experiment collections from qdrant

### Line A — Public access (branch `feat/cloudflare-access`) — issue 032
Cloudflare Tunnel + Access (free tier confirmed ≤50 users). Bearer-key auth, hot-reloadable
per-user rate limits, multi-channel ingress. Keyed-and-invited only (copyrighted excerpts).

### Line B — Telemetry (branch `feat/telemetry`) — issue 033  **[in progress]**
What to log and how to use it. The toolchain is solved; the value is in *which signals*.
Method: 50-agent open-web literature survey -> exploratory experiments -> validated telemetry
design. Request log -> `librarian stats` -> weak-query acquisition loop + confidence
recalibration.

## Shelved / non-issues
- Contextual retrieval: pilot inconclusive (baseline at ceiling); revisit with real queries from Line B
- Hybrid sparse+dense retrieval: untested; highest-value for identifier-heavy queries; test with real queries
- `chunk_index` "bug": non-issue (self-inflicted in the pilot's Python upsert); production is correct

## Standing constraints
- Generation (enrichment, judging) -> Anthropic subscription; vectors/queries -> OpenAI. Never mix budgets.
- New corpus additions must pass `docs/quality-standard.md`.
- Copyrighted excerpts: keyed-and-invited only, never public.
