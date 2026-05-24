# cargo semver-checks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `cargo semver-checks` to detect unintentional breaking API changes in `etch-lib` — advisory on PRs (compares HEAD vs `origin/main`), blocking at release time (compares HEAD vs previous git tag).

**Architecture:** Three touch points: a `semver` Makefile target for local use, a `semver-check` CI job (advisory, `continue-on-error: true`, not in auto-merge `needs`), and a step in the release workflow inserted after tests and before the build (blocking, no `continue-on-error`). All three run `cargo semver-checks check-release -p etch-lib`.

**Tech Stack:** `cargo-semver-checks` (installed via `cargo install --locked`), GitHub Actions, Make.

---

## File Structure

- Modify: `Makefile` — add `semver` target and add it to `.PHONY`
- Modify: `.github/workflows/ci.yml` — add `semver-check` job (advisory)
- Modify: `.github/workflows/release.yml` — add install + check steps after `Run tests`
- Modify: `docs/superpowers/README.md` — add row to All Plans table (In Progress)

---

### Task 1: Add `semver` Makefile target

**Files:**

- Modify: `Makefile`

- [ ] **Step 1: Install cargo-semver-checks locally**

```bash
cargo install cargo-semver-checks --locked
```

Expected: installs without error. Verify with `cargo semver-checks --version`.

- [ ] **Step 2: Add `semver` to `.PHONY` and add the target**

Replace the `.PHONY` line and add the target after `bench`:

```makefile
.PHONY: all test lint build build-linux install-hooks mutants bench changelog fuzz fuzz-manifest fuzz-path semver
```

Add after the `bench` target:

```makefile
semver:
	cargo semver-checks check-release -p etch-lib --baseline-rev origin/main
```

The full `Makefile` after changes (showing the relevant section — do not change anything else):

```makefile
.PHONY: all test lint build build-linux install-hooks mutants bench changelog fuzz fuzz-manifest fuzz-path semver

...

bench:
	cargo bench -p etch-lib

semver:
	cargo semver-checks check-release -p etch-lib --baseline-rev origin/main

changelog:
	git-cliff -o CHANGELOG.md
```

- [ ] **Step 3: Verify the target runs**

```bash
git fetch origin main --depth=1
make semver
```

Expected: exits 0 with output like:

```
Finished checking etch-lib v0.9.3
```

If `cargo-semver-checks` is not installed, you get `command not found: cargo-semver-checks` — install it (Step 1) first.

- [ ] **Step 4: Commit**

```bash
git add Makefile
git commit -m "feat(ci): add semver Makefile target for etch-lib"
```

---

### Task 2: Add advisory `semver-check` CI job

**Files:**

- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Add the `semver-check` job**

Add this job block between the `docs-build` job and the `auto-merge` job. Insert it so the `auto-merge` job remains last:

```yaml
semver-check:
    name: Semver Check
    runs-on: ubuntu-latest
    continue-on-error: true
    steps:
        - uses: actions/checkout@v6

        - name: Install Rust stable
          uses: dtolnay/rust-toolchain@stable

        - uses: Swatinem/rust-cache@v2

        - name: Fetch main for baseline
          run: git fetch origin main --depth=1

        - name: Install cargo-semver-checks
          run: cargo install cargo-semver-checks --locked

        - name: Check semver compatibility
          run: cargo semver-checks check-release -p etch-lib --baseline-rev origin/main
```

**Do NOT add `semver-check` to the `needs` list of the `auto-merge` job.** The `auto-merge` job must remain:

```yaml
auto-merge:
    name: Auto Merge
    runs-on: ubuntu-latest
    needs: [test, cargo-audit, secret-scan, snyk-scan, docs-lint, docs-build]
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "feat(ci): add advisory semver-check job for etch-lib"
```

---

### Task 3: Add blocking semver-check step to release workflow

**Files:**

- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Add the install and check steps**

In `release.yml`, locate this existing step:

