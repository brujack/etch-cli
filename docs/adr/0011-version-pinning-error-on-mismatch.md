# ADR-0011: package.install version pinning uses error-on-mismatch semantics

**Date:** 2026-06-02
**Status:** Accepted

## Context

The `package.install` action needed an optional `version:` field so operators can pin a package to an exact version for reproducibility or compatibility. When the declared version doesn't match what is currently installed, three behaviors were possible:

1. **Auto-reconcile** — silently upgrade or downgrade to the declared version
2. **Warn and continue** — log a warning and proceed with the existing installation
3. **Error on mismatch** — return an actionable error and require manual resolution

Packages pinned to a specific version are usually pinned for a reason: a known-good version for a running service, a version required by a dependent tool, or a reproducibility constraint. Silently changing the installed version could break running services or introduce unexpected behavior. Auto-reconcile also makes `etch apply` a tool that modifies existing state, which conflicts with the idempotency model (skip if already correct).

## Decision

`package.install version:` uses **error-on-mismatch semantics**: when the declared version does not match the installed version, etch returns an error at plan time with the installed and declared versions, and does not generate any install steps. The operator must resolve the mismatch manually before re-running apply.

The three outcomes are:

| Installed state         | Behavior                                        |
| ----------------------- | ----------------------------------------------- |
| Not installed           | Install at declared version                     |
| Correct version         | Skip (no steps emitted — idempotent)            |
| Wrong version installed | Error: "wrong version installed: got X, want Y" |

Per-provider version format:

- **Homebrew**: `brew install <name>@<version>`
- **apt**: `apt-get install <pkg>=<version>`
- **snap**: `version:` is a channel name (e.g. `latest/stable`); compared against the Tracking column of `snap list`

`version:` requires `name:` (not `list:`); incompatible with `cask: true`.

See `examples/package/version-pinning.yaml` for all three provider examples.

## Consequences

- Operators get explicit control — no silent package changes during `etch apply`
- Mismatch surfaces immediately at plan time with a clear message rather than silently passing
- Operators must manually remove and reinstall packages when changing a pinned version
- Does not attempt to auto-upgrade or auto-downgrade; version management remains outside etch's responsibility
- `version:` with `list:` is rejected at plan time with a clear error message

## Related

- [spec: package.install version pinning](../superpowers/specs/2026-05-29-version-pinning-design.md)
- [ADR-0006: Native action expansion strategy](0006-native-action-expansion-strategy.md)
