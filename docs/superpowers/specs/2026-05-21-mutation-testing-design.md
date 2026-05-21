# Mutation Testing — Design Spec

**Date:** 2026-05-21
**Status:** Accepted

---

## Context

etch-cli is a correctness-critical Rust tool — wrong actions could corrupt filesystem state,
overwrite files, or leave systems in inconsistent states. The global `tdd.md` standard
requires cargo-mutants for correctness-critical Rust repos. math already follows this pattern.

Coverage % (currently ~72% on Linux CI) measures whether code is exercised. Mutation testing
measures whether tests actually catch behavior changes — a much stronger quality signal.

---

## Decision

Add `cargo mutants` mutation testing targeting `etch-lib` (the library crate containing all
action logic). Mirrors math's established pattern: `workflow_dispatch` + monthly schedule,
non-blocking, results uploaded as artifacts.

---

## Files

- **Create:** `.github/workflows/mutation-testing.yml`
- **Modify:** `Makefile` — add `mutants` target

---

## Workflow Design

`.github/workflows/mutation-testing.yml`:

- **Triggers:** `workflow_dispatch` (with optional `timeout` input, default `120`) +
  monthly schedule `cron: "0 4 1 * *"`
- **Job:** single `mutants` job, `ubuntu-latest`, `timeout-minutes: 120`
- **Steps:** checkout@v5, rust-toolchain@stable, rust-cache@v2, install cargo-mutants,
  run mutation testing, upload artifact
- **Mutation command:** `cargo mutants --timeout ${TIMEOUT} --no-shuffle`
  run from `lib/` directory (targets etch-lib only)
- **Artifact:** `lib/mutants.out/` uploaded as `mutants-output`, 30-day retention, `if: always()`

## Makefile

```makefile
mutants:
	cd lib && cargo mutants --timeout 120 --no-shuffle
```

---

## Consequences

- Monthly runs catch test-quality regressions automatically
- `workflow_dispatch` allows on-demand runs (e.g. after adding new action logic)
- No impact on PR cycle time — not in ci.yml
- First run establishes baseline surviving mutant count; future runs show trends
- `jsonschemagen/` and `app/` excluded — minimal correctness-critical logic there
