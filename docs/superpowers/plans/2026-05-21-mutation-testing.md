> **Status: DONE**

# Mutation Testing — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add cargo-mutants mutation testing to etch-lib (the correctness-critical action library) via a monthly GitHub Actions workflow and a local Makefile target.

**Architecture:** Single `mutation-testing.yml` workflow mirroring math's pattern — `workflow_dispatch` with optional timeout input plus monthly schedule. Runs `cargo mutants` from the `lib/` directory targeting etch-lib only. `make mutants` target for local runs. Non-blocking: not in `ci.yml`, does not affect PR cycle time.

**Tech Stack:** `cargo-mutants`, GitHub Actions, `actions/upload-artifact@v5`

---

## Files

- **Create:** `.github/workflows/mutation-testing.yml`
- **Modify:** `Makefile` — add `mutants` target
- **Modify:** `docs/superpowers/README.md` — **post-merge on main only**

---

## Task 1: Add `mutants` target to Makefile

**Files:**

- Modify: `Makefile`

- [ ] **Step 1: Read current Makefile**

```bash
cat Makefile
```

Confirm it ends after `install-hooks`. The `mutants` target goes at the end.

- [ ] **Step 2: Add mutants target**

Append to `Makefile` after the `install-hooks` target:

```makefile
mutants:
	cd lib && cargo mutants --timeout 120 --no-shuffle
```

Also add `mutants` to the `.PHONY` line:

```makefile
.PHONY: all test lint build build-linux install-hooks mutants
```

- [ ] **Step 3: Verify cargo-mutants is installed (or installable)**

```bash
cargo mutants --version 2>/dev/null || echo "not installed — install with: cargo install cargo-mutants --locked"
```

If not installed, install it:

```bash
cargo install cargo-mutants --locked
```

- [ ] **Step 4: Validate the target runs (optional dry-run)**

```bash
make mutants -- --list 2>&1 | head -20
```

Expected: lists mutant candidates from etch-lib. If `cargo mutants` isn't installed locally, skip — CI will install it.

- [ ] **Step 5: Commit**

```bash
git add Makefile
git commit -m "build: add mutants target for etch-lib mutation testing

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 2: Create mutation-testing.yml workflow

**Files:**

- Create: `.github/workflows/mutation-testing.yml`

- [ ] **Step 1: Create the workflow file**

```yaml
name: mutation-testing

# Runs cargo-mutants against etch-lib to verify tests catch behavior changes.
# Coverage % measures whether code is exercised; mutation testing measures
# whether tests catch behavior changes.
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

jobs:
    mutants:
        name: Mutation testing (etch-lib)
        runs-on: ubuntu-latest
        timeout-minutes: 120
        steps:
            - uses: actions/checkout@v5

            - uses: dtolnay/rust-toolchain@stable

            - uses: Swatinem/rust-cache@v2
              with:
                  workspaces: lib

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

- [ ] **Step 2: Validate YAML**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/mutation-testing.yml'))" && echo "valid"
```

Expected: `valid`

- [ ] **Step 3: Confirm workflow does NOT appear in ci.yml requires**

```bash
grep "mutation" .github/workflows/ci.yml || echo "not referenced in ci.yml — correct"
```

Expected: `not referenced in ci.yml — correct`

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/mutation-testing.yml
git commit -m "ci: add monthly mutation testing workflow for etch-lib

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 3: Post-merge docs update

> **Do this directly on main after the PR merges — not inside the worktree.**

- [ ] **Step 1: Update plan index**

In `docs/superpowers/README.md`, update the mutation-testing row: add plan link, set status to Done.

- [ ] **Step 2: Add Done banner**

Add `> **Status: DONE**` at the top of `docs/superpowers/plans/2026-05-21-mutation-testing.md`.

- [ ] **Step 3: Commit on main**

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-05-21-mutation-testing.md
git commit -m "docs: mark mutation-testing plan done

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
git push
```
