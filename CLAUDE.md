## ⛔ SHELL POLICY (bts)

**MANDATORY** — bts is installed. Use bts verbs and rtk wrappers for ALL shell commands.

Filtering is **active**: `vitest`, `jest`, `pytest`, `cargo test/build/check/clippy/run/bench/doc`,
`npx`, `tsc`, `astro` output is compressed automatically via rtk wrapper scripts in
`~/.local/bin/`. You do not need to prefix these — they are already wired.

**Escape hatch** — bypass filtering with `raw <cmd>` (routes through `rtk proxy`):
- `raw cargo metadata`   — structured JSON, must not be filtered
- `raw vitest --json`    — JSON reporter output
- `raw tsc --version`    — version check only

**Decision table:**

| Task | Command |
|------|---------|
| Code search (batch / agent-safe) | `bts grep <pattern> [path]` |
| Code search (interactive) | `bts find <pattern>` |
| Repo overview | `bts map` |
| Library docs | `bts docs <lib>` |
| Package source | `bts src <pkg>` |
| Compress output > 200 lines | `bts compress <file>` or `cmd \| bts compress` |
| Noisy command (no bts verb) | `rtk <cmd>` |
| Bypass filter | `raw <cmd>` |
| Toolchain health | `bts doctor` |

If a tool is MISSING, run `bts doctor` — it prints the exact fix command per tool.

---
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

<!-- rtk-instructions v2 -->
# RTK (Rust Token Killer) - Token-Optimized Commands

## Golden Rule

**Always prefix commands with `rtk`**. If RTK has a dedicated filter, it uses it. If not, it passes through unchanged. This means RTK is always safe to use.

**Important**: Even in command chains with `&&`, use `rtk`:
```bash
# ❌ Wrong
git add . && git commit -m "msg" && git push

# ✅ Correct
rtk git add . && rtk git commit -m "msg" && rtk git push
```

## RTK Commands by Workflow

### Build & Compile (80-90% savings)
```bash
rtk cargo build         # Cargo build output
rtk cargo check         # Cargo check output
rtk cargo clippy        # Clippy warnings grouped by file (80%)
rtk tsc                 # TypeScript errors grouped by file/code (83%)
rtk lint                # ESLint/Biome violations grouped (84%)
rtk prettier --check    # Files needing format only (70%)
rtk next build          # Next.js build with route metrics (87%)
```

### Test (60-99% savings)
```bash
rtk cargo test          # Cargo test failures only (90%)
rtk go test             # Go test failures only (90%)
rtk jest                # Jest failures only (99.5%)
rtk vitest              # Vitest failures only (99.5%)
rtk playwright test     # Playwright failures only (94%)
rtk pytest              # Python test failures only (90%)
rtk rake test           # Ruby test failures only (90%)
rtk rspec               # RSpec test failures only (60%)
rtk test <cmd>          # Generic test wrapper - failures only
```

### Git (59-80% savings)
```bash
rtk git status          # Compact status
rtk git log             # Compact log (works with all git flags)
rtk git diff            # Compact diff (80%)
rtk git show            # Compact show (80%)
rtk git add             # Ultra-compact confirmations (59%)
rtk git commit          # Ultra-compact confirmations (59%)
rtk git push            # Ultra-compact confirmations
rtk git pull            # Ultra-compact confirmations
rtk git branch          # Compact branch list
rtk git fetch           # Compact fetch
rtk git stash           # Compact stash
rtk git worktree        # Compact worktree
```

Note: Git passthrough works for ALL subcommands, even those not explicitly listed.

### GitHub (26-87% savings)
```bash
rtk gh pr view <num>    # Compact PR view (87%)
rtk gh pr checks        # Compact PR checks (79%)
rtk gh run list         # Compact workflow runs (82%)
rtk gh issue list       # Compact issue list (80%)
rtk gh api              # Compact API responses (26%)
```

### JavaScript/TypeScript Tooling (70-90% savings)
```bash
rtk pnpm list           # Compact dependency tree (70%)
rtk pnpm outdated       # Compact outdated packages (80%)
rtk pnpm install        # Compact install output (90%)
rtk npm run <script>    # Compact npm script output
rtk npx <cmd>           # Compact npx command output
rtk prisma              # Prisma without ASCII art (88%)
```

### Files & Search (60-75% savings)
```bash
rtk ls <path>           # Tree format, compact (65%)
rtk read <file>         # Code reading with filtering (60%)
rtk grep <pattern>      # Search grouped by file (75%). Format flags (-c, -l, -L, -o, -Z) run raw.
rtk find <pattern>      # Find grouped by directory (70%)
```

### Analysis & Debug (70-90% savings)
```bash
rtk err <cmd>           # Filter errors only from any command
rtk log <file>          # Deduplicated logs with counts
rtk json <file>         # JSON structure without values
rtk deps                # Dependency overview
rtk env                 # Environment variables compact
rtk summary <cmd>       # Smart summary of command output
rtk diff                # Ultra-compact diffs
```

### Infrastructure (85% savings)
```bash
rtk docker ps           # Compact container list
rtk docker images       # Compact image list
rtk docker logs <c>     # Deduplicated logs
rtk kubectl get         # Compact resource list
rtk kubectl logs        # Deduplicated pod logs
```

### Network (65-70% savings)
```bash
rtk curl <url>          # Compact HTTP responses (70%)
rtk wget <url>          # Compact download output (65%)
```

### Meta Commands
```bash
rtk gain                # View token savings statistics
rtk gain --history      # View command history with savings
rtk discover            # Analyze Claude Code sessions for missed RTK usage
rtk proxy <cmd>         # Run command without filtering (for debugging)
rtk init                # Add RTK instructions to CLAUDE.md
rtk init --global       # Add RTK to ~/.claude/CLAUDE.md
```

## Token Savings Overview

| Category | Commands | Typical Savings |
|----------|----------|-----------------|
| Tests | vitest, playwright, cargo test | 90-99% |
| Build | next, tsc, lint, prettier | 70-87% |
| Git | status, log, diff, add, commit | 59-80% |
| GitHub | gh pr, gh run, gh issue | 26-87% |
| Package Managers | pnpm, npm, npx | 70-90% |
| Files | ls, read, grep, find | 60-75% |
| Infrastructure | docker, kubectl | 85% |
| Network | curl, wget | 65-70% |

Overall average: **60-90% token reduction** on common development operations.
<!-- /rtk-instructions -->
