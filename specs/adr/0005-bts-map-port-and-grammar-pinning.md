# ADR-0005: `bts map` ports aider's repo map; pin grammars to the `.scm` vintage

- **Status:** Accepted (2026-06-15) · **Grounded in source:** 2026-06-16 · **Spike-validated:** 2026-06-17
- **Relates to:** [TECH_STACK §3–§4](../plans/TECH_STACK_LATEST.md) · [SPIKE-bts-map](../archive/spikes/SPIKE-bts-map.md)

## Context

`bts map` reimplements aider's repo map (`aider/repomap.py`): tree-sitter `.scm` tag
extraction → def/ref graph → PageRank → token-budgeted code skeleton. The deepest
technical risk is **tree-sitter ABI / query coupling**: a `.scm` query is written against
a specific grammar version. If a grammar crate drifts, the query **still compiles but
captures nothing → silent empty map, no error**. This is confirmed real — aider's own
`rust-tags.scm` differs across its two query dirs across grammar versions.

aider avoids this by pinning one atomic wheel (`tree-sitter-language-pack==0.13.0` on
`tree-sitter==0.25.2`). Rust has no atomic bundle; each grammar is a separate crate.

## Decision

1. **Port the algorithm faithfully** — `petgraph` + hand-rolled power-iteration PageRank
   (petgraph ships none), `tiktoken-rs`, `ignore`, and a reimplemented `TreeContext`
   skeleton renderer. Keep aider's edge-weight multipliers and personalization vector.
2. **Vendor `.scm` queries** with per-grammar provenance + a MIT/Apache-MIX `NOTICE`
   (never relabel monolithically).
3. **Pin each grammar crate to the `tree-sitter-language-pack v0.13.0` vintage** — source
   of truth is that tag's `sources/language_definitions.json`. Pin `tree-sitter` core to
   the matching ABI. **Never bump a grammar without re-vendoring its `.scm` from the same
   vintage.**
4. **Verify with a real query per language** — each fixture has ≥1 def and ≥1 ref; the
   test asserts non-empty `def` *and* `ref` tags for ≥10 languages. A compile-only test is
   insufficient (it passes on a silently-broken grammar).

## Consequences

- **+** Turns the silent-failure mode into a hard CI failure.
- **−** Coverage spans both aider query dirs (TypeScript only in `tree-sitter-languages`,
  Swift only in `tree-sitter-language-pack`) → the vendor layout must carry both.
- **−** Determinism: PageRank float ordering must be made byte-stable (fixed iterations +
  `(file, ident)` tie-break) for snapshot tests.
- Rated epic size **L**; defer until the shell unifier is in daily use.

## Validation (spike, 2026-06-17)

A throwaway spike ([SPIKE-bts-map](../archive/spikes/SPIKE-bts-map.md)) proved the approach end-to-end
for Rust/Python/TypeScript. Refinements this ADR now mandates:

- **Proven pins:** `tree-sitter = "=0.24.6"`, `tree-sitter-rust/python/typescript = "=0.23.{2,6,2}"`,
  plus **`streaming-iterator`** (`QueryMatches` is a `StreamingIterator`, not `std::Iterator`).
- **API reality:** grammars export a `LANGUAGE` constant (`.into()`), not `language()`; TypeScript
  has **no single entry point** — pick `LANGUAGE_TYPESCRIPT` (`.ts`) vs `LANGUAGE_TSX` (`.tsx`) by extension.
- **The silent failure is real but narrower than feared.** tree-sitter 0.24.x rejects unknown
  node-type/field/structure at `Query::new()` (→ compile error). What stays silent is a *structural*
  mismatch (valid nodes, wrong tree shape): it compiles and returns 0 captures. Confirmed with a
  reproduction that passes `cargo build` while yielding `defs=0`.
- **Strengthen the gate:** count-only (`defs>0 && refs>0`) misses the aider dual-vintage *wrong-anchor*
  drift (both `.scm` versions pass a count check but one anchors `@definition.method` to the container).
  Also assert the def capture's **node kind == `identifier`**.
