### Story 6.3: npm distribution — global install + npx zero-install

**type:** feat
**context:** infra
**bcps:** 3

**Context:** Ship `bts` as an npm package (`big-token-saver`) so users can
`npm install -g big-token-saver` or `npx big-token-saver` without PATH setup.
The shell dispatcher (`bin/bts`) is the package entry point; the Rust `bts-map`
binary is bundled per-platform (prebuilt in CI). No new dependencies — pure
package.json config + bash path resolution + CI wiring.

## Acceptance Criteria (Gherkin)

```gherkin
Scenario: Global install provides bts on PATH
  Given a machine with Node.js installed
  When I run "npm install -g big-token-saver"
  Then "bts doctor" exits 0 and lists tools

Scenario: bts map works from global install
  Given bts is installed globally
  When I run "bts map --budget 64 ."
  Then a ranked repo map is printed within the budget

Scenario: npx zero-install doctor
  Given an empty temporary directory
  When I run "npx big-token-saver doctor"
  Then it reports tool status or degrades gracefully with clear messaging

Scenario: npm pack produces correct tarball
  When I run "npm pack --dry-run"
  Then bin/bts is listed
  And the correct platform binary for bts-map is included
```

## Steps

1. Update `package.json` with `name`, `bin`, `files`, `engines`, remove
   `private: true`. Add `.npmignore` to exclude crates/, specs/, target/
   except the bundled binaries. Semantic-release controls the version field.
   → verify: `npm pack --dry-run 2>&1 | grep -q 'bin/bts'`

2. Add npm publish step to `.github/workflows/release.yml`: after
   semantic-release creates the GitHub release, download bts-map build
   artifacts, inject them into the npm package (under bin/), and run
   `npm publish`. Use `NPM_TOKEN` secret.
   → verify: `grep -q 'npm publish' .github/workflows/release.yml`

3. Extend `bin/bts` binary resolution for `bts map`: add a lookup for
   `$HERE/bts-map-$SUFFIX` (bundled alongside the shell script) before
   falling back to PATH/cargo paths. Detect platform via `uname -sm`.
   → verify: `PATH=/tmp/fake bin/bts map --budget 64 . 2>&1 | head -3 | grep -qE '(crates/bts-map|MISSING)'`

4. Smoke test: `npm pack` into a tarball, unpack in temp dir, verify
   `bin/bts` is executable and `bts map` locates the bundled binary.
   → verify: `npm pack && tar xzf big-token-saver-*.tgz -C /tmp/test-npm && /tmp/test-npm/package/bin/bts map --budget 64 . | head -3 | grep -q 'crates/bts-map'`

## Out of scope

- npm registry authentication setup (one-time manual `npm login` or `NPM_TOKEN` secret)
- Windows support (no win32 bts-map build target yet)
- `npm install` as a local dev dependency (global-only for now; local could follow)

## Risks

| Risk | Mitigation |
|------|------------|
| bts-map binary not found by bundled resolver (platform suffix mismatch) | Step 3 verifies; add explicit error listing expected suffixes |
| npm publish fails due to 2FA | Document `NPM_TOKEN` requirement in CI setup |
| Package.json `files` excludes something bin/bts needs | `npm pack --dry-run` catches this early (step 1 verify) |
