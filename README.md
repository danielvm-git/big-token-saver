# bts — big-token-saver

> One command installs my entire AI-assisted coding toolchain, unifies it behind a single
> front door, and adds the one capability none of the pieces have.

Not a product, not middleware — a personal **toolchain bootstrapper + unifier**. Tired of
`brew install` here, `cargo install` there, `npm i -g` somewhere else, and `pipx`/`uv` for
the Python bits? `bts` is one manifest and one command.

## What it installs

| Tool | Role | Backend |
|---|---|---|
| ripgrep, fd, bat, eza, fzf, jq, gh, direnv | shell power-tools | mise registry |
| opensrc | fetch + cache package source for agents | npm |
| context7 (mcp) | live, version-specific docs | npm |
| aider | repo-map coding agent | pipx |
| rtk | command-output token filter | ubi (GitHub releases) |
| sqz | compression engine | ubi (GitHub releases) |
| bigpowers | skills lifecycle | npm |

All declared in one file: [mise.toml](mise.toml).

---

## Getting started

Follow these three steps in order on any machine or new project.

### Step 1 — Install the toolchain

One command, fresh machine:

```sh
curl -fsSL https://raw.githubusercontent.com/danielvm-git/big-token-saver/main/install.sh | bash
```

Or from a local clone:

```sh
bash install.sh
```

What happens:
1. Installs `mise` (the cross-ecosystem version manager) if it is missing.
2. Copies `mise.toml` to `~/.config/mise/config.toml` (backs up any existing config first).
3. Runs `mise install` — pulls every tool from npm / pipx / GitHub releases.
4. Installs the `bts` dispatcher itself to `~/.local/bin/bts`.

> Make sure `~/.local/bin` is in your `PATH` before continuing.

### Step 2 — Verify the toolchain is healthy

```sh
bts doctor
```

Every tool prints `ok` or `MISSING`. Fix any `MISSING` entries before moving on —
later steps silently degrade when tools are absent.

You can also run `mise run doctor` directly; `bts doctor` is a thin wrapper around it.

### Step 3 — Wire a project (run once per repo)

```sh
cd /path/to/your/project
bts init
```

`bts init` is idempotent. It:

1. **Scaffolds `.bts.toml`** — project-level config (model, token budget). Inherits
   values from `~/.config/bts/config.toml` when present.
2. **Scaffolds `.envrc`** — exports `BTS_MODEL` and `BTS_TOKEN_BUDGET` so every tool
   in the session picks up the right values automatically via `direnv`.
3. **Wires sqz hooks** (`sqz init --only claude --yes`) — patches the agent's
   instruction file so context compression is applied transparently.
4. **Wires rtk hooks** (`rtk init --agent claude --auto-patch`) — patches the agent's
   instruction file so command outputs are token-filtered automatically.

If `sqz` or `rtk` are missing, `init` prints `MISSING` and continues — the rest of the
wiring still completes.

---

## Are agents ready?

After `bts init` the agents should be self-instructed to use the toolchain. You can confirm:

```sh
# 1. Check every required binary is present
bts doctor

# 2. Confirm the project config is correct
bts config show

# 3. Dry-run: see which agent instruction files would be patched
bts wire --dry-run

# 4. Actually patch any instruction file not yet wired
bts wire
```

`bts wire` patches `CLAUDE.md`, `GEMINI.md`, `AGENTS.md`, `.gemini/GEMINI.md`, and
`OPENCODE.md` (whichever exist) with a compact `## bts toolchain` section that tells the
agent which `bts` verbs to prefer. It is idempotent — already-wired files are skipped.

**Green state checklist:**

| Check | Command | Expected |
|---|---|---|
| All binaries present | `bts doctor` | all lines show `ok` |
| Project config loaded | `bts config show` | correct model + budget |
| Instruction files wired | `bts wire --dry-run` | `0 patched` (already done) |
| Token filter active | `rtk gain` | shows session savings (after first command) |

---

## Full command reference

```sh
bts setup              # (re)install / sync the toolchain
bts doctor             # what's present, what's missing
bts init               # wire the CURRENT project: .bts.toml, .envrc, agent hooks
bts config             # show effective merged config (env ← global ← project)
bts config get <key>   # get a single config value
bts config set <key> <val>  # set a project-level config value
bts wire               # patch agent instruction files with bts usage rules
bts wire --dry-run     # preview which files would be patched
bts find <pattern>     # rg → fzf picker → bat preview, one command
bts find --print <pattern>  # non-interactive: plain rg output
bts src <pkg>          # opensrc source fetch, your defaults
bts docs <lib>         # context7 live docs
bts ai                 # launch your agent with hooks + token budget pre-loaded
bts compress <file>    # compress a file through sqz
cmd | bts compress     # compress stdin through sqz
bts map [budget]       # ⭐ ranked, token-budgeted repo map (the one original piece)
```

---

## Downloads

Pre-built `bts-map` binaries for each release are attached to [GitHub Releases](https://github.com/danielvm-git/big-token-saver/releases):

| Platform | Binary |
|---|---|
| macOS (Intel) | `bts-map-darwin-x86_64.tar.gz` |
| macOS (Apple Silicon) | `bts-map-darwin-arm64.tar.gz` |
| Linux (x86_64) | `bts-map-linux-x86_64.tar.gz` |

## How it's built

- **The installer** is a `mise.toml` manifest + a thin `install.sh`. mise does the
  cross-ecosystem heavy lifting (versioning, idempotency, upgrades). We don't reinvent it.
- **The unifier** is a ~100-line shell dispatcher (`bts`) that routes to the right tool
  with opinionated defaults.
- **`bts map`** is the only compiled component — a small Rust binary (aider's PageRank
  repo-map technique), shipped as a GitHub-release binary and installed via mise like
  everything else.

## License

Apache-2.0. The only code shipped here is the shell unifier and the `bts-map` crate.
Everything else is **installed from its own upstream source**, never bundled or relinked —
so no third-party license obligations ride along in a `bts` binary. (`bts map` vendors
aider's tree-sitter query files, which are an MIT/Apache mix — see `crates/bts-map/NOTICE`
for per-grammar attribution.)
