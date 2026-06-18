# Tech Stack — big-token-saver (`bts`)

> **Phase artifact:** DISCOVER / map-codebase deliverable. Produced 2026-06-16.
> **Provenance:** grounded in real upstream source read locally via `opensrc`
> (aider `paul-gauthier/aider@main`, sqz `ojuschugh1/sqz@main`, rtk `rtk-ai/rtk@develop`),
> plus this repo's `mise.toml` + `install.sh`. Not written from memory.
> **Anchors:** [SCOPE_LATEST.yaml](../requirements/SCOPE_LATEST.yaml) ·
> [release-plan.yaml](../release-plan.yaml) · [ADRs](../adr/) · [state.yaml](../state.yaml)

## 1. System shape

`bts` is **not a product** — it is a personal toolchain bootstrapper + thin unifier
(see [ADR-0001](../adr/0001-pivot-to-toolchain-bootstrapper.md)). Three parts, one
compiled component:

| Part | What | Language | Ships? |
|---|---|---|---|
| **Installer** | `mise.toml` manifest + `install.sh` | TOML + bash | yes (manifest declares deps, does not contain them) |
| **Unifier** | `bin/bts` dispatcher + verb functions | bash | yes (Apache-2.0) |
| **`bts map`** | repo-map engine | Rust (`crates/bts-map`) | yes (Apache-2.0; the only net-new capability) |

Everything else is **installed upstream, never bundled or linked**
([ADR-0002](../adr/0002-install-upstream-never-bundle.md)).

## 2. The toolchain set (installed, not owned)

From `mise.toml` — backend prefix is load-bearing (it determines the install source):

| Tool | Backend | Role in `bts` |
|---|---|---|
| ripgrep, fd, bat, eza, fzf, jq | `registry:` | `bts find` (rg → fzf picker → bat preview) |
| gh, direnv | `registry:` | GitHub CLI; per-project env for `bts init` |
| shellcheck, shfmt, bats | `registry:` | dispatcher lint / format / test (dev only) |
| opensrc | `npm:` | `bts src <pkg>` — cached source fetch |
| bigpowers | `npm:` | skills lifecycle (this planning system) |
| @upstash/context7-mcp | `npm:` | `bts docs` backing — **an MCP server, see §6** |
| aider-chat | `pipx:` | `bts ai` — interactive coding agent |
| httpie | `pipx:` | API debugging |
| **rtk-ai/rtk** | `ubi:` | token-filter command proxy (installed as agent hook) |
| **ojuschugh1/sqz** | `ubi:` | compression CLI (installed as agent hook) |

`ubi:` = GitHub release binaries. rtk and sqz **must** come this way, never
crates.io ([ADR-0003](../adr/0003-rtk-sqz-via-ubi.md)).

## 3. `bts map` — architecture of the one owned crate

A Rust reimplementation of **aider's repo map** (`aider/repomap.py`, 867 lines).
Pipeline:

```
files → tree-sitter parse → .scm tag query → (def, ref) tags
      → graph (defs ↔ refs) → PageRank → rank-weighted tag selection
      → token-budgeted render → ranked code skeleton
```

### 3.1 Algorithm (source-confirmed)

- **Tag extraction** — per file, `filename_to_lang()` picks a grammar, the matching
  `{lang}-tags.scm` query captures `name.definition.*` (→ `def`) and
  `name.reference.*` (→ `ref`). Aider backfills refs via pygments when a grammar
  emits defs but no refs (notably C++) so the file isn't a graph dead-end.
- **Graph** — `networkx.MultiDiGraph`; an edge `referencer → definer` per shared
  identifier, `weight = use_mul * sqrt(num_refs)`. Weight multipliers worth porting
  faithfully: mentioned ident ×10; long snake/camel/kebab ident (≥8 chars) ×10;
  `_private` ×0.1; ident defined in >5 files (commodity name) ×0.1; referencer is a
  chat/open file ×50.
- **Ranking** — `nx.pagerank(weight="weight")` (power iteration, α=0.85) with a
  **personalization vector** seeded from chat files + mentioned files/idents; dangling
  mass redistributes to the personalization vector. Each node's rank is then split
  across its out-edges to score `(file, ident)` pairs.
- **Budgeting** — binary search on tag count (`middle = max_tokens // 25` to start),
  rendering with `grep_ast.TreeContext` and counting tokens until the rendered map
  fits the budget within ~15%.

### 3.2 Rust port mapping

| aider (Python) | bts-map (Rust) | Risk |
|---|---|---|
| `tree_sitter` + `grep_ast.tsl` grammars | `tree-sitter` crate + per-language grammar crates | **highest — §4** |
| `networkx.pagerank` | `petgraph` graph + **hand-rolled power-iteration PageRank** (petgraph has none) | medium — float determinism for snapshot tests |
| `grep_ast.TreeContext` renderer | **must be reimplemented** (no Rust equivalent) | high — AST-aware line selection is non-trivial |
| litellm/tiktoken token count | `tiktoken-rs` | low |
| `diskcache` (SQLite) tag cache | `rusqlite` or `sled` | low |
| pygments ref backfill | regex tokenizer, or defer | low (optimization) |
| file walk | `ignore` crate | low |

### 3.3 Language coverage

Acceptance is ≥10 languages with **non-empty** maps. Aider keeps queries in two dirs;
coverage is not uniform across them:

