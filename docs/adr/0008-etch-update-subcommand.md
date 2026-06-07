# ADR-0008: etch update subcommand

**Date:** 2026-05-24
**Status:** Accepted

## Context

dotfiles previously managed system updates via a bash script (`update.sh`) that called `brew upgrade`, `apt-get dist-upgrade`, `gem update`, and `pip install --upgrade` in sequence. After etch-cli became the primary config tool, system updates needed to be integrated — running the bash script separately from etch was inconsistent with the declarative workflow.

The naive approach — calling `gem update` or `pip install --upgrade` without scoping — upgrades system-wide packages including those managed outside etch. This is incorrect: it can overwrite versions pinned by other manifests or system packages.

Alternatives considered: keep the bash update script alongside etch (works but defeats consolidation), add an `update:` key to each individual action (too granular — users want a single `etch update` to refresh all package managers), use `brew bundle` for everything (only covers Homebrew, not gem/pip/apt).

## Decision

Add `etch update` subcommand that wraps system package manager updates in a platform-aware, scoped way:

- `brew upgrade` (macOS only, skipped if brew not installed)
- `apt-get update && apt-get dist-upgrade -y` (Linux only, skipped if apt-get not installed)
- `gem update` scoped to user gems only (`--user-install` path)
- `pip list --user --outdated` followed by upgrade for user-scoped packages only

Each step is optional and skipped if the tool is not installed (`command -v` guard). Configuration lives in `etch.yaml` under an `update:` key, allowing steps to be disabled per-machine.

`pip list --user` and gem user scoping required extracting testable helper functions (PR #60) — the scoping logic is unit-tested via PATH mocks.

## Consequences

- Updates are platform-aware and scoped — no unintended upgrades of system packages or packages managed by other tools.
- Step functions that invoke real binaries (`brew`, `apt-get`, `gem`, `pip`) are tarpaulin-excluded, which is the primary reason the macOS coverage ceiling is ~82% rather than reaching 90%.
- Helper functions extracted for testability are unit-tested; the top-level `update` subcommand orchestration is not covered by tarpaulin (excluded as an integration step).
- Users run `etch update` in place of the previous bash script — no change to workflow, same sequence of operations.

## Interface Evolution

**2026-06-07 (PR #95):** The 10 individual per-category bool flags (`--brew`, `--rust`, etc.) were replaced with two filter flags: `--only <categories>` and `--skip <categories>`, both accepting comma-separated category names. The filter pattern scales better as categories are added and expresses intent more directly ("run only these" vs. listing many flags). `--only` and `--skip` are mutually exclusive; an unknown category name is a hard error.

## Related

- PR #59 (etch update subcommand)
- PR #60 (helper function extraction for testability)
- PR #95 (replace per-category flags with --only/--skip)
- ADR-0004: CI coverage floor at 70% (explains why step functions are excluded)
- `etch.yaml` — `update:` configuration key
