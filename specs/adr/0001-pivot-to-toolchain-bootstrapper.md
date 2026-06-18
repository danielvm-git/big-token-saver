# ADR-0001: Pivot from token-saving product to toolchain bootstrapper + unifier

- **Status:** Accepted (2026-06-15)
- **Deciders:** Daniel (author)
- **Supersedes:** the original 4-layer token-saving middleware thesis

## Context

The project began as a 4-layer "token-saving" middleware: an orchestrator linking
`sqz-engine` (ELv2), shelling `rtk`/`opensrc`, and porting aider's repo map. A 4-angle
red-team review (legal, technical, product, security) dismantled the thesis:

- **No moat** — 3 of the 4 layers are commodity or already native to the agents.
- **Token value erodes** and proxy/tee approaches **bust prompt caching** (can *raise* cost).
- **ELv2 distribution mislabeling** risk if `sqz-engine` were linked/redistributed.
- **Secret-leaking tee store** in the interception design.

The author's real pain was never a product — it was **installing a scattered toolchain
across five package managers** (brew/cargo/npm/pipx/GitHub releases) on every new machine.

## Decision

Reframe the project as a **personal toolchain bootstrapper + thin unifier** for an
AI-coding workflow, with **`bts map` as the single owned, net-new capability**. No proxy,
no command-output interception, no tee store, no market-moat requirement.

## Consequences

- **+** Scope collapses to something shippable in days, with immediate personal value.
- **+** Dissolves every red-team finding at once (legal, caching, security).
- **−** Abandons the "product" framing and any token-saving revenue story.
- Guardrail: new ideas land as **small `bts` verbs**, never new subsystems. Resisting
  scope-creep back toward the dropped product is an ongoing discipline (CLAUDE.md rule).
