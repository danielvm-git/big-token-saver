---
bug_id: BUG-20260709
date: 2026-07-09
severity: medium
priority: p2
scope: ci
status: fixed
summary: "Release workflow fails with EINVALIDNEXTVERSION on stale release/v0.1.0 branch"
file: specs/bugs/BUG-20260709-release-branch-stale.md
---

## Reproduction

1. Trigger `workflow_dispatch` on `release/v0.1.0` branch
2. semantic-release computes next version as `0.2.0` (minor bump from `feat` commits)
3. Fails: `EINVALIDNEXTVERSION The release '0.2.0' on branch 'release/v0.1.0' cannot be published as it is out of range. Based on the releases published on other branches, only versions within the range >=1.0.0 can be published from branch release/v0.1.0.`

**Failing run**: [27734299007](https://github.com/danielvm-git/big-token-saver/actions/runs/27734299007)

## Root Cause (RCA)

**Direct cause**: `.releaserc` lists `{"name": "release/v0.1.0", "prerelease": false}` as a release branch. When `main` has tags `v0.2.0` through `v0.6.0`, semantic-release's built-in branch-version-range logic computes that `release/v0.1.0` can only publish versions `>=1.0.0` — but the computed next version is `0.2.0`.

**Underlying cause**: The `release/v0.1.0` branch is stale — 23 commits behind `main`. It was originally created for the v0.1.0 release and was intended to channel fixes to a maintenance branch, but all subsequent releases (v0.2.0–v0.6.0) have shipped from `main`. The branch entry in `.releaserc` was never cleaned up.

**Why the "fix" commit didn't work**: `e0afdc8` ("fix(ci): remove channel constraint from release/v0.1.0 branch") changed the entry from `{"name": "release/v0.1.0", "channel": "v0.1.x"}` to `{"name": "release/v0.1.0", "prerelease": false}`. The `channel` constraint was never the problem — the version range computation is based on tags on **other** branches, not the `channel` field.

## Verify Steps

- verify: `gh api '/repos/danielvm-git/big-token-saver/contents/.releaserc?ref=main' --jq '.content' | base64 -d | python3 -c "import sys,json; d=json.load(sys.stdin); assert all(isinstance(b,str) or b.get('name') != 'release/v0.1.0' for b in d['branches'])"` — .releaserc on main no longer references release/v0.1.0
- verify: `gh api /repos/danielvm-git/big-token-saver/branches/release%2Fv0.1.0` — returns 404 (branch deleted)
- verify: `gh run list --repo danielvm-git/big-token-saver --workflow release.yml --branch main --limit 1 --json conclusion --jq '.[0].conclusion'` — latest Release workflow on main is `success`

## Fix Approach

Two changes on `main`:

1. **Update `.releaserc`**: Remove the `release/v0.1.0` branch entry. Only `"main"` remains.
2. **Delete `release/v0.1.0` branch**: `gh api -X DELETE /repos/danielvm-git/big-token-saver/git/refs/heads/release%2Fv0.1.0`

Risk: low. `release/v0.1.0` is stale (2 ahead, 23 behind main). The 2 ahead commits were cherry-picked/ported to main.

## Resolution

**Changes**:
1. Updated `.releaserc` to remove `release/v0.1.0` branch entry (only `main` remains)
2. Changed default branch from `release/v0.1.0` to `main`
3. Deleted stale `release/v0.1.0` remote branch

**Commit**: `fix(ci): remove stale release/v0.1.0 branch from .releaserc`

**Verification** (all passed):
- ✅ `.releaserc` on main no longer references `release/v0.1.0` — branches: `["main"]`
- ✅ `release/v0.1.0` branch deleted (HTTP 404)
- ✅ Default branch is `main`

**Side effect**: The Release workflow no longer has the stale branch constraint. Next release on `main` will proceed normally.

**Orphaned commits (post-deletion verification)**:
- `afa086c` (`feat(cli): add bts wire #5`) — content independently merged to `main` as `13e4e81`
- `3d8d5a5` (`fix(cli): restore bts wire verb lost in merge`) — `bts_wire()` present on `main` (enhanced in e08)
- `65d4faa` — WIP stash; no content value

No data loss. Git GC will clean up in ~90 days.
