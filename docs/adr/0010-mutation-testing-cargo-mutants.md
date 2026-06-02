# ADR-0010: Mutation testing with cargo-mutants

**Date:** 2026-05-24
**Status:** Accepted

## Context

Coverage at 70% floor (ADR-0004) measures whether lines execute during tests. It does not verify that tests actually detect behavior changes. A mutation that changes `>` to `>=` in an idempotency guard is still "covered" — the line runs — but no test may detect the behavioral difference. This gap is especially important for etch-lib, where the correctness of idempotency guards is the core invariant.

`cargo-mutants` applies systematic mutations to the source (operator replacements, literal changes, return value inversions) and checks whether the test suite detects each one. A mutation that is not detected ("survived") indicates either an equivalent mutation or a gap in test assertions.

Alternatives considered: `mutagen` (Python-focused), manual mutation review (not scalable), property-based testing only (Hypothesis/proptest covers input ranges, not operator correctness).

## Decision

Add `mutation-testing.yml` workflow running `cargo mutants` against `etch-lib/` on a monthly schedule and on-demand via `workflow_dispatch`. Configuration:

- `--timeout 120` per mutant to avoid infinite loops from sieve-style mutations
- `--no-shuffle` for reproducible ordering
- 60% kill rate gate: the workflow computes `(caught + timeout) / total` and fails if below 0.60
- Results uploaded as artifact (30-day retention) so surviving mutants can be inspected without re-running

**60% kill rate chosen (not 80%):** etch-lib has many step functions (`brew.execute`, `apt.execute`, `gem.execute`, etc.) that invoke external binaries. These functions cannot be meaningfully mutated in CI without real system state — mutations in these paths survive because no test exercises them. The coverable core logic (idempotency guards, manifest parsing, struct validation) targets 60%+ kill rate; step functions are structurally immune to mutation testing in a sandboxed environment.

Runs are non-blocking on PRs (monthly + manual only) — mutation testing runs are 20–60 minutes and not suitable for per-PR CI. The score gate is enforced within the workflow but the workflow itself is not required for auto-merge.

## Consequences

- Monthly runs catch test quality regressions in etch-lib's core logic independent of the coverage percentage.
- Surviving mutants above the 60% gate require investigation: either add an assertion that catches the mutation, document it as an equivalent mutation, or add it to `exclude_re` in `.cargo/mutants.toml` with justification.
- `// mutants::skip` comments are not supported in cargo-mutants 27.x — use `.cargo/mutants.toml` with `exclude_re` for expression-level exclusions.
- `etch-lib/mutants.out/` is gitignored — results are ephemeral and accessed via CI artifacts.
- The 60% gate will need revisiting if the proportion of step functions (structurally untestable) vs core logic changes significantly.

## Related

- ADR-0004: CI coverage floor at 70% (complementary quality gate)
- `etch-lib/.cargo/mutants.toml` — exclusion configuration
- `mutation-testing.yml` — monthly workflow
- dotfiles `standards/rust.md` — cargo-mutants usage patterns and `exclude_re` syntax
