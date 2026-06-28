# Project Context — big-token-saver (`bts`)

> **Produced:** 2026-06-18 by `map-codebase` — cold analysis of the live codebase.
> **Canonical source of architectural truth.** Regenerate after major structural changes.

## Stack

| Layer | Technology | Version | Role |
|-------|-----------|---------|------|
| **Dispatcher** | Bash 5+ | — | Verb routing, config merging, tool delegation |
| **Repo map engine** | Rust (edition 2021) | 1.85+ | Tag extraction + PageRank + token budgeting |
| **Installer** | Mise | 2026.6.x | Cross-ecosystem toolchain bootstrap |
| **Tag parsing** | tree-sitter | 0.24.6 | AST parsing for 10 languages |
| **Graph** | petgraph | 0.6 | Directed graph + hand-rolled PageRank |
| **Token counting** | tiktoken-rs | 0.5 | cl100k_base encoder for budget enforcement |
| **CLI** | clap | 4 (derive) | Argument parsing for bts-map binary |
| **Serialization** | serde + serde_json | 1 | Feedback DB persistence |
| **File walk** | walkdir | 2 | Directory traversal with exclusion rules |
| **Snapshot testing** | insta | 0.1 (dev) | Byte-stable output verification |

### Grammar crates (pinned, critical coupling)

| Language | Crate | Version | .scm source |
|----------|-------|---------|-------------|
| Rust | tree-sitter-rust | 0.23.2 | vendor/queries/rust-tags.scm |
| Python | tree-sitter-python | 0.23.6 | vendor/queries/python-tags.scm |
| TypeScript | tree-sitter-typescript | 0.23.2 | vendor/queries/typescript-tags.scm |
| JavaScript | tree-sitter-javascript | 0.23.1 | vendor/queries/javascript-tags.scm |
| Go | tree-sitter-go | 0.23.4 | vendor/queries/go-tags.scm |
| Java | tree-sitter-java | 0.23.5 | vendor/queries/java-tags.scm |
| Ruby | tree-sitter-ruby | 0.23.1 | vendor/queries/ruby-tags.scm |
| C | tree-sitter-c | 0.23.4 | queries/c-tags.scm (divergent) |
| C++ | tree-sitter-cpp | 0.23.4 | queries/cpp-tags.scm (divergent) |
| Swift | npezza93-tree-sitter-swift | 0.4.4 | queries/swift-tags.scm (divergent, fork) |

> **⚠️ Swift uses a non-canonical fork** (`npezza93-tree-sitter-swift`) because the official
> `tree-sitter-swift` 0.7.3 ships ABI v15, incompatible with tree-sitter core 0.24.6 (ABI v14).
> This is tracked with `SWIFT-FORK-TRACKING` comments. Revisit when bumping tree-sitter to ≥0.25.
>
> **⚠️ C, C++, Swift .scm files live in `queries/` (not `vendor/queries/`)** — they are divergent
> from the verbatim aider upstream. `vendor/queries/` is pure verbatim.

## Architecture

### High-level shape

```
                    ┌─────────────────────────────┐
                    │         bin/bts (bash)        │
                    │  setup | doctor | init        │
                    │  config | find | src | docs   │
                    │  ai | compress | map           │
                    └──────┬──────────────────────┘
                           │ shells out to:
        ┌──────────────────┼──────────────────┐
        ▼                  ▼                  ▼
   mise / sqz / rtk    rg+fzf+bat       bts-map (Rust)
   (toolchain mgmt)   (code search)    (repo map engine)
```

Three tiers:
1. **Shell dispatcher** (`bin/bts`, ~500 lines) — verb router with `case` switch. Each verb delegates to an installed tool or the Rust binary. Graceful degradation: missing tools print hints pointing to `bts doctor`.
2. **Rust binary** (`crates/bts-map/`, ~600 lines) — the only compiled, owned artifact. CLI via clap.
3. **Installed tools** — 12 tools from mise (rg, fd, bat, eza, fzf, jq, gh, direnv, opensrc, aider, rtk, sqz). Never bundled; declared in mise.toml.

### Data flow: `bts map`

```
walkdir traverse          tree-sitter parse         .scm tag query
  (skip .git/,               (per-language          (name.definition.*
   target/, etc.)             grammar)                → Def,
                                                      name.reference.*
                                                      → Ref)
        │                         │                       │
        ▼                         ▼                       ▼
   PathBuf list              Concrete syntax tree      Vec<Tag>
                                                      {rel_fname, name,
                                                       line, kind, node_kind}
                                                              │
                                                              ▼
                                                     petgraph DiGraph
                                                     (file node, edge weight
                                                      = shared ident count)
                                                              │
                                                              ▼
                                                     hand-rolled PageRank
                                                     (α=0.85, 50 iterations,
                                                      deterministic)
                                                              │
                                                              ▼
                                                     Vec<RankedFile>
                                                     {rel_fname, score,
                                                      raw_score, feedback_factor}
                                                              │
                                                              ▼
                                                     tiktoken-rs cl100k_base
                                                     (greedy budget: include
                                                      files until token limit)
                                                              │
                                                              ▼
                                                     String (rendered map)
```

### Config architecture (e07s01)

Three-layer merge, implemented in bash with awk-based TOML parsing:

```
env vars (lowest priority)
  └─ global ~/.config/bts/config.toml
       └─ project .bts.toml (highest priority)
```

Keys: `model`, `token_budget`. Verbs: `bts config [show|get|set|init]`.

### Feedback / regret learning (e07s01)

