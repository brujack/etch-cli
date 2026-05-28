# Mutation Score Threshold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a minimum mutation score gate to `mutation-testing.yml` so the workflow fails when the mutation score drops below a threshold derived from the baseline run.

**Architecture:** Two tasks in sequence — Task 1 establishes the baseline score and computes the threshold (a one-time measurement); Task 2 modifies the workflow to embed that threshold and add the scoring step. The `cargo mutants` step gets `continue-on-error: true` so the scoring step always runs regardless of whether mutants survived.

**Tech Stack:** GitHub Actions (bash scripting), `cargo mutants` (outputs `mutants.out/caught.txt`, `missed.txt`, `timeout.txt`).

---

**Note:** A baseline run (`cargo mutants --timeout 120 --no-shuffle` from `lib/`) may already be in progress or completed from the current session. If `lib/mutants.out/` already exists with output files, skip to Task 1 Step 2.

---

### Task 1: Establish baseline mutation score and compute threshold

**Files:** None modified — this task produces the threshold value needed by Task 2.

- [ ] **Step 1: Run baseline (skip if lib/mutants.out/ already exists)**

```bash
cd lib
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"
cargo mutants --timeout 120 --no-shuffle 2>&1 | tee /tmp/mutants-baseline.log
```

Expected: run completes with a summary line like `781 mutants: 520 caught, 240 missed, 5 timeout, 16 unviable`. This takes 60–120 minutes.

If `cargo mutants` is not installed:

```bash
cargo install cargo-mutants --locked
```

- [ ] **Step 2: Compute baseline score and threshold**

```bash
cd lib
count_lines() { [[ -f "$1" ]] && grep -c . "$1" || echo 0; }
CAUGHT=$(count_lines mutants.out/caught.txt)
MISSED=$(count_lines mutants.out/missed.txt)
TIMEOUT=$(count_lines mutants.out/timeout.txt)
TOTAL=$((CAUGHT + MISSED + TIMEOUT))
BASELINE=$(( TOTAL > 0 ? (CAUGHT + TIMEOUT) * 100 / TOTAL : 100 ))
THRESHOLD=$(( (BASELINE - 5) / 5 * 5 ))
printf "Baseline: %s%%  Threshold: %s%%  (caught=%s missed=%s timeout=%s)\n" \
  "${BASELINE}" "${THRESHOLD}" "${CAUGHT}" "${MISSED}" "${TIMEOUT}"
```

**Record both values** — you need them in Task 2.

Example output for a 67% baseline:

```
Baseline: 67%  Threshold: 60%  (caught=522 missed=256 timeout=3)
```

---

### Task 2: Implement the score gate in mutation-testing.yml

**Files:**

- Modify: `.github/workflows/mutation-testing.yml`

This task requires the `BASELINE` and `THRESHOLD` values from Task 1.

- [ ] **Step 1: Read the current workflow to understand what you're modifying**

```bash
cat .github/workflows/mutation-testing.yml
```

Current structure (from spec):

```yaml
env:
    FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true
    TIMEOUT: ${{ github.event.inputs.timeout || '120' }}

jobs:
    mutants:
        steps:
            - uses: actions/checkout@v6
            - uses: dtolnay/rust-toolchain@stable
            - uses: Swatinem/rust-cache@v2
            - name: Install cargo-mutants
              run: cargo install cargo-mutants --locked
            - name: Run mutants
              working-directory: lib
              run: cargo mutants --timeout "${TIMEOUT}" --no-shuffle
            - name: Upload mutants output
              if: always()
              uses: actions/upload-artifact@v5
              with:
                  name: mutants-output
                  path: lib/mutants.out/
                  retention-days: 30
```

- [ ] **Step 2: Write the updated workflow**

Replace `.github/workflows/mutation-testing.yml` with the content below. Substitute `[THRESHOLD]` with the integer value from Task 1 (e.g. `60`). Do not add a `%` sign — the value is a bare integer.

```yaml
name: mutation-testing

# Runs cargo-mutants against etch-lib to verify tests catch behavior changes.
# Coverage % measures whether code is exercised; mutation testing measures
# whether tests actually catch behavior changes — a much stronger quality signal.
#
# Triggered manually (workflow_dispatch) or monthly on schedule. Not on every PR
# because mutant runs can take 20-60 minutes for etch-lib.

on:
    workflow_dispatch:
        inputs:
            timeout:
                description: "Per-mutant timeout in seconds (default: 120)"
                required: false
                default: "120"
    schedule:
        # Monthly run catches test-quality regressions
        - cron: "0 4 1 * *"

env:
    FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true
    TIMEOUT: ${{ github.event.inputs.timeout || '120' }}
    MINIMUM_MUTATION_SCORE: "[THRESHOLD]"

jobs:
    mutants:
        name: Mutation testing (etch-lib)
        runs-on: ubuntu-latest
        timeout-minutes: 120
        steps:
            - uses: actions/checkout@v6

            - uses: dtolnay/rust-toolchain@stable

            - uses: Swatinem/rust-cache@v2
              with:
                  workspaces: lib

            - name: Install cargo-mutants
              run: cargo install cargo-mutants --locked

            - name: Run mutants
              working-directory: lib
              run: cargo mutants --timeout "${TIMEOUT}" --no-shuffle
              continue-on-error: true

            - name: Score gate
              working-directory: lib
              if: always()
              run: |
                  if [[ ! -f mutants.out/caught.txt && ! -f mutants.out/missed.txt ]]; then
                    printf "ERROR: mutants.out/ missing — cargo mutants did not produce output\n"
                    exit 1
                  fi
                  count_lines() { [[ -f "$1" ]] && grep -c . "$1" || echo 0; }
                  CAUGHT=$(count_lines mutants.out/caught.txt)
                  MISSED=$(count_lines mutants.out/missed.txt)
                  TIMEOUT=$(count_lines mutants.out/timeout.txt)
                  TOTAL=$((CAUGHT + MISSED + TIMEOUT))
                  SCORE=$(( TOTAL > 0 ? (CAUGHT + TIMEOUT) * 100 / TOTAL : 100 ))
                  printf "Mutation score: %s%% (caught=%s missed=%s timeout=%s total=%s)\n" \
                    "${SCORE}" "${CAUGHT}" "${MISSED}" "${TIMEOUT}" "${TOTAL}"
                  if [[ ${SCORE} -lt ${MINIMUM_MUTATION_SCORE} ]]; then
                    printf "FAIL: score %s%% is below threshold %s%%\n" \
                      "${SCORE}" "${MINIMUM_MUTATION_SCORE}"
                    exit 1
                  fi
                  printf "PASS: score %s%% >= threshold %s%%\n" \
                    "${SCORE}" "${MINIMUM_MUTATION_SCORE}"

            - name: Upload mutants output
              if: always()
              uses: actions/upload-artifact@v5
              with:
                  name: mutants-output
                  path: lib/mutants.out/
                  retention-days: 30
```

