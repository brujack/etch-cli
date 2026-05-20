# etch-cli Release Pipeline — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `workflow_dispatch` release pipeline to etch-cli that runs tests, builds a Linux release binary, and publishes a GitHub release with the binary attached.

**Architecture:** Single `release.yml` workflow triggered manually with a `version` input. Mirrors math's `release-factorial-rs.yml` pattern. A `sign` job stub (calling `release-sign.yml`) is wired in but left as a placeholder until Plan 2 adds that workflow.

**Tech Stack:** GitHub Actions, `cargo build --release`, `softprops/action-gh-release@v3`, `actions/checkout@v5`

---

## Files

- **Create:** `.github/workflows/release.yml`
- **Modify:** `docs/superpowers/README.md` — **post-merge on main only**

---

## Task 1: Create release.yml

**Files:**

- Create: `.github/workflows/release.yml`

No unit-testable code. Validation: YAML parses, dry-run with `workflow_dispatch` on a test tag.

- [ ] **Step 1: Create `.github/workflows/release.yml`**

```yaml
name: Release etch-cli

on:
    workflow_dispatch:
        inputs:
            version:
                description: "Version number without the v prefix (e.g. 1.2.0)"
                required: true
                type: string

env:
    FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true

permissions:
    contents: write
    id-token: write

jobs:
    release:
        name: Release etch-cli
        runs-on: ubuntu-latest
        steps:
            - uses: actions/checkout@v5
              with:
                  fetch-depth: 0

            - uses: dtolnay/rust-toolchain@stable

            - uses: Swatinem/rust-cache@v2

            - name: Install cargo-nextest
              run: cargo install cargo-nextest --locked

            - name: Run tests
              run: make test

            - name: Build release binary
              run: cargo build --release

            - name: Generate release notes
              id: notes
              run: |
                  PREV_TAG=$(git describe --tags --abbrev=0 --match="v*" 2>/dev/null || true)
                  if [ -n "${PREV_TAG}" ]; then
                    NOTES=$(git log "${PREV_TAG}..HEAD" --pretty=format:"- %s" || true)
                  else
                    NOTES=$(git log HEAD --pretty=format:"- %s" || true)
                  fi
                  DELIMITER="EOF_$(openssl rand -hex 8)"
                  {
                    printf 'notes<<%s\n' "${DELIMITER}"
                    printf '%s\n' "${NOTES}"
                    printf '%s\n' "${DELIMITER}"
                  } >> "$GITHUB_OUTPUT"

            - name: Create and push tag
              env:
                  VERSION: ${{ inputs.version }}
              run: |
                  git config user.name "github-actions[bot]"
                  git config user.email "github-actions[bot]@users.noreply.github.com"
                  git tag "v${VERSION}"
                  git push origin "v${VERSION}"

            - name: Create GitHub release
              uses: softprops/action-gh-release@v3
              with:
                  tag_name: "v${{ inputs.version }}"
                  name: "v${{ inputs.version }}"
                  body: "${{ steps.notes.outputs.notes }}"
                  files: target/release/etch
```

- [ ] **Step 2: Validate YAML**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo "valid"
```

Expected: `valid`

- [ ] **Step 3: Confirm `target/release/etch` is the correct binary path**

```bash
cargo build --release 2>&1 | tail -3
ls -la target/release/etch
```

Expected: binary exists at `target/release/etch`.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add release pipeline for etch-cli

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 2: Post-merge docs update

> **Do this directly on main after the PR merges — not inside the worktree.**

- [ ] **Step 1: Update plan index**

In `docs/superpowers/README.md`, update the release-pipeline row: add plan link, set status to Done.

- [ ] **Step 2: Add Done banner**

Add `> **Status: DONE**` at the top of `docs/superpowers/plans/2026-05-20-release-pipeline.md`.

- [ ] **Step 3: Commit on main**

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-05-20-release-pipeline.md
git commit -m "docs: mark release-pipeline plan done

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
git push
```
