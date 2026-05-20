# ADR-0005: CodeQL SAST Is Advisory

**Date:** 2026-05-19
**Status:** Accepted

## Context

etch-cli is a public Rust repo. GitHub CodeQL is free for public repositories and provides
SAST (static application security testing) coverage beyond what cargo-audit or Snyk offer —
catching logic flaws, injection patterns, and vulnerability classes at the code level.

The question was whether CodeQL results should gate merges (blocking auto-merge) or be
advisory (surfaced in the Security tab without blocking).

Alternatives considered:

- **Blocking (required check)** — CodeQL in `auto-merge.needs`, PR cannot merge until
  CodeQL passes. Rejected: SAST tools produce false positives; a solo-dev repo with rapid
  iteration shouldn't be gated on a scan that can take 5–15 minutes and fire on benign patterns.
- **Advisory (separate workflow)** — CodeQL runs in parallel, results in Security tab, not
  in auto-merge gate. Chosen.
- **Skip CodeQL entirely** — No SAST layer beyond cargo-audit. Rejected: CodeQL is free for
  public repos and catches a different class of issues.

## Decision

CodeQL runs as a standalone advisory workflow (`.github/workflows/codeql.yml`), not as a
job in `ci.yml`. It is not in `auto-merge.needs`. Results surface in the GitHub Security →
Code scanning tab.

- Trigger: `pull_request` targeting master/main + weekly schedule (Saturday 03:00 UTC)
- Language: Rust, `build-mode: none` (no explicit build step required)
- Not referenced from `ci.yml` — fully isolated workflow

## Consequences

- Security alerts appear in the Security tab without slowing PR merges
- Weekly schedule catches new CodeQL rule releases between PRs
- False positives require manual dismissal in the Security tab rather than blocking the branch
- If a confirmed true positive is found and severity warrants it, the workflow can be promoted
  to a required check by adding it to `auto-merge.needs` in `ci.yml`

## Related

- [.github/workflows/codeql.yml](../../.github/workflows/codeql.yml)
- [docs/superpowers/specs/2026-05-19-codeql-design.md](../superpowers/specs/2026-05-19-codeql-design.md)
- [ADR-0004: CI Coverage Floor](0004-ci-coverage-floor.md)
