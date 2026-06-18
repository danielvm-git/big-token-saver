# ADR-0004: Shell-first unifier; Rust only for `bts map`

- **Status:** Accepted (2026-06-15)

## Context

The unifier is glue: subcommand routing, tool detection, opinionated default flags. The
temptation (from the dropped product design) was a multi-crate Rust workspace aliasing
`rg`, `fzf`, etc. That is over-engineering for shelling out to existing binaries, and it
slows the edit-test loop to a compile cycle.

Exactly one part has real computation: `bts map` (graph + PageRank + tree-sitter parsing).

## Decision

The dispatcher and all verbs are **bash** (`bin/bts`, ~100 lines + verb functions),
`shellcheck`/`shfmt` clean. **Rust is used only for `crates/bts-map`.** No Rust workspace
to wrap shell-able tools.

## Consequences

- **+** Fast to write, read, and change; degrades gracefully per verb (`command -v`).
- **+** Keeps the compiled surface tiny — one crate to pin, audit, and cross-compile.
- **−** Bash limits structured logic; acceptable because verbs are thin delegations.
- Convention: subcommands `bts <verb>`, shell functions `bts_<verb>`, every external call
  guarded with a `bts doctor` hint on miss.
