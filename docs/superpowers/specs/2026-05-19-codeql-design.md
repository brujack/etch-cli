# CodeQL Workflow — Design Spec

**Date:** 2026-05-19
**Status:** Accepted

---

## Context

etch-cli is a public Rust repo. GitHub CodeQL is free for public repositories. Existing CI
covers functional correctness (test, cargo-audit, snyk-scan) but has no static application
security testing (SAST) layer. CodeQL adds cross-cutting vulnerability detection (buffer
issues, injection patterns, logic flaws) beyond what cargo-audit covers.

---

## Decision

Add a standalone `.github/workflows/codeql.yml` workflow. CodeQL results surface in the
Security → Code scanning tab and are advisory — they do not gate auto-merge.

---

## Design

### Trigger

- `pull_request` targeting `master` or `main`
- `schedule`: weekly, Saturday 03:00 UTC (catches new CVE rules without a PR)

### Job

Single job `analyze` with matrix `language: [rust]`.

- `build-mode: none` — Rust CodeQL database creation requires no build step (confirmed
  supported for Rust without Kotlin)
- `actions/checkout@v5` — matches existing CI standard
- `github/codeql-action/init@v3`
- `github/codeql-action/analyze@v3`
- `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true` env — matches existing CI standard

### Not in auto-merge

CodeQL is advisory. SAST produces false positives that should not block valid merges.
The workflow is intentionally excluded from `auto-merge.needs` in `ci.yml`.

---

## Consequences

- Security alerts appear in the Security → Code scanning tab on GitHub
- Weekly schedule catches new CodeQL query rule releases
- Zero impact on PR merge speed (runs in parallel, not a gate)
- No changes to `ci.yml` — fully isolated workflow file