```yaml
- name: Run tests
  run: make test
```

Insert two new steps immediately after it, before `Build release binary`:

```yaml
- name: Install cargo-semver-checks
  run: cargo install cargo-semver-checks --locked

- name: Check semver compatibility
  run: |
      PREV_TAG=$(git tag --sort=-version:refname | grep "^v" | head -1)
      if [ -n "${PREV_TAG}" ]; then
        cargo semver-checks check-release -p etch-lib --baseline-rev "${PREV_TAG}"
      else
        echo "No previous tag found — skipping semver check"
      fi
```

The step has no `continue-on-error` — a semver violation stops the release.

The release workflow already uses `fetch-depth: 0` on checkout so all git tags are available for `git tag --sort=-version:refname`.

After the edit, the relevant section of `release.yml` should look like:

```yaml
- name: Install cargo-nextest
  run: cargo install cargo-nextest --locked

- name: Run tests
  run: make test

- name: Install cargo-semver-checks
  run: cargo install cargo-semver-checks --locked

- name: Check semver compatibility
  run: |
      PREV_TAG=$(git tag --sort=-version:refname | grep "^v" | head -1)
      if [ -n "${PREV_TAG}" ]; then
        cargo semver-checks check-release -p etch-lib --baseline-rev "${PREV_TAG}"
      else
        echo "No previous tag found — skipping semver check"
      fi

- name: Build release binary
  run: cargo build --release
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "feat(ci): add blocking semver-check step to release workflow"
```

---

### Task 4: Update docs and open PR

**Files:**

- Modify: `docs/superpowers/README.md`

- [ ] **Step 1: Add row to All Plans table**

In `docs/superpowers/README.md`, add this row to the All Plans table after the `criterion-benchmarks` row:

```markdown
| 2026-05-24 | [cargo-semver-checks](plans/2026-05-24-cargo-semver-checks.md) | [spec](specs/2026-05-24-cargo-semver-checks-design.md) | In Progress |
```

Also remove the `cargo semver-checks` row from the Backlog table (the feature has a plan now).

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/README.md
git commit -m "docs(superpowers): add cargo-semver-checks plan — In Progress"
```

- [ ] **Step 3: Push and open PR**

```bash
git push origin feat/cargo-semver-checks
gh pr create \
  --title "feat(ci): add cargo-semver-checks for etch-lib API compatibility" \
  --body "$(cat <<'EOF'
## Summary

- Adds `make semver` Makefile target (local developer check)
- Adds advisory `semver-check` CI job on PRs (compares HEAD vs `origin/main`, `continue-on-error: true`)
- Adds blocking semver-check step in release workflow (compares HEAD vs previous git tag)

## Test plan

- [ ] `make semver` runs locally and exits 0 on current main (no breaking changes)
- [ ] CI shows `Semver Check` job on this PR
- [ ] `Semver Check` job passes (or warns on any advisory finding)
- [ ] `Semver Check` NOT in auto-merge blocking requirements

## Definition of Done

- All CI checks pass (Semver Check advisory only)
- PR auto-merges
EOF
)"
```

- [ ] **Step 4: Monitor CI — confirm `Semver Check` job appears and runs**

```bash
gh pr checks --watch
```

Expected: `Semver Check` appears in the list with `continue-on-error` — it shows as pass or warn but does NOT block auto-merge.

- [ ] **Step 5: After PR merges — update docs on main** _(Do this directly on main after the PR merges — not inside the worktree.)_

```bash
git fetch --prune
git reset --hard origin/main
```

In `docs/superpowers/README.md`, change the `cargo-semver-checks` row status from `In Progress` to `Done` and add `> **Status: DONE**` at the top of `docs/superpowers/plans/2026-05-24-cargo-semver-checks.md`.

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-05-24-cargo-semver-checks.md
git commit -m "docs(superpowers): mark cargo-semver-checks Done"
git push origin main
```
