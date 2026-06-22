#!/usr/bin/env bash
# bts — single-command toolchain installer.
# Installs the `mise` engine if missing, then the entire set from mise.toml.
#
#   curl -fsSL https://raw.githubusercontent.com/danielvm-git/big-token-saver/main/install.sh | bash
#   # or locally:
#   bash install.sh
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"

# 1. ensure mise (the cross-ecosystem install engine)
if ! command -v mise >/dev/null 2>&1; then
	echo "→ installing mise…"
	if command -v brew >/dev/null 2>&1; then
		brew install mise
	else curl -fsSL https://mise.run | sh; fi
fi
eval "$(mise activate bash)" 2>/dev/null || true

# 2. locate manifest (co-located in local mode; fetch from repo in pipe mode)
REPO_RAW="https://raw.githubusercontent.com/danielvm-git/big-token-saver/main"
MISE_TOML="$HERE/mise.toml"
if [ ! -f "$MISE_TOML" ]; then
	echo "→ fetching mise.toml from repository…"
	MISE_TOML="$(mktemp)"
	curl -fsSL "$REPO_RAW/mise.toml" -o "$MISE_TOML"
fi

# 3. apply the manifest globally (back up any existing global config first)
GLOBAL="$HOME/.config/mise/config.toml"
mkdir -p "$(dirname "$GLOBAL")"
if [ -f "$GLOBAL" ]; then
	cp "$GLOBAL" "$GLOBAL.bak.$(date +%s)"
	echo "→ backed up existing global mise config"
fi
cp "$MISE_TOML" "$GLOBAL"

# 3. install everything, then report
echo "→ installing toolchain (this fetches from npm / pipx / GitHub releases)…"
mise install
echo
mise doctor || true
echo
echo "✓ done. Per-project wiring (hooks, .bts.toml, conventions) comes from:  bts init"
