# ADR-0003: rtk and sqz install via `ubi:` (GitHub releases), not crates.io

- **Status:** Accepted (2026-06-15) · **Grounded in source:** 2026-06-16

## Context

Both rtk and sqz are Rust projects, so `cargo:`/crates.io looks like the obvious mise
backend. Source inspection shows it is wrong for both:

- **rtk:** crates.io has a **name collision** — a different project ("Rust Type Kit")
  owns `rtk` there. rtk's own README warns: *"If `rtk gain` fails, you have the wrong
  package."* The real distribution (v0.40.0) is **GitHub releases**; the crates.io entry
  is unrelated/stale. Release asset: `rtk-{TARGET}.tar.gz` — clean for `ubi:`.
- **sqz:** ships **prebuilt binaries** on GitHub releases (and npm `sqz-cli`, brew). The
  crate path compiles ELv2 source, which we explicitly avoid ([ADR-0002](0002-install-upstream-never-bundle.md)).
  Release asset: `sqz-{VERSION}-{PLATFORM}.tar.gz`.

## Decision

Install both via the **`ubi:` backend**: `ubi:rtk-ai/rtk` and `ubi:ojuschugh1/sqz`.

## Consequences

- **+** Bypasses the rtk crates.io collision entirely; gets a prebuilt sqz (no ELv2 source).
- **− / watchpoint:** sqz's `{VERSION}`-in-filename asset pattern may need an explicit
  `exe = "sqz"` hint for ubi's asset detection; fallback is `npm:sqz-cli` (same binary via
  postinstall). Verify on a fresh machine during e01 `mise run doctor`.
- mise registry short-names must be confirmed via `mise registry` before being added —
  never guessed.
