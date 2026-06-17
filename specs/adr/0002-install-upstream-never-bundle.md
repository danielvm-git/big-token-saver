# ADR-0002: Install upstream via mise; never bundle or relink (ELv2 boundary)

- **Status:** Accepted (2026-06-15) · **Grounded in source:** 2026-06-16
- **Relates to:** [ADR-0003](0003-rtk-sqz-via-ubi.md), [ADR-0004](0004-shell-first-rust-only-for-map.md)

## Context

Several installed tools carry restrictive or source-available licenses — most notably
**sqz is Elastic-License-2.0** (verified in source: `LICENSE`, `Cargo.toml`
`license-file`, npm manifest `"license":"ELv2"`). The original design linked
`sqz-engine` as a Cargo dependency, which would pull ELv2 source into `bts`'s build graph
and shipped artifacts.

ELv2 is **not copyleft**. Its core restriction is use-based: you may not provide the
software to third parties **as a hosted or managed service** exposing its substantial
features. It does **not** require downstream code to be open, and it does not attach to
software that merely *invokes* an installed binary.

## Decision

`bts` **installs every third-party tool upstream from its official source via mise, as a
binary** — it never bundles, vendors, links, or redistributes any of them. The only code
`bts` ships is its own Apache-2.0 shell dispatcher and the `bts-map` crate.

## Consequences

- **+** Shipped artifacts incorporate **zero ELv2 source** → **zero ELv2 obligation**.
  Installing sqz as a prebuilt binary is use, not redistribution.
- **+** mise owns version pinning, idempotency, upgrades, uninstall — not our problem.
- **− / watchpoint:** The one way to breach ELv2 is to wire `bts` into a **multi-tenant
  hosted service** that exposes sqz's features (e.g. exposing sqz's coded-but-unwired
  `api_proxy.rs` over a network). The CLAUDE.md "no proxy / no tee store / localhost-only"
  rule is the safeguard and must hold.
- Implication: if `bts map` ever wants compression it **shells out to the installed `sqz`
  binary** — it does not link it.