- [ ] **Step 3: Verify the scoring script locally against the baseline output**

```bash
cd lib
MINIMUM_MUTATION_SCORE=[THRESHOLD]
if [[ ! -f mutants.out/caught.txt && ! -f mutants.out/missed.txt ]]; then
  printf "ERROR: mutants.out/ missing\n"; exit 1
fi
count_lines() { [[ -f "$1" ]] && grep -c . "$1" || echo 0; }
CAUGHT=$(count_lines mutants.out/caught.txt)
MISSED=$(count_lines mutants.out/missed.txt)
TIMEOUT=$(count_lines mutants.out/timeout.txt)
TOTAL=$((CAUGHT + MISSED + TIMEOUT))
SCORE=$(( TOTAL > 0 ? (CAUGHT + TIMEOUT) * 100 / TOTAL : 100 ))
printf "Score: %s%%  Threshold: %s%%\n" "${SCORE}" "${MINIMUM_MUTATION_SCORE}"
[[ ${SCORE} -lt ${MINIMUM_MUTATION_SCORE} ]] && printf "FAIL\n" || printf "PASS\n"
```

Expected: `PASS` (because the threshold was set from this same baseline).

- [ ] **Step 4: Simulate a failure case to verify the gate works**

```bash
cd lib
# Temporarily move caught.txt aside so score drops to 0%
mv mutants.out/caught.txt mutants.out/caught.txt.bak

MINIMUM_MUTATION_SCORE=[THRESHOLD]
count_lines() { [[ -f "$1" ]] && grep -c . "$1" || echo 0; }
CAUGHT=$(count_lines mutants.out/caught.txt)
MISSED=$(count_lines mutants.out/missed.txt)
TIMEOUT=$(count_lines mutants.out/timeout.txt)
TOTAL=$((CAUGHT + MISSED + TIMEOUT))
SCORE=$(( TOTAL > 0 ? (CAUGHT + TIMEOUT) * 100 / TOTAL : 100 ))
printf "Score: %s%%  Threshold: %s%%\n" "${SCORE}" "${MINIMUM_MUTATION_SCORE}"
[[ ${SCORE} -lt ${MINIMUM_MUTATION_SCORE} ]] && printf "FAIL\n" || printf "PASS\n"

# Restore
mv mutants.out/caught.txt.bak mutants.out/caught.txt
```

Expected: `FAIL` with score `0%`.

- [ ] **Step 5: Simulate the infrastructure-error case**

```bash
MINIMUM_MUTATION_SCORE=[THRESHOLD]
if [[ ! -f /tmp/nonexistent/caught.txt && ! -f /tmp/nonexistent/missed.txt ]]; then
  printf "ERROR: mutants.out/ missing — cargo mutants did not produce output\n"
  result="exit 1"
fi
printf "Result: %s\n" "$result"
```

Expected: `Result: exit 1`.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/mutation-testing.yml
git commit -m "ci(mutants): add mutation score gate at [THRESHOLD]%

Threshold derived from baseline: baseline=[BASELINE]%, threshold=[THRESHOLD]%.
Score formula: (caught + timeout) / (caught + missed + timeout) * 100.
continue-on-error on cargo mutants step so scoring step always runs.
Infrastructure error (missing mutants.out/) is treated as hard failure."
```

Replace `[THRESHOLD]` and `[BASELINE]` with the actual values from Task 1.

---

### Task 3: Update plan index and add DONE banner

> **Do this directly on main after the PR merges — not inside the worktree.**

- [ ] **Step 1: Mark the plan Done in docs/superpowers/README.md**

In `docs/superpowers/README.md`, find the row:

```
| 2026-05-28 | —  | [mutation-score-threshold](specs/2026-05-28-mutation-score-threshold-design.md) | In Progress |
```

Update it to:

```
| 2026-05-28 | [mutation-score-threshold](plans/2026-05-28-mutation-score-threshold.md) | [mutation-score-threshold](specs/2026-05-28-mutation-score-threshold-design.md) | Done |
```

- [ ] **Step 2: Add DONE banner to this plan file**

At the top of `docs/superpowers/plans/2026-05-28-mutation-score-threshold.md`, add:

```markdown
> **Status: DONE**
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-05-28-mutation-score-threshold.md
git commit -m "docs(superpowers): mark mutation-score-threshold Done"
```
