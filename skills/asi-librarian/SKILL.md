---
name: asi-librarian
description: >-
  Search the shared reference library (software-engineering & CS books, and
  particle-physics / detector papers) via the `librarian` CLI. Use this WHENEVER
  you need to ground a claim, cite a book or paper, look up a definition, API,
  threshold, algorithm, or method, or the user asks "what does X say about Y" or
  wants references — even if they don't say "search the library". Prefer it over
  answering reference questions from memory.
---

# ASI Librarian — reference search via the CLI

The reference corpus is served read-only at `https://asi-librarian.com`, queried with
the `librarian` CLI. Searching the corpus before you assert something is the whole
point: the books and papers are the source of truth, and a real citation beats a
confident paraphrase from memory.

## Setup (once)

You need the CLI and two environment variables (the key comes from the library
operator — see the colleague-access runbook in the repo). Any OS works; the CLI
builds with Rust's `cargo` (install via https://rustup.rs if you don't have it):

```
git clone git@bitbucket.org:amscins/librarian.git   # or: https://github.com/CiaranWelsh/librarian
cd librarian
cargo install --path crates/cli
```

Then set two environment variables, persistently for your platform:

- Linux / macOS (shell profile, e.g. `~/.bashrc` or `~/.zshrc`):
  ```
  export LIBRARIAN_DAEMON=https://asi-librarian.com
  export LIBRARIAN_KEY=<your personal key>
  ```
- Windows (PowerShell, persists for your user account):
  ```
  [Environment]::SetEnvironmentVariable("LIBRARIAN_DAEMON","https://asi-librarian.com","User")
  [Environment]::SetEnvironmentVariable("LIBRARIAN_KEY","<your personal key>","User")
  ```

The key is a personal bearer token: treat it like a password, never commit it.

## Search

```
librarian query <collection> "<natural-language query>" --limit <N>
```

- **collections:**
  - `software` — software engineering, CS, GPU/parallel computing, networking, ML,
    microservices, Rust/Qt/TypeScript, and detector manuals (e.g. Timepix4).
  - `particle-physics` — detector and particle-physics papers (Timepix, TPX3Cam,
    time-of-arrival, ion trapping, etc.).
- `--limit` defaults to 5; **use 8 as your working default**; up to 20 only for hard
  single-topic lookups — never for synthesis (see recipe).
- The query is embedded and matched semantically — phrase it as the *concept* you
  want, not bare keywords.

## The validated usage recipe

Measured in controlled experiments (two rounds, cross-model judged). Condition your
strategy on the TASK:

| Your task | Strategy (measured winner) |
|---|---|
| **Fact / API / definition lookup** | 1 search, verbatim question, `--limit 8`, quote-first |
| **Answer spans two sources** (compare a value here with a mechanism there) | **Decompose first**: one search per sub-question, then combine. A single query starves — it abstained on 88% of two-hop questions |
| **Synthesis / survey / trade-offs** | **3 perspective queries × limit 8**, merge mentally — never one giant `--limit 20` pull (measured worst: context dilution) |
| **Maths / theory / derivations** | **HyDE**: write the one-sentence *target result statement* yourself and search with THAT, not the question |
| **Explaining / teaching a concept** | Retrieve `--limit 8` for grounding, then **explain in your own words with an example**, citing sources — verbatim quote-stitching is the measured *worst* teaching pattern |

Always, regardless of task:
- **Verbatim natural questions beat keyword rewrites** (measured; keyword queries lost
  on every metric) — except the maths/HyDE case above.
- **The abstention contract is the 0%-hallucination floor**: if the passages don't
  contain the answer, say "the corpus doesn't cover this" and stop — closed-book
  answering hallucinated on 12% of in-corpus questions and 100% of absent-topic ones.
- Cite the `source_id` for every claim you take from a chunk.
- **Report the retrieval confidence to the user** — every query prints a `confidence:`
  line; it is your honest signal of how well-grounded the answer is, and the user
  can't see it unless you pass it on.

## Getting good results — search to *locate*, extract to *read*

Top-k search returns the most *similar* chunks, and chunks are small — often a single
heading or sentence. So `librarian query` alone gives you **pointers, not coherent
passages**. Two patterns turn it into reliable information:

**1. Phrase the query as a question or concept, not a keyword list.** A keyword query
matches section *headings* — high score, no content. A question matches explanatory
*prose*. Question hits often score slightly lower but carry the actual answer —
**score is similarity, not usefulness.**

**2. Locate the best source, then extract its surrounding chunks to read.** Pick the
most relevant hit and paste its full `source_id#chunk_index` token straight into
`extract` — `--context N` pulls N chunks either side, in order, reconstructing the
original passage:

```
librarian extract <collection> "<source_id>#<index>" --context 5
```

**Rules of thumb**
- `--limit` widens *breadth* (more sources), not *depth*. For depth, extract a window.
- Skim 3–5 hits to find the best *source*, then extract that one region, rather than
  stitching tiny fragments from many books.
- If every hit is a bare heading, the query is too keyword-y — rephrase as a question.

## Reading the output

```
[0.561] software/markdown/networking-dordal/chapter-19-queuing-and-scheduling.md#53
  ...the idea behind a token bucket is that there is a notional bucket somewhere...
```

- `[score]` — cosine similarity (0–1); higher is closer. Treat below ~0.3 as weak.
- `<source_id>#<chunk_index>` — the source and chunk. **Cite the source_id** when you
  use the material so the user can trace it.
- Read several hits — the top one isn't always the most useful passage.

## Report retrieval confidence to the user

Every query ends with a retrieval-confidence line:

```
confidence: STRONG (0.51)  [top 0.512, margin 0.041, fragments 0%]
```

- `STRONG` — a close, distinguishable, substantial top hit. Trust it; cite normally.
- `WEAK` — **the common, conservative case.** It fires whenever the top hits aren't
  cleanly distinguishable or are fragmentary — *even when the answer is genuinely
  present*. `WEAK` means "verify", not "wrong".
- `LIKELY_NO_ANSWER` — probably out-of-corpus. Treat a low top score together with
  visibly off-topic text as the real out-of-corpus signal.

This is a **coarse triage signal, not a precise grade** — the label is a prior; your
reading of the retrieved text is the verdict. Pass an honest confidence assessment on
to the user: they can't see the retrieval, so your report is their only signal.
- Strong + on-topic → present and cite normally.
- Thin / fragmentary / stitched scraps → say so: "the library only weakly supports
  this — treat it as a lead, not a settled fact."
- Off-topic hits → abstain: tell the user the corpus doesn't appear to cover this.

## Errors

- `401 unauthorized` — `LIBRARIAN_KEY` missing, mistyped, or revoked. Check the env
  var; ask the operator if it persists.
- `429 rate_limited` — per-key rate limit (60 req/min); wait for the `Retry-After`
  and slow down. Batch your thinking, not your requests.
- Connection errors — the service or your network is down; say so plainly rather
  than silently falling back to memory.
