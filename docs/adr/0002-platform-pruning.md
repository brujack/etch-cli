# ADR-0002: Prune to macOS and Ubuntu 24.04/26.04 Only

**Date:** 2026-05-02
**Status:** Accepted

## Context

The comtrya fork inherited provider code for Windows, RHEL/CentOS, older Ubuntu versions (20.04, 22.04), and various other platforms. The target machines are a Mac Studio M1 Ultra (macOS aarch64) and a Linux workstation running Ubuntu 24.04 (AMD Ryzen 9 7950X, x86_64). Provider files for unsupported platforms added dead code, lint noise, and untested paths.

## Decision

Remove all provider files and platform-specific code for: Windows (WSL2), RHEL/CentOS, Ubuntu 20.04, Ubuntu 22.04, Debian, FreeBSD, and any other non-target platforms. Keep: macOS (all versions), Ubuntu 24.04, Ubuntu 26.04 (forward-looking).

Platform detection uses `os_info::Type` at runtime; removed variants no longer compile, so there is no silent runtime fallback to removed platforms.

## Consequences

- 11 provider files removed — simpler codebase, no dead code warnings from unused imports.
- CI runs only on ubuntu-latest; macOS provider tests run locally only (`#[cfg(target_os = "macos")]`).
- If a third machine is added on a different distro in future, the relevant provider must be re-added from the git history.
- Coverage gap: macOS-only tests add ~10pp locally that Linux CI never sees, creating a permanent macOS/CI coverage differential.

## Related

- [Platform pruning spec](../superpowers/specs/2026-05-02-platform-pruning-design.md)
- [Platform pruning plan](../superpowers/plans/2026-05-02-platform-pruning.md)
