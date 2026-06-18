# BUG-20260618: Missing `type:` and `context:` metadata in e07 spec YAMLs

## Problem

The three spec artifacts updated during e07 (profiles + compress + regret learning) lack
`type:` and `context:` metadata fields required by the `plan-work` convention:

- `specs/epics/e07-backlog.yaml` — no `type:` or `context:` field
- `specs/execution-status.yaml` — no `type:` or `context:` field
- `specs/state.yaml` — no `type:` or `context:` field

The `audit-code` checklist item "New plan artefacts include `type:` and `context:` metadata"
caught this. The `plan-work` skill defines the format as:
- `type: feat | fix | refactor`
- `context: domain | infra`

No other spec files in this project carry these fields either — this is a project-wide
convention gap that first surfaced here. The fix adds them to the three files changed in
e07 (the scope under audit), establishing the pattern for future spec artefacts.

## Root Cause Analysis

The `plan-work` skill's plan template includes `type:` and `context:` metadata fields, but
the project's CONVENTIONS.md doesn't explicitly require them for spec YAMLs. When e07 was
built, the developer followed the existing project pattern (which doesn't include these
fields) rather than the bigpowers `plan-work` output template (which does).

Contributing factors:
- Project convention drift: no prior spec file has these fields
- The `audit-code` checklist enforces them, but they're not part of the local CONVENTIONS.md

Risk: **Low** — cosmetic/metadata gap. No behavioral impact.

## TDD Fix Plan

1. **RED**: `grep -L 'type:' specs/epics/e07-backlog.yaml specs/execution-status.yaml specs/state.yaml`
   should return nothing (all three files currently fail this check).
   **GREEN**: Add `type: feat` and `context: infra` to the three spec YAMLs.
   **verify**: `grep -L 'type:' specs/epics/e07-backlog.yaml specs/execution-status.yaml specs/state.yaml | wc -l | grep '^0$'`

2. **RED**: `grep -L 'context:' specs/epics/e07-backlog.yaml specs/execution-status.yaml specs/state.yaml`
   should return nothing.
   **GREEN**: Verify both fields are present.
   **verify**: `grep -L 'context:' specs/epics/e07-backlog.yaml specs/execution-status.yaml specs/state.yaml | wc -l | grep '^0$'`

**REFACTOR**: None needed — purely additive change.

## Acceptance Criteria

- [ ] `specs/epics/e07-backlog.yaml` has `type: feat` and `context: domain`
- [ ] `specs/execution-status.yaml` has `type: feat` and `context: infra`
- [ ] `specs/state.yaml` has `type: feat` and `context: infra`
- [ ] YAML remains valid (no syntax errors)
- [ ] Audit-code `type:`/`context:` checklist item passes

## Resolution

Fixed 2026-06-18: Added `type: feat` and `context: {domain,infra}` to all three
spec YAMLs (`e07-backlog.yaml`, `execution-status.yaml`, `state.yaml`).

Verification:
- All three files have exactly 1 `type:` and 1 `context:` line
- YAML valid (Python yaml.safe_load passes)
- 16/16 tests pass, shellcheck clean, clippy clean
- No code changes needed — purely additive metadata