```
bts map feedback --good <file>     bts map feedback --bad <file>
        │                                   │
        ▼                                   ▼
  ~/.config/bts/map-feedback.json   (JSON: {"files": {"src/a.rs": {"good": 3, "bad": 1}}})
        │
        ▼
  FeedbackDb::factor(fname) = 1.0 + α * (good - bad) / (good + bad + 1)
        │  α = 0.3
        ▼
  PageRank score *= factor   (boost good files, penalise ignored ones)
```

## Conventions (observed from code)

### Error handling

- **Shell**: `set -euo pipefail` at top; `_require <bin>` guard function checks command availability and exits 1 with a hint. Verbs print to stderr on failure, stdout on success.
- **Rust**: `anyhow::Result` in `main()` for top-level errors; `thiserror` derive for library error types (`MapError`, `FeedbackError`). Warnings go to stderr via `eprintln!`. File I/O errors are caught and reported per-file (not fatal).

### Type system

- **Rust**: `#![deny(unsafe_code)]` — zero unsafe blocks. Strong typing throughout. No `any` equivalents.
- **Shell**: Untyped; shellcheck enforces quoting, `-n` checks, and common pitfalls. No `eval` of untrusted input.

### Testing strategy

- **Rust**: 19 unit tests in `main.rs` (inline `#[cfg(test)] mod tests`). Per-language def/ref gates for all 10 languages. Determinism test (byte-stable output). Budget test (never exceeds limit). Snapshot test (fixture repo + insta). Feedback tests (boost, penalise, noop, reorder). Integration tests for walkdir exclusion rules.
- **Shell**: Smoke tests in `mise.toml` `[tasks.test]` — assert_ok/assert_eq pattern. Tests cover compress --stdin regression, help/version exit codes, and bts-map test pass-through.
- **CI**: `cargo clippy -- -D warnings`, `shellcheck install.sh bin/bts`, `cargo test -p bts-map` on every push. Full installer check on workflow_dispatch.

### API / CLI conventions

- **Shell verbs**: `bts <verb> [args…]`. Unknown verb → usage + exit 1. `-h`/`--help` for usage, `-V`/`--version` for version.
- **Rust binary**: `bts-map --budget N [--feedback FILE] [path]`. Structured with clap derive. Deterministic output for fixed input.

### File organization

```
bin/bts                              # single-file shell dispatcher
crates/bts-map/
  Cargo.toml                         # dependencies + grammar pins
  src/
    main.rs                          # CLI + walkdir loop + all tests
    tags.rs                          # Lang enum, language_for, scm_for, extract_tags
    graph.rs                         # rank_files, hand-rolled pagerank
    render.rs                        # budget-bounded skeleton renderer
    feedback.rs                      # FeedbackDb (regret learning)
    error.rs                         # MapError enum (thiserror)
  vendor/queries/                    # verbatim aider .scm (7 languages)
    rust-tags.scm, python-tags.scm, typescript-tags.scm,
    javascript-tags.scm, go-tags.scm, java-tags.scm, ruby-tags.scm
    c-tags.scm, cpp-tags.scm, swift-tags.scm  # stale verbatim copies
  queries/                           # divergent .scm (C, C++, Swift)
    c-tags.scm, cpp-tags.scm, swift-tags.scm
  tests/fixtures/                    # multi-language fixture repo + snapshot
mise.toml                            # toolchain manifest (source of truth)
install.sh                           # mise bootstrap + manifest application
package.json                         # npm distribution metadata (private)
```

## Signals / Active Considerations

### 1. npm distribution pending (e06s03)
`package.json` exists but is `"private": true` with no `bin` entry. CI builds bts-map for 3 platforms but only attaches to GitHub Releases. Story queued: add npm publish + npx zero-install.

### 2. Swift fork tracking (supply chain risk)
`npezza93-tree-sitter-swift` is a non-canonical personal fork. Blocked on tree-sitter core ≥0.25 (would allow official `tree-sitter-swift` with ABI v15). Tracked with `SWIFT-FORK-TRACKING` comments and `deny.toml`.

### 3. .scm vendoring split
Two `.scm` directories: `vendor/queries/` (verbatim aider, 10 files) and `queries/` (divergent, 3 files). The stale verbatim copies for C/C++/Swift in `vendor/` are NOT used at runtime — `tags.rs` points `C_SCM`/`CPP_SCM`/`SWIFT_SCM` to `../queries/`. Risk: someone might "clean up" the divergent queries thinking they're duplicates.

### 4. No Windows support
CI builds for macOS ARM/Intel and Linux x86_64 only. No `x86_64-pc-windows-msvc` target. The shell dispatcher requires bash (unavailable natively on Windows). Not currently a requirement.

### 5. Snapshot regeneration is manual
`GENERATE_SNAPSHOT=1 cargo test -p bts-map snapshot` must be run explicitly. No auto-regeneration on CI. Good guardrail, but easy to forget when changing render output.

### 6. PageRank determinism is hand-rolled
petgraph ships no built-in PageRank. The hand-rolled power iteration (50 fixed iterations, no convergence check) is deterministic but wastes compute on converged graphs. Alpha (damping) = 0.85 matches NetworkX default.

### 7. Single-file Rust test module
All 19 tests live in `main.rs` as an inline `#[cfg(test)] mod tests`. No separate test files. Works for current size (~700 lines of tests) but will need splitting if tests grow.

### 8. Shell config parsing is awk-based
TOML parsing for merged config uses awk with basic pattern matching. Sufficient for the two keys (`model`, `token_budget`) but no nested table support. A real TOML parser would be needed if config grows complex.