- `tree-sitter-language-pack/` (31 queries): rust, python, javascript, go, java, ruby,
  c, cpp, **swift**, c#, … — **no typescript**.
- `tree-sitter-languages/` (27 queries): adds **typescript**, kotlin, scala, php, zig,
  haskell, … — **no swift**.

→ The 10-language target (Rust, Python, JS, **TS**, Go, Java, Ruby, C, C++, **Swift**)
spans **both** dirs. The vendor layout must carry both, with a per-language source-of-truth
note. Query naming: `{lang}-tags.scm`.

## 4. The hardest risk: tree-sitter ABI / `.scm` coupling

This is the deepest finding and the reason `bts map` is rated L.

- A `.scm` query is **bound to the grammar version it was written against**. If a
  grammar crate is bumped and a node type is renamed/restructured, the query **still
  compiles** (capture names are valid) but **matches nothing** → **silent empty map,
  no error**.
- Confirmed real, not theoretical: aider's own `rust-tags.scm` differs between its two
  query dirs (a `@definition.method` parenthesization), i.e. the same language drifted
  across grammar versions.
- aider sidesteps this in Python by pinning one wheel — `tree-sitter-language-pack==0.13.0`
  on `tree-sitter==0.25.2` — that atomically ships all grammars + queries together. Rust
  has no such atomic bundle; each grammar is a separate crate.

**Mitigation (must be enforced in e05):**
1. Source of truth for which grammar commit each `.scm` was written against:
   `tree-sitter-language-pack` tag `v0.13.0` → `sources/language_definitions.json`.
   Pin each Rust grammar crate to the version at/nearest that commit.
2. Pin the `tree-sitter` core crate to the ABI those grammar crates compiled against.
3. **Never bump a grammar crate without re-vendoring its `.scm` from the same vintage.**
4. **Verify with a real query per language, not just `cargo build`.** Each fixture must
   contain ≥1 def and ≥1 ref; the test asserts the parser returns non-empty `def` *and*
   `ref` tags. A compile-only test passes on a silently-broken grammar.

See [ADR-0005](../adr/0005-bts-map-port-and-grammar-pinning.md).

## 5. Licensing posture

- **Shipped code** = Apache-2.0 (the `bts` shell + `bts-map` crate). Nothing else ships.
- **sqz is Elastic-License-2.0** (confirmed: `LICENSE`, `Cargo.toml` `license-file`,
  npm `"license":"ELv2"`). ELv2 is a *use* restriction (no offering it as a hosted/managed
  service), **not copyleft**. Installing sqz as a **prebuilt binary via ubi** incorporates
  no source → **zero ELv2 obligation** on `bts` artifacts. The one breach vector is wiring
  `bts` into a multi-tenant hosted service that exposes sqz's features — the CLAUDE.md
  "no proxy / no tee store" rule closes it. (sqz ships a coded-but-unwired `api_proxy.rs`;
  do not expose it over a network.) See [ADR-0002](../adr/0002-install-upstream-never-bundle.md).
- **rtk is Apache-2.0** (LICENSE file; its `Cargo.toml` `license = "MIT"` is a cosmetic
  upstream mismatch — neither imposes copyleft).
- **Vendored `.scm` queries are a MIT/Apache MIX** (e.g. Elixir is Apache-2.0, most are
  MIT), inherited via aider from each grammar repo. `crates/bts-map/NOTICE` must reproduce
  per-grammar attribution — **do not relabel them monolithically.**

## 6. Gray areas / planning signals (feed back into the plan)

1. **`bts docs` ≠ a run-and-pipe verb.** context7 is an **MCP server**, not a CLI. A thin
   shell wrapper can't transparently pipe through it. e04 should redefine `bts docs` as
   "show/launch the context7 MCP config for the active agent," not "wrap a binary."
2. **`bts init` should call `sqz init` / `rtk init`**, which inject their own PreToolUse
   hooks (`sqz init --only claude`, `rtk init --agent claude`). e03 must delegate hook
   wiring to them, not hand-roll JSON injection.
3. **ubi asset-name hint for sqz.** sqz release assets are `sqz-{VERSION}-{PLATFORM}.tar.gz`;
   ubi may need an explicit `exe = "sqz"` hint, or fall back to `npm:sqz-cli`. rtk's
   `rtk-{TARGET}.tar.gz` is clean. Verify during e01 `mise run doctor` on a fresh machine.
4. **PageRank determinism.** Snapshot tests require a fixed float ordering; pin iteration
   count + tie-break by `(file, ident)` so output is byte-stable for a fixed git tree.
5. **`grep_ast.TreeContext` has no Rust port** — budget real effort for the skeleton
   renderer; it's the second-biggest unknown after grammar pinning.

## 7. Verification strategy (per-component)

- **Installer:** fresh machine → `bash install.sh` → `mise run doctor` reports every tool
  present; re-run is idempotent; backs up existing global config.
- **Dispatcher:** `shellcheck` + `shfmt` clean; each verb degrades gracefully (prints a
  hint pointing at `bts doctor`, exits non-zero) when its tool is absent.
- **bts-map:** `cargo clippy -- -D warnings`; snapshot test vs a fixture repo, deterministic
  for a fixed git tree; **per-language non-empty def+ref assertion** for ≥10 languages.
