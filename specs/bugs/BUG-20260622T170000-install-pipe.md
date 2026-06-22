# BUG-20260622T170000: install.sh fails in curl-pipe mode (two bugs)

## Problem

When running the install script via `curl | bash`, two bugs block a successful install at the very end, leaving users uncertain whether the toolchain was installed.

### Actual behavior

1. `cp: /Users/<user>/mise.toml: No such file or directory` — the script tries to copy mise.toml from a path where it doesn't exist, and `set -e` aborts the script.
2. If it somehow gets past step 1, `mise run doctor` fails with `mise ERROR unknown command: doctor`.

### Expected behavior

- The install script should work identically whether run locally (`bash install.sh`) or via pipe (`curl | bash`).
- Post-install verification should use a command that works regardless of mise config state.

### Reproduction

```bash
# In a clean directory (no mise.toml):
cd /tmp
bash <(curl -fsSL https://raw.githubusercontent.com/danielvm-git/big-token-saver/main/install.sh)
```

## Root Cause Analysis

### Bug 1: `cp` fails — mise.toml not locatable in pipe mode

The script resolves its own directory via:

```bash
HERE="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
```

In `curl | bash` mode, `BASH_SOURCE[0]` is unset and `$0` is `"bash"`. `dirname "bash"` is `"."`, so `HERE` resolves to the user's CWD — not the repo directory. mise.toml doesn't exist in CWD, so `cp "$HERE/mise.toml" "$GLOBAL"` fails. Because `set -e` is active, the script aborts immediately, before `mise install` even runs.

When run locally (`bash install.sh`), `$0` is the actual path to the script, so `HERE` resolves correctly to the directory containing both install.sh and mise.toml. This path works.

**Contributing factor:** The script distributes as a single file via curl pipe, but depends on a second file (mise.toml) that isn't downloaded. The two must travel together, but only install.sh is fetched.

### Bug 2: Wrong verification command

The script runs `mise run doctor`, which looks for a `[tasks.doctor]` definition in a loaded mise config file. On a fresh install where:
- The global config is empty (cp failed, or didn't exist yet)
- No local mise.toml is present

…there is no `[tasks.doctor]` task, so mise responds with "unknown command: doctor."

The intended command is `mise doctor` — the built-in mise diagnostic that checks mise installation health and lists installed tools. It requires no config and always works.

**Risk level:** Low — both fixes are contained to a single shell script with no downstream dependencies.

## TDD Fix Plan

### Cycle 1: mise.toml is downloadable in pipe mode

1. **RED**: Write a test that runs install.sh from a directory without mise.toml, simulating the pipe-mode path. Assert that mise.toml is fetched and the global config is populated.

   **GREEN**: Detect when mise.toml is not available locally (at `$HERE/mise.toml`) and download it from the GitHub repo raw URL. Keep the local-path fast path for the `bash install.sh` case.

   **verify**: `bash -c 'cd /tmp && curl -sL <url>/install.sh | bash'` produces a populated `~/.config/mise/config.toml`

### Cycle 2: Verification uses built-in mise doctor

2. **RED**: Write a test that asserts the post-install verification command works even when no mise.toml config is available (i.e., no `[tasks.doctor]` defined).

   **GREEN**: Replace `mise run doctor || true` with `mise doctor || true`.

   **verify**: `mise doctor` exits 0 (or 1 with helpful output) even without a mise.toml present.

### Cycle 3: End-to-end pipe install succeeds

3. **RED**: Write a smoke test that runs the full `curl | bash` install (possibly via a local file redirection to avoid network) and asserts:
   - Script exits 0
   - Global mise config file exists and is non-empty
   - `mise doctor` runs without "unknown command" error

   **GREEN**: No additional code changes needed — cycles 1-2 should make this pass. This test is the integration guard.

   **verify**: `bash <(cat install.sh)` from a clean CWD exits 0 with a populated config.

## Acceptance Criteria

- [ ] `curl | bash` install path copies mise.toml successfully (no "No such file or directory")
- [ ] Post-install verification uses `mise doctor` (not `mise run doctor`)
- [ ] Local `bash install.sh` path still works (no regression)
- [ ] Script is shellcheck-clean after changes
- [ ] `set -e` doesn't abort prematurely due to missing files

## Resolution

### Fix applied

1. **Bug 1 (mise.toml not locatable):** Added a fallback — if `mise.toml` isn't co-located with `install.sh` (pipe mode), download it from the repo via `curl`. Uses a temp file; local mode still uses the fast in-repo path.
2. **Bug 2 (wrong command):** Changed `mise run doctor` → `mise doctor` (built-in, config-independent).

### Verification

- [x] Shellcheck-clean
- [x] Pipe-mode simulation from /tmp: mise.toml fetched from GitHub (119 lines)
- [x] Local mode: mise.toml found in repo dir, no network fetch
- [x] `mise doctor` runs without errors
- [x] `set -e` no longer aborts due to missing mise.toml
