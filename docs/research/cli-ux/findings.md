# CLI design — research findings (feat/cli-ux)

Online research (fan-out, 2026-06-09) into what makes a CLI usable, precise, and correct, then
mapped to the `librarian` CLI. Optimised for humans; kept clean for scripts/AI agents.

**Primary sources:** [clig.dev](https://clig.dev/) · [12 Factor CLI Apps](https://jdxcode.medium.com/12-factor-cli-apps-dd3c227a0e46)
· [POSIX Utility Conventions §12](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap12.html)
· [GNU CLI Standards](https://www.gnu.org/prep/standards/html_node/Command_002dLine-Interfaces.html)
· [no-color.org](https://no-color.org/) · [XDG Base Dir](https://specifications.freedesktop.org/basedir-spec/latest/)
**Exemplars studied:** git, `gh`, ripgrep, `fd`, cargo, kubectl, `fzf`, `bat`.
**UX sources:** Thoughtworks CLI guidelines, Atlassian "delightful CLIs", NN/g error messages, PatternFly CLI handbook, lucasfcosta UX patterns.

The four research briefs (full bullet lists + per-tool analyses) are summarised below by theme.
Each theme states the principle, why it matters, the source, and the **librarian application**.

---

## 1. Configuration & context — stop retyping what doesn't change
- **Precedence is law: flags > env vars > config file > built-in default.** Explicit flags must
  always win; never invert. (clig.dev #configuration; 12factor III)
- **Use an env var for things that vary by *where* you run, not *what* you do** — a remote daemon
  URL is exactly that. (clig.dev #environment-variables)
- **librarian:** add `LIBRARIAN_DAEMON` (clap `env=`); `--daemon` overrides it; `localhost:6700`
  stays the last-resort default. Kills the "type `--daemon http://turbo:6700` every call" pain.
  Optional later: `~/.config/librarian/config.toml` (XDG) for persistent defaults.

## 2. Streams & exit codes — the composability contract
- **stdout = primary data; stderr = everything else** (progress, warnings, errors). A status line
  leaking into stdout breaks every `| jq`. (clig.dev #output; Unix convention)
- **Exit 0 iff success; non-zero on any failure; distinct codes where useful** (e.g. 2 = usage
  error, deterministic don't-retry). Agents and scripts branch on `$?`. (clig.dev #exit-codes; sysexits.h)
- **Empty results are not an error** — exit 0 (or a distinct code), not an error message. (ripgrep GUIDE)
- **librarian:** today everything prints to stdout via `println!`. Move messages/errors to stderr,
  keep results on stdout, set meaningful exit codes.

## 3. Human vs machine output — one tool, two audiences
- **Detect the audience: `isatty(stdout)`.** TTY → color, alignment, hints. Pipe → plain text, no
  ANSI, no spinner. (clig.dev; bat, rg, fd, git, cargo all do this)
- **`--json` is the stability contract for machines**; human output may change freely between
  versions, `--json` may not. Field names are public API. (clig.dev; gh `--json`, cargo `--message-format=json`)
- **Color signals meaning, never decorates; honor `NO_COLOR`, `TERM=dumb`, `--no-color`.** (no-color.org; clig.dev)
- **librarian:** add `--json` (stable schema for the hit/chunk records); TTY-gate color; honor `NO_COLOR`.

## 4. Help & discoverability — the help *is* the manual
- **Lead help with a concrete example, not a usage template** — newcomers read examples first. (clig.dev; bettercli.org)
- **Bare invocation and no-required-args both print grouped help**, never a silent no-op or a terse
  usage line. (gh, git, cargo)
- **Top-level `after_help`: a "what next" workflow tip** — the one place for cross-command guidance. (clig.dev; Atlassian)
- **Consistent verb/noun grammar so users guess correctly** (`--limit` always means count, etc.). (kubectl, lucasfcosta)
- **librarian:** real `about`; per-subcommand examples; `after_help` documenting locate→extract;
  `librarian` alone lists subcommands. (The 25 future invitees won't have the `cw:librarian` skill.)

## 5. Error messages — what failed + why + how to fix
- **Three-part errors:** context (what op), cause (what broke), resolution (what to try). Not a bare
  code or stack trace. (Thoughtworks; NN/g; clig.dev #errors)
- **Distinguish network failure modes:** connection refused ("is it running?") vs timeout ("is the
  host reachable?") vs non-200 ("daemon returned 503: …"). Each needs a different user action.
- **Did-you-mean on typos; never silently auto-run the guess.** (git autocorrect; clig.dev)
- **No raw stack traces by default — `--verbose` opt-in.** (clig.dev)
- **librarian:** the daemon error-envelope decode is decent; add a "fix" line and the three distinct
  connection diagnostics.

## 6. The `extract` pain point — never make the user transform an ID
- **Clean/normalise input at the entry point, not at the call site.** Two independent briefs flagged
  the exact `extract` workflow (copy id → strip `#idx` → hand-compute `--start/--end`) as a textbook
  violation of "make the default the right choice". (clig.dev defaults; Thoughtworks)
- **librarian:** `extract` accepts the full `source_id#idx` token `query` prints; the `#idx` becomes
  the window centre; `--context N` widens it; with no window, show the referenced chunk. `--start/--end`
  remain as explicit overrides.

## 7. Network robustness — never hang forever
- **Always set an explicit timeout** (OS default backoff is ~127s). ~5s connect, ~30s read for a LAN
  daemon; expose `--timeout`. (evanjones TCP timeouts; clig.dev)
- **librarian:** the blocking client has no timeout today — a wedged daemon hangs the CLI. Add it.

## 8. Result presentation — built for scanning
- **Highlight the matched query term within each result**; users scan, they don't read. (rg, fd, fzf)
- **Aligned plain columns, not ASCII box tables** — scannable *and* grep-able. (PatternFly)
- **Most important line last** (terminal eye lands at the bottom); meta-info (timing, counts) dim or
  behind `--verbose`. (clig.dev)
- **Default result cap with `--limit`** so a broad query doesn't flood the terminal. (gh defaults to 30)
- **librarian:** already has `--limit 5`. Add term highlight (TTY only), tidy alignment, keep the
  confidence line but make it dim.

## 9. Follow-up suggestions — replace the docs for step 2
- **After a successful command, suggest the logical next command.** (clig.dev; gh, Atlassian)
- **librarian:** after `query`, print a ready-to-paste `extract` line for the top hit (TTY only).

## 10. Verbosity — `--quiet` / `--verbose`
- **`-q/--quiet`** suppresses non-result output (scripts/agents); **`-v/--verbose`** adds timing,
  request detail; `--debug`/`DEBUG` env for raw bodies. (clig.dev)

## 11. Stability — the CLI surface is a public API
- **Never rename/remove a flag or subcommand without deprecation; add freely.** librarian is used in
  agent prompts and the `cw:librarian` skill, so **every change here must be additive**
  (new env var, new `--json`, smarter `extract`) — existing invocations keep working. (clig.dev; MS .NET)

## 12. Shell completion & agent-friendliness (nice-to-have)
- **`librarian completions <bash|zsh|fish>`** generated by `clap_complete` (auto-stays-in-sync). (kbknapp.dev)
- **Never block on a prompt when stdin isn't a TTY**; provide `--yes`/`--force`. Make repeatable ops
  idempotent. (clig.dev; AI-agent CLI guidance)
- **librarian:** read-only today, so prompts are mostly moot; relevant if `ingest`/`remove` grow prompts.

---

## What this means for our CLI (supersedes issues 035 + 036)

Phased by human value, additive-only:

- **Phase 1 — human daily friction:** `LIBRARIAN_DAEMON` env (#1) · `extract` accepts `sid#idx` +
  `--context`, defaults to the referenced chunk (#6) · help: examples + `after_help` + bare→help +
  real `about` (#4) · network timeout + distinct connection errors + fix lines (#5,#7) · `query`
  prints a follow-up `extract` hint (#9).
- **Phase 2 — correctness + dual audience (cheap, makes it *right*):** stdout/stderr split +
  meaningful exit codes (#2) · `--json` with a stable typed schema (#3, ties to 036 typed structs) ·
  TTY-aware color + `NO_COLOR` + matched-term highlight + alignment (#3,#8) · `--quiet`/`--verbose`
  (#10) · shared HTTP client underpinning timeout+json (036).
- **Phase 3 — polish:** shell completion (#12) · smart-case · progress indicator on slow calls ·
  optional XDG config file (#1).

Constraint throughout: **additive only** (#11) — no existing invocation may break.
