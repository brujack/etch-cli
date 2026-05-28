# Mutation Score Threshold — Design Spec

**Date:** 2026-05-28
**Status:** Accepted

---

## Context

`mutation-testing.yml` runs monthly and on `workflow_dispatch`. The current `cargo mutants`
step exits 1 when any mutant survives, so the workflow already fails on regressions — but
there is no floor: one missed mutant and one hundred missed mutants both fail the same way.

The desired behavior is a score-based gate: the workflow fails only when the mutation score
drops below a threshold, giving a buffer for inherently hard-to-catch mutants (display
strings, default implementations, format-only functions) while still catching meaningful
regressions.

---

## Decision

Add a **scoring step** to `mutation-testing.yml` that parses `mutants.out/` after
`cargo mutants` completes, computes the mutation score, and fails the workflow when the
score drops below `MINIMUM_MUTATION_SCORE`.

The `cargo mutants` step gets `continue-on-error: true` so the scoring step always runs,
even when mutants are missed. The scoring step is the real gate.

This does **not** change the PR workflow. `mutation-testing.yml` is not in `ci.yml` and
not in the auto-merge `needs:` list. The gate only applies to monthly and on-demand runs.

---

## Threshold Policy

### Computing the threshold

1. Run `cargo mutants --timeout 120 --no-shuffle` from `lib/` to establish the baseline.
2. Record the baseline mutation score `B` (integer percent, e.g. 63).
3. Threshold `T = floor((B - 5) / 5) * 5` — baseline minus 5pp, rounded down to nearest 5.
    - Baseline 78% → T = 70%
    - Baseline 65% → T = 60%
    - Baseline 62% → T = 55%
4. Hardcode `T` as `MINIMUM_MUTATION_SCORE` in the workflow's `env:` block.

### Rationale

- 5pp buffer absorbs natural run-to-run variation and avoids false alarms.
- Rounding to the nearest 5 keeps the threshold stable across minor score fluctuations.
- A PR that deletes test coverage without deleting the tested code will be caught.
- Hard-to-cover mutants (display strings, schema default impls, unreachable format args)
  are known and bounded; they don't need tests.

### Updating the threshold

When tests improve, re-run the baseline, compute the new threshold, and update the
`MINIMUM_MUTATION_SCORE` env var via a PR. The threshold should only ever increase.

---

## Score Computation

### Input files (from `mutants.out/`)

| File           | Meaning                                    |
| -------------- | ------------------------------------------ |
| `caught.txt`   | Mutants caught — at least one test failed  |
| `missed.txt`   | Mutants missed — all tests passed          |
| `timeout.txt`  | Mutants that timed out (counted as caught) |
| `unviable.txt` | Didn't compile — excluded from score       |

### Formula

```
score = (caught + timeout) / (caught + missed + timeout) * 100
```

Timeouts count as caught: the mutation caused observable behavior change (tests became
slow enough to timeout), which is the property we care about.

Unviable mutants are excluded: they never ran tests, so they say nothing about test quality.

### Integer arithmetic

```bash
count_lines() { [[ -f "$1" ]] && grep -c . "$1" || echo 0; }
CAUGHT=$(count_lines mutants.out/caught.txt)
MISSED=$(count_lines mutants.out/missed.txt)
TIMEOUT=$(count_lines mutants.out/timeout.txt)
TOTAL=$((CAUGHT + MISSED + TIMEOUT))
SCORE=$(( TOTAL > 0 ? (CAUGHT + TIMEOUT) * 100 / TOTAL : 100 ))
```

Integer division truncates toward zero — acceptable for a CI gate (conservative).

### Infrastructure error detection

If neither `caught.txt` nor `missed.txt` exists, `cargo mutants` did not produce output
(unmutated baseline failure, I/O error, or timeout). The scoring step treats this as an
error and exits 1.

---

## Workflow Changes

### `mutation-testing.yml` — diff

```yaml
env:
  FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true
  TIMEOUT: ${{ github.event.inputs.timeout || '120' }}
+ MINIMUM_MUTATION_SCORE: "60"   # set from baseline; see spec

  steps:
    ...
    - name: Run mutants
      working-directory: lib
-     run: cargo mutants --timeout "${TIMEOUT}" --no-shuffle
+     run: cargo mutants --timeout "${TIMEOUT}" --no-shuffle
+     continue-on-error: true

+   - name: Score gate
+     working-directory: lib
+     if: always()
+     run: |
+       if [[ ! -f mutants.out/caught.txt && ! -f mutants.out/missed.txt ]]; then
+         printf "ERROR: mutants.out/ missing — cargo mutants did not produce output\n"
+         exit 1
+       fi
+       count_lines() { [[ -f "$1" ]] && grep -c . "$1" || echo 0; }
+       CAUGHT=$(count_lines mutants.out/caught.txt)
+       MISSED=$(count_lines mutants.out/missed.txt)
+       TIMEOUT=$(count_lines mutants.out/timeout.txt)
+       TOTAL=$((CAUGHT + MISSED + TIMEOUT))
+       SCORE=$(( TOTAL > 0 ? (CAUGHT + TIMEOUT) * 100 / TOTAL : 100 ))
+       printf "Mutation score: %s%% (caught=%s missed=%s timeout=%s total=%s)\n" \
+         "${SCORE}" "${CAUGHT}" "${MISSED}" "${TIMEOUT}" "${TOTAL}"
+       if [[ ${SCORE} -lt ${MINIMUM_MUTATION_SCORE} ]]; then
+         printf "FAIL: score %s%% is below threshold %s%%\n" \
+           "${SCORE}" "${MINIMUM_MUTATION_SCORE}"
+         exit 1
+       fi
+       printf "PASS: score %s%% >= threshold %s%%\n" \
+         "${SCORE}" "${MINIMUM_MUTATION_SCORE}"
```

---

## Files Modified

- **Modify:** `.github/workflows/mutation-testing.yml`
    - Add `MINIMUM_MUTATION_SCORE` to top-level `env:` block
    - Add `continue-on-error: true` to the `Run mutants` step
    - Add `Score gate` step after `Run mutants`

---

## Test Plan

1. Simulate a missed-mutant scenario: manually create `mutants.out/missed.txt` with one
   entry and an empty `caught.txt`. Run the scoring script with threshold above 0. Expect
   exit 1 with "score 0% is below threshold".
2. Simulate an all-caught scenario: non-empty `caught.txt`, empty `missed.txt`. Expect
   exit 0 with "PASS".
3. Simulate the infrastructure error: no `mutants.out/` directory at all. Expect exit 1
   with "mutants.out/ missing".
4. Verify the actual baseline run with the real crate and the computed threshold.

---

## Hard-to-Cover Mutants (Known Ceiling)

These mutant classes survive because they mutate code the tests don't exercise through
observable behavior:

- `summarize()` / `display()` / `output_string()` / `error_message()` — formatting only;
  no test asserts on their output
- `json_schema()` defaults — schema gen code not exercised by unit tests
- Context `to_tera()` default — integration-only path
- `resolve()` in manifests — path logic only exercised when running `etch apply`

The threshold is set to accommodate these known misses. Any new missed mutant in core
action/atom logic is a regression.
