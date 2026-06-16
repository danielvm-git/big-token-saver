# Claude guidance for big-token-saver

## What this project is

`big-token-saver` (binary `bts`) is a personal **toolchain bootstrapper + thin unifier**
for an AI-assisted coding workflow. It is NOT a token-saving product, proxy, or middleware
— an earlier 4-layer design was dropped after a red-team review (see specs/state.yaml
`pivot` and the memory files).

Three parts:
1. **Installer** — `mise.toml` (manifest) + `install.sh`. One command installs the whole
   set across mise's registry / npm / pipx / ubi backends.
2. **Unifier** — a shell `bts` dispatcher: `setup`, `doctor`, `init`, and verb wrappers
   (`find`, `src`, `docs`, `ai`) with opinionated defaults.
3. **`bts map`** — the one compiled component: a small Rust crate (aider's repo-map).

## Hard rules

1. **Install upstream; never bundle or relink.** Tools are installed from their official
   sources via mise. Do NOT add `sqz-engine`, `rtk`, etc. as Cargo/code dependencies. This
   is what keeps shipped artifacts license-clean (zero ELv2 obligations).
2. **Don't reinvent the install engine.** mise owns version pinning, idempotency, upgrades,
   uninstall, per-project env. `bts setup` shells out to mise; it does not reimplement it.
3. **rtk and sqz install via `ubi:` (GitHub releases), NOT crates.io.** rtk's crates entry
   is a stale v0.1.0 stub; sqz ships binaries, not a CLI crate.
4. **Shell-first.** The unifier is shell. Rust is used ONLY for `bts map`. Don't build a
   multi-crate workspace to alias `rg`.
5. **`bts map` grammar pinning.** Vendored aider `.scm` queries are bound to specific
   tree-sitter grammar versions. Pin each grammar crate to match the `.scm` vintage and
   verify with a real query per language — mismatches yield silent empty maps, not errors.

## Layout

```
mise.toml            toolchain manifest (source of truth)
install.sh           bootstrap: install mise, apply manifest globally
bin/bts              shell dispatcher (setup / doctor / init / find / src / docs / ai)
crates/bts-map/      the ONLY Rust crate (petgraph PageRank + vendored aider .scm)
  vendor/queries/    aider *.scm tag queries (MIT/Apache mix — NOTICE has per-grammar credit)
specs/               bigpowers planning (release-plan.yaml, state.yaml, epics/)
```

## Test strategy

- **Installer**: `mise run doctor` must report every tool present after `install.sh`.
  Re-running install.sh must be idempotent and back up any existing global mise config.
- **bts dispatcher**: shellcheck-clean; each verb degrades gracefully if its tool is absent.
- **bts-map**: snapshot test against a fixture repo under `crates/bts-map/tests/fixtures/`;
  deterministic output for a fixed git tree; >=10 languages produce non-empty maps.

## Gotchas

- mise registry short-names (`ripgrep`, `fzf`, …) resolve via `mise registry` — verify a
  name there before adding it to the manifest.
- `bts init` hooks shell out to the installed `rtk`/`sqz` binaries — never link them.
- Don't let scope creep back toward the dropped product: no proxy, no tee store, no
  command-output interception as a core feature. New ideas land as small `bts` verbs.
