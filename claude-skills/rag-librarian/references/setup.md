# Setup

Done once per machine. The CLI is a thin HTTP client. The index and the embedder live on
the server, so there is nothing to download beyond the CLI itself.

## 1. Get a key

Access is keyed and invited because the corpus contains copyrighted excerpts. Ask the
library operator for a personal bearer key (the issuing procedure is in
`docs/runbooks/colleague-access.md` in the repository). Treat the key as a password.
Keep it in your shell profile or a password manager, never in committed code, scripts
or shared documents. Anyone holding it is you, and keys are per person so that usage
can be attributed and revoked individually.

## 2. Install the CLI

Needs Rust's `cargo` (https://rustup.rs). Linux, macOS and Windows all work.

```
git clone https://github.com/CiaranWelsh/librarian.git
cd librarian
cargo install --path crates/cli
```

## 3. Set two environment variables

Linux / macOS, in your shell profile (`~/.bashrc` or `~/.zshrc`):

```
export LIBRARIAN_DAEMON=https://rag-librarian.com
export LIBRARIAN_KEY=<your personal key>
```

Windows, in PowerShell (persists for your user account):

```
[Environment]::SetEnvironmentVariable("LIBRARIAN_DAEMON","https://rag-librarian.com","User")
[Environment]::SetEnvironmentVariable("LIBRARIAN_KEY","<your personal key>","User")
```

Open a new shell afterwards so the variables are picked up.

## 4. Check it works

```
librarian list
```

prints the collections with their document and chunk counts. A `401` here means the key
is not set in this shell or was mistyped.

## No Rust? Use curl

The same key works against the HTTP API directly.

```
curl -s -H "Authorization: Bearer $LIBRARIAN_KEY" -H 'content-type: application/json' \
  -X POST https://rag-librarian.com/v1/search \
  -d '{"collection":"software","query":"how does a token bucket work?","limit":8}'
```

Endpoints are `GET /v1/collections`, `POST /v1/search`, `GET /v1/documents?collection=`
and `POST /v1/extract`.

## Limits

60 requests a minute per key. Exceeding it returns `429` with a `Retry-After` header.
Lost or leaked key? Tell the operator. Revocation is immediate and you get a fresh one.
