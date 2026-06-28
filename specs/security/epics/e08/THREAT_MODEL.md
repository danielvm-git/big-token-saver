# Threat Model — e08 Agent Behavioral Integration

## Surface Area

| Component | Change |
|-----------|--------|
| `bin/bts` (bts init) | Writes executable files to `~/.local/bin/` |
| `bin/bts` (bts wire) | Writes/prepends to `CLAUDE.md`, `AGENTS.md`, etc. |
| `bin/bts` (bts grep) | New verb: shells out to `rg` |
| `bin/bts` (bts doctor) | Prints per-tool fix commands |
| `.envrc` | PATH prepend (`~/.local/bin` first) |

## Vulnerability Categories

### HIGH — Wrapper script content injection
**Story:** e08s01  
**Risk:** `bts init` writes `~/.local/bin/vitest`, `cargo`, etc. containing `exec rtk <cmd> "$@"`. If the `rtk` binary path or any argument is interpolated from user-controlled config (`.bts.toml`), a malicious config could hijack any invocation of those commands for the user.  
**Mitigation:** Wrapper content is hardcoded literals — no interpolation from `.bts.toml` or env. Use `cat > file << 'EOF'` (quoted heredoc) so `$` characters are never expanded.

### MEDIUM — PATH prepend in `.envrc`
**Story:** e08s02  
**Risk:** Placing `~/.local/bin` first on PATH means any file written there shadows system binaries (`git`, `cargo`, `node`, etc.). If another tool or script writes a malicious binary to `~/.local/bin/`, it would be silently executed instead of the real one.  
**Mitigation:** `~/.local/bin` shadowing is a known, accepted Unix pattern for user-local overrides. The wrappers `bts init` writes are clearly named and short enough to audit at a glance. Document the behavior in `bts init` output. No fix needed — this is the intent.

### LOW — File injection via bts wire
**Story:** e08s03  
**Risk:** `bts wire` prepends a block to `CLAUDE.md` and similar files. If the sentinel string (`## ⛔ SHELL POLICY (bts)`) is crafted to match a string already in the file, the idempotency check could be fooled into skipping injection (under-injection) or misplacing it (if the file is adversarially pre-seeded).  
**Mitigation:** The threat is self-inflicted (the user controls their own `CLAUDE.md`). No untrusted input flows in. Acceptable.

### LOW — rg passthrough in bts grep
**Story:** e08s04  
**Risk:** `bts grep <pattern>` passes the pattern directly to `rg`. A pattern with shell metacharacters could behave unexpectedly if the dispatcher uses `eval` or unquoted `$@`.  
**Mitigation:** Use `"$@"` (double-quoted) throughout the dispatcher — never `eval`. rg itself handles arbitrary regex safely.

### INFO — bts doctor fix commands printed to stdout
**Story:** e08s05  
**Risk:** Doctor output includes `brew install rtk-ai/tap/rtk` style commands. If a user copies and runs these without understanding, they run third-party install scripts. This is not a new risk — the same commands appear in the README.  
**Mitigation:** None needed. Standard practice.

## Risk Level

**Overall: LOW** — All changes write to user-local paths the user already owns. No network calls, no privilege escalation, no secret handling. The one non-trivial risk (wrapper content injection) is fully mitigated by using hardcoded heredocs.

## WSJF security boost

No HIGH/CRITICAL unmitigated risks → no WSJF numerator boost needed.
