# Coding Conventions — big-token-saver

The project is mostly a manifest + shell, with one small Rust crate. Conventions reflect that.

## Shell (the `bts` dispatcher + install.sh)

- `#!/usr/bin/env bash`, `set -euo pipefail` at the top of every script.
- Must pass `shellcheck` with no warnings.
- Prefer POSIX-portable constructs; bashisms only where they earn their keep.
- Every external tool call is guarded: check `command -v <tool>` and point the user at
  `bts doctor` if missing — never fail with a raw "command not found".
- User-facing output: concise, prefixed (`→` progress, `✓` success, `MISSING` problems).
- No secrets in scripts. Never echo env values that could be tokens.

## Manifest (mise.toml)

- One `[tools]` table is the single source of truth for the set.
- Use explicit backend prefixes (`npm:`, `pipx:`, `ubi:`) when a bare registry name is
  ambiguous or wrong (notably rtk/sqz → `ubi:`, never `cargo:`).
- Comment every non-obvious entry with its role and why that backend.
- Pin to `"latest"` for tools that auto-update cleanly; pin exact versions only when a tool
  has broken you before.

## Rust (crates/bts-map — the only compiled code)

- Edition 2021; MSRV 1.80 stable; no nightly.
- `#![deny(unsafe_code)]`; `cargo clippy -- -D warnings` must pass.
- Errors: `thiserror` typed enum at public boundaries; `anyhow` only in `main`.
- Deps kept minimal: `tree-sitter` + per-language grammar crates (version-pinned to match
  the vendored `.scm`), `petgraph`, `tiktoken-rs`, `ignore`, `clap`.
- **No `sqz-engine`, no `rtk`, no networked deps.** If `bts map` ever wants compression, it
  shells out to the installed `sqz` binary — it does not link it.

## Vendored files (crates/bts-map/vendor/queries/)

- Each `.scm` keeps a provenance header (source repo + license).
- `crates/bts-map/NOTICE` reproduces per-grammar attribution — the queries are an MIT/Apache
  MIX inherited via aider, not a single license. Do not relabel them monolithically.
- Don't edit vendored queries in place; copy to `crates/bts-map/queries/` with a divergence
  comment if a fix is needed.

## Naming

| Thing | Convention | Example |
|---|---|---|
| Subcommands | `bts <verb>` | `bts setup`, `bts find`, `bts map` |
| Shell functions | `bts_<verb>` | `bts_doctor`, `bts_init` |
| Rust crate | `bts-map` | — |
| Config files | `.bts.toml` (project), `mise.toml` (toolchain) | — |

## Git

- [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/).
  Types in use: `feat:`, `fix:`, `ci:`, `docs:`, `chore:`, `refactor:`, `test:`, `build:`, `perf:`.
- Branch per epic: `e01/installer`, `e02/dispatcher`, `e05/bts-map`.
- Before merge: `shellcheck` clean; if bts-map touched, `cargo clippy -- -D warnings` + `cargo test`.
