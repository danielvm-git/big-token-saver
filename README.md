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

## Install

```sh
bash install.sh        # installs the mise engine if needed, then the whole set
mise run doctor        # verify everything resolved
```

## Use

```sh
bts setup              # (re)install / sync the toolchain
bts doctor             # what's present, what's missing
bts init               # wire the CURRENT project: .bts.toml, .envrc, agent hooks
bts find <pattern>     # rg → fzf picker → bat preview, one command
bts src <pkg>          # opensrc source fetch, your defaults
bts docs <lib>         # context7 live docs
bts ai                 # launch your agent with hooks + token budget pre-loaded
bts map [budget]       # ⭐ ranked, token-budgeted repo map (the one original piece)
```

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
