# ADR-0006: Native action expansion strategy

**Date:** 2026-05-19
**Status:** Accepted

## Context

Before native actions, etch-config manifests used `command.run` for everything that didn't have a first-class action. The problem: no type-safety, no idempotency guarantees per-action, no serde validation of fields. Each workaround was bespoke Bash embedded in YAML — fragile, untestable, and inconsistent across machines.

PRs #61–#72 identified a set of recurring patterns in the dotfiles migration: installing Ruby gems, pip packages, pyenv versions and virtualenvs, npm global packages, removing unused packages, and adding users to groups. All of these were written as `command.run` workarounds.

Alternatives considered: keep using `command.run` for one-off installs (low effort, high fragility), use Ansible for package management (too heavy — ADR-0001 rejected Ansible), expand comtrya's existing action set (archived upstream, no path forward).

## Decision

Expand the native action surface by adding one action per PR. Actions added: `ruby.install` (via ruby-install), `gem.install`, `pip.install`, `pyenv.install`, `pyenv.virtualenv`, `npm.install`, `package.autoremove`, `user.group`.

Each action must:

- Check idempotency before running (e.g. `gem list --installed`, `id -nG` for groups, `pyenv versions` for pyenv installs)
- Validate fields via serde at manifest parse time — unknown or malformed fields are rejected before any action runs
- Support `skip_if_exists` where applicable
- Use serde field rename as the canonical source of truth for YAML field names

## Consequences

- YAML manifests become declarative (desired state, not procedural). Each action has a clear idempotency guarantee documented in its `execute()` implementation.
- Each new action requires: a Rust struct, serde derives, `execute()`, and tests with PATH mocks. The implementation pattern is consistent across all actions in `etch-lib/src/actions/`.
- Coverage floor at 70% (ADR-0004) limits how much action logic gets tested — step functions that invoke real binaries (`ruby-install`, `gem`, `pip`, `pyenv`, `npm`) are excluded from tarpaulin. Tests cover the idempotency guard and struct validation, not the external binary invocation.
- `command.run` remains available for truly one-off operations, but the preference is a native action whenever a pattern recurs across more than one manifest.

## Related

- ADR-0004: CI coverage floor at 70%
- PRs #61–#72 (native action expansion series)
- `etch-lib/src/actions/` — action implementations
