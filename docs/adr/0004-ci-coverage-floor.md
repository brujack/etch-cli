# ADR-0004: CI Coverage Floor at 70% (Not the Global 90% Standard)

**Date:** 2026-05-16
**Status:** Accepted

## Context

The global Rust standard (`~/.claude/standards/rust.md`) requires a 90% tarpaulin coverage gate. etch-cli cannot reach 90% due to permanently hard-to-cover code paths:

- Network operations: GitHub API, git clone via `gix`, DNS TXT lookups
- CLI binary dispatch (`app/src/commands/apply.rs` and related — only coverable via binary test harnesses, not unit tests)
- Privilege escalation atoms (`sudo`/`root` required)
- Package manager operations requiring live `apt`/`brew` installs
- Tarpaulin instrumentation limits: `error!()`, `trace!()`, `anyhow!()` macro internals and struct literal fields in `return` statements show as uncovered even when the branch executes

The practical ceiling on Linux CI (ubuntu-latest) is ~74%; locally on macOS it is ~83% due to macOS-specific tests.

## Decision

Set the tarpaulin CI gate at 70% (2pp below the observed Linux CI measurement of ~72%). The local macOS gate for running `make test` locally matches CI at 70%.

The 90% global standard does not apply to this codebase. The documented hard-to-cover categories are genuine instrumentation limits, not missing tests.

## Consequences

- CI passes reliably on Linux even with minor coverage fluctuations.
- The 2pp buffer below the observed ~72% provides headroom for minor regressions without blocking work.
- New PRs must not decrease coverage — the floor is a minimum, not a target.
- If new coverable code is added, coverage should improve; if it doesn't, that signals missing tests.

## Related

- `~/.claude/standards/rust.md` — global standard (overridden by this ADR for this repo)
- Coverage ceiling notes in `CLAUDE.md` Testing section
