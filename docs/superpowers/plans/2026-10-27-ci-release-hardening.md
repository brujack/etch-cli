# CI and Release Hardening Implementation Plan

spec: docs/superpowers/specs/2026-10-27-ci-release-hardening-design.md

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** [2026-10-27-ci-release-hardening-design.md](../specs/2026-10-27-ci-release-hardening-design.md)

**Goal:** Make etch-cli's security scans able to fail, verify every downloaded tool, produce correct releases, and keep builds inside the repository.

**Architecture:** Four sequential PRs (A: tasks 1-3, B: 4-5, C: 6, D: 7). Each workflow invariant is enforced by a pytest module under `tests/` that scans the workflow text without PyYAML, matching `tests/test_automerge_bot_hold.py`. Behavioural checks extract a step's `run:` text and execute it under `bash -e` against fixtures.

**Tech Stack:** GitHub Actions, pytest, bash, git-cliff, taiki-e/install-action, GNU make.

## Global Constraints

- Pinned values are the spec's table, copied exactly. Do not pick newer versions.
- Python tests use only the standard library plus pytest. Read workflows as text; no PyYAML. Paths are relative to the repo root, as in `tests/test_automerge_bot_hold.py`.
- Every new test module asserts its extraction matched something before comparing, so an empty match cannot pass.
- A test that runs git or make in a fixture sets `HOME` to `tmp_path` and strips `GIT_DIR`, `GIT_WORK_TREE`, `GIT_COMMON_DIR`, `GIT_INDEX_FILE` from the child environment.
- `.github/workflows/*` edits keep each file's existing indentation and comment style.
- `make test` takes several minutes, so task gates are scoped (`pytest` on the task's module, `ruff check tests/`, `ruff format --check tests/`). The orchestrator runs `make test` once before each PR push; the `pre-push` hook runs it again.

## Verification (session level)

- PR A: in the PR's CI, `Secret Scan` and `Snyk Security Scan` pass with no `continue-on-error` on either step. The Snyk log shows `Open issues: 0`, and both install steps print `OK` from `sha256sum -c`.
- PR B: `pytest tests/test_release_workflow.py` passes, covering accept, reject (`v1.2.3`, `1.2`, `1.2.3;id`, a version differing from `Cargo.toml`, an existing tag, a non-main ref) and the off-main previous-tag range. Locally, `git-cliff "$(git merge-base v0.14.0 HEAD)..HEAD" --tag v0.15.0 --strip all` lists only post-v0.14.0 commits (measured 2026-10-27: 64 commits; `--unreleased` instead reaches back to v0.10.4).
- PR C: CI `Test`, `Cargo Audit`, `Docs Build` and `Semver Check` pass, and their logs show install-action installing the pinned versions.
- PR D: `make -n build` and `make -n build-linux` print no `Downloads`; `make -n install XDG_BIN_HOME=/x` prints an `install -m 755 target/release/etch` line naming `/x/etch`.
- Each new test module fails on `origin/main` as it stood before its PR (recorded per task).

---

### Task 1: continue-on-error allow-list, unmask both scans, exclude Snyk false positives

```yaml-task
id: 1
requirements: [R1, R2, R3]
description: Add the continue-on-error allow-list test, remove the gitleaks and Snyk masks, and add a .snyk exclude for the two false-positive files
role: executor
model: sonnet
tdd: required
acceptance:
  - cmd: pytest tests/test_continue_on_error.py -v
    exit_code: 0
  - cmd: ruff check tests/
    exit_code: 0
  - cmd: ruff format --check tests/
    exit_code: 0
max_retries: 3
files_touched: [tests/test_continue_on_error.py, .github/workflows/ci.yml, .snyk]
depends_on: []
```

**Test (`tests/test_continue_on_error.py`):** scan every `.github/workflows/*.yml`. A _site_ is a line matching `^\s*continue-on-error:\s*(?!false\s*$)\S`. Key each site by `(file name, nearest preceding name: value)`, using the regex `^\s*(-\s+)?name:\s*(.+?)\s*$` and stripping quotes; `None` when no `name:` precedes it. `ALLOWED` maps key to a non-empty reason:

- `("ci.yml", "Coverage with network tests (informational)")`: network-dependent tests are informational; the main coverage step gates.
- `("ci.yml", "Semver Check")`: job-level and kept out of `auto-merge` `needs:`; every new action adds an enum variant.
- `("mutation-testing.yml", "Run mutants")`: the `Score gate` step (`if: always()`) scores the result.
- `("release.yml", "Check semver compatibility")`: advisory on release, as in `semver-check`.
- `("release.yml", "Commit CHANGELOG")`: removed by PR B (Task 4).

Tests: every site's key is in `ALLOWED`; every `ALLOWED` key occurs at least once; there is at least one site; the scanner finds an unnamed site in an inline sample and keys it `None`; every reason is non-empty.

**RED:** on the base tree it fails on `("ci.yml", "Run gitleaks")` and `("ci.yml", "Run Snyk code scan")`.

**GREEN:** delete `continue-on-error: true` from those two steps in `ci.yml`. Create `.snyk`:

```yaml
# Snyk Code cannot ignore a single finding from the repo (only via the web UI),
# so these files are excluded whole. All three findings were LOW false positives
# (2026-10-27): the test feeds its own fixed env to a bash run of ci.yml's hold
# step and opens a constant path; test_metrics.py parses our own JUnit report on
# Python 3.13, where the XXE finding does not apply. See the 2026-10-27 spec.
version: v1.25.0
exclude:
    global:
        - tests/test_automerge_bot_hold.py
        - scripts/test_metrics.py
```

**Interfaces:** Produces `ALLOWED` (dict) in `tests/test_continue_on_error.py`; Task 4 removes its `Commit CHANGELOG` entry.

### Task 2: verify every direct download

```yaml-task
id: 2
requirements: [R4]
description: Add a test that every workflow download is version-pinned and sha256-verified, and pin and checksum gitleaks, Snyk and mdbook
role: executor
model: sonnet
tdd: required
acceptance:
  - cmd: pytest tests/test_workflow_downloads.py -v
    exit_code: 0
  - cmd: ruff check tests/
    exit_code: 0
  - cmd: ruff format --check tests/
    exit_code: 0
max_retries: 3
files_touched: [tests/test_workflow_downloads.py, .github/workflows/ci.yml]
depends_on: [1]
```

**Test (`tests/test_workflow_downloads.py`):** split each workflow into `run:` blocks (the `run: |` line plus following lines indented deeper than it, or a one-line `run:`). For each block containing `curl` or `wget` with an `https://` URL: the URL must not contain `/latest/`; the download must go to a file (`-o <path>`, `-O <path>` or `-qO <path>`), never a pipe; and the same block must contain `sha256sum -c` on a line naming that same path, before any line that uses the file. Tests: the rules over the real workflows; at least two download blocks found (shellcheck in `ci.yml` and `release.yml` already comply); inline-sample negative controls for a missing checksum, a checksum of a different path, a pipe into `tar`, and a `/latest/` URL.

**RED:** fails on gitleaks (no checksum), Snyk (`latest`, no checksum) and mdbook (pipe, no checksum).

**GREEN:** follow the shellcheck step's existing shape (`env:` with `*_VER` and `*_SHA256`, `curl -fsSL -o`, `echo "${SHA}  <path>" | sha256sum -c -`, then extract or install). Values from the spec: gitleaks 8.30.1, Snyk `v1.1308.0` from `https://static.snyk.io/cli/v1.1308.0/snyk-linux`, mdbook 0.5.3 into `/tmp/mdbook.tar.gz` then `tar -xzf ... -C /usr/local/bin` (use `sudo` if the existing step needs it). After install, check the version (`gitleaks version`, `snyk --version`, `mdbook --version`) contains the pin.

### Task 3: PR A documentation

```yaml-task
id: 3
description: Docs-only update for PR A (CLAUDE.md CI table and Backlog rows); no behaviour change, so TDD does not apply
role: executor
model: sonnet
tdd: not-applicable
acceptance:
  - cmd: '! grep -q "Snyk and gitleaks pass while failing" docs/superpowers/README.md'
    exit_code: 0
  - cmd: '! grep -q "CI downloads binaries with no checksum" docs/superpowers/README.md'
    exit_code: 0
  - cmd: 'grep -q "ci-release-hardening" docs/superpowers/README.md'
    exit_code: 0
max_retries: 2
files_touched: [CLAUDE.md, docs/superpowers/README.md]
depends_on: [2]
```

- `CLAUDE.md` CI table: `secret-scan` reads "gitleaks v8.30.1, pinned and checksum-verified. **Blocking**"; `snyk-scan` reads "Snyk CLI v1.1308.0, pinned and checksum-verified; `.snyk` excludes two false-positive files. **Blocking**". Add one sentence under the table: a `continue-on-error` needs an entry in `tests/test_continue_on_error.py`'s `ALLOWED`.
- `docs/superpowers/README.md`: delete the P1 "Snyk and gitleaks pass while failing" row and the P2 "CI downloads binaries with no checksum" row; add the plan to "All Plans" (date 2026-10-27, plan and spec links, status In Progress).

### Task 4: release workflow validation, notes range, no branch push

```yaml-task
id: 4
requirements: [R5, R6]
description: Validate the release version, compute the notes range from the merge-base of the previous tag, and remove the CHANGELOG generate and commit steps
role: executor
model: sonnet
tdd: required
acceptance:
  - cmd: pytest tests/test_release_workflow.py tests/test_continue_on_error.py -v
    exit_code: 0
  - cmd: ruff check tests/
    exit_code: 0
  - cmd: ruff format --check tests/
    exit_code: 0
max_retries: 3
files_touched: [.github/workflows/release.yml, tests/test_release_workflow.py, tests/test_continue_on_error.py]
depends_on: [3]
```

**Workflow:** directly after `actions/checkout`, add step `id: validate`, `name: Validate version`, whose script (reading `VERSION` and `GITHUB_REF` from env) exits 1 with a message unless all hold: `[[ "${VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]`; `GITHUB_REF` is `refs/heads/main`; the first `^version = "..."` in `app/Cargo.toml` and in `lib/Cargo.toml` both equal `VERSION`; `git rev-parse -q --verify "refs/tags/v${VERSION}"` fails. Replace both git-cliff steps and the `Commit CHANGELOG` step with: step `id: range`, `name: Release notes range`, setting `PREV` to `git tag --sort=-version:refname | grep '^v' | head -1` and writing `base=$(git merge-base "${PREV}" HEAD)` to `GITHUB_OUTPUT`, or nothing when there is no tag; then one git-cliff step with the same pinned action SHA, `args: ${{ steps.range.outputs.base && format('{0}..HEAD', steps.range.outputs.base) || '' }} --tag v${{ env.VERSION }} --strip all`, `OUTPUT: /tmp/release-notes.md`.

**Test (`tests/test_release_workflow.py`):** extract the `validate` and `range` steps' `run:` text by `id:` (reuse the approach of `extract_hold_run` in `tests/test_automerge_bot_hold.py`) and run each with `bash -e -c` in a fixture git repo under `tmp_path`, with `app/Cargo.toml` and `lib/Cargo.toml` at `version = "1.2.3"`. Validate cases: `1.2.3` on `refs/heads/main` exits 0; `v1.2.3`, `1.2`, `1.2.3;id`, `1.2.4`, an existing `v1.2.3` tag, and `refs/heads/feature` each exit non-zero. Range cases: previous tag on a side commit whose parent is main gives base equal to that parent; a tag on main gives the tagged commit; no tags writes no `base`. Static: `release.yml` contains no `--latest`, no `CHANGELOG`, and no `git push` other than `git push origin "v${VERSION}"`; the `validate` step precedes every other step that reads `VERSION`. Remove `("release.yml", "Commit CHANGELOG")` from `ALLOWED`.

**RED:** fails on the base tree (no `validate` or `range` step; `--latest` present).

### Task 5: retire CHANGELOG.md, ADR, PR B docs

```yaml-task
id: 5
requirements: [R7]
description: Docs and file retirement for PR B (ADR-0017, ADR-0009 status, CHANGELOG.md and make changelog removal, Backlog); no runtime behaviour, so TDD does not apply
role: executor
model: sonnet
tdd: not-applicable
acceptance:
  - cmd: test ! -e CHANGELOG.md
    exit_code: 0
  - cmd: '! grep -qE "^changelog:" Makefile'
    exit_code: 0
  - cmd: test -f docs/adr/0017-release-notes-in-release-body.md
    exit_code: 0
  - cmd: '! grep -q "Release workflow produces a wrong release" docs/superpowers/README.md'
    exit_code: 0
max_retries: 2
files_touched: [CHANGELOG.md, Makefile, README.md, docs/adr/0017-release-notes-in-release-body.md, docs/adr/0009-release-strategy-git-cliff-cosign-sha256.md, docs/adr/README.md, docs/superpowers/README.md]
depends_on: [4]
```

- `git rm CHANGELOG.md`. Delete the `changelog` target and its `.PHONY` entry from `Makefile`. In `README.md`, change the git-cliff prerequisite line so it no longer names `make changelog` (or delete it if nothing else uses git-cliff locally).
- ADR-0017, following the shape of the existing ADRs: Context (the rejected CHANGELOG push, seven off-`main` tags, wrong-release notes); Decision (notes in the release body only, range from the merge-base of the previous tag, version validated); Consequences (no committed changelog; the off-`main` tags stay, see Backlog). Set ADR-0009's status to note it is amended by ADR-0017 for the changelog clause; add the ADR-0017 row to `docs/adr/README.md`.
- Backlog: delete the P2 release row. Rewrite the P4 "Release tags off `main`" row: `v0.10.5`-`v0.14.0` (seven tags, checked 2026-10-27 with `git merge-base --is-ancestor`), not `v0.12.0`-`v0.14.0`; release notes no longer depend on them.

### Task 6: pinned tool installs (PR C)

```yaml-task
id: 6
requirements: [R8]
description: Replace every cargo install in workflows with a SHA-pinned taiki-e/install-action at pinned tool versions, enforced by a test
role: executor
model: sonnet
tdd: required
acceptance:
  - cmd: pytest tests/test_tool_installs.py -v
    exit_code: 0
  - cmd: ruff check tests/
    exit_code: 0
  - cmd: ruff format --check tests/
    exit_code: 0
max_retries: 3
files_touched: [tests/test_tool_installs.py, .github/workflows/ci.yml, .github/workflows/release.yml, .github/workflows/cargo-audit-scheduled.yml, .github/workflows/mutation-pr.yml, .github/workflows/mutation-testing.yml, docs/superpowers/README.md]
depends_on: [5]
```

**Test:** across all workflows: no line matches `cargo install`; at least one `uses: taiki-e/install-action@` exists; each is `@<40 hex>` followed by `# v2.87.28`; the following `with:` block has `fallback: none` and a `tool:` value where every comma-separated item matches `^[a-z0-9-]+@\d+\.\d+\.\d+$`; one tool resolves to one version repo-wide. Inline negative controls for a tag ref, a bare tool name and a missing `fallback`.

**RED:** 11 `cargo install` lines on the base tree (`ci.yml` 5, `release.yml` 3, `mutation-pr.yml` 1, `mutation-testing.yml` 1, `cargo-audit-scheduled.yml` 1).

**GREEN:** replace each install step with `uses: taiki-e/install-action@1168f6810a3cb30a3514bf5756a3b7f0422c6df8  # v2.87.28` plus `with: { tool: <name>@<pin>, fallback: none }`, in block YAML style. Where consecutive install steps sit in one job, merge them into one step with a comma-separated `tool:`. Delete the Backlog P3 row "Replace `cargo install` with a pinned `taiki-e/install-action`".

### Task 7: build stays in the repo, add make install (PR D)

```yaml-task
id: 7
requirements: [R9]
description: Remove the ~/Downloads copies from make build and build-linux and add make install to XDG_BIN_HOME, enforced by a make -n test
role: executor
model: sonnet
tdd: required
acceptance:
  - cmd: pytest tests/test_makefile.py -v
    exit_code: 0
  - cmd: ruff check tests/
    exit_code: 0
  - cmd: ruff format --check tests/
    exit_code: 0
max_retries: 3
files_touched: [Makefile, tests/test_makefile.py, CLAUDE.md, smoke-tests/README.md, docs/superpowers/README.md]
depends_on: [6]
```

**Test (`tests/test_makefile.py`):** run `make -n <target>` from the repo root with `HOME=tmp_path`. `build` and `build-linux` output contains no `Downloads` and contains `cargo build --release` and `cargo zigbuild` respectively. `install XDG_BIN_HOME=/x` output has a line starting `install -m 755 target/release/etch` that names `/x/etch`, after the `cargo build --release` line. With `XDG_BIN_HOME` unset, output contains `<tmp_path>/.local/bin/etch` and a `mkdir -p` of that directory.

**GREEN:** delete both `cp` lines. Add `install` to `.PHONY` and:

```make
INSTALL_DIR ?= $(or $(XDG_BIN_HOME),$(HOME)/.local/bin)

install: build
	mkdir -p "$(INSTALL_DIR)"
	install -m 755 target/release/etch "$(INSTALL_DIR)/etch"
```

Update `CLAUDE.md` Quick Reference (`make build-linux` line ends at the target path; add `make install`), and `smoke-tests/README.md` lines 12-13 to copy from `target/x86_64-unknown-linux-gnu/release/etch`. Delete the Backlog P3 "`make build` writes to `~/Downloads`" row. Mark the plan Done in "All Plans".

## Non-goal check

Fresh reviewer, 2026-10-27. All three CLEAR; no plan revision or amendment needed.

- N1 CLEAR: Task 5 rewrites the P4 Backlog row's text only; no step re-points or deletes a tag.
- N2 CLEAR: the only new SHA pin is `taiki-e/install-action` (Task 6); Task 4's git-cliff step reuses the existing pinned SHA.
- N3 CLEAR: `semver-check` keeps `continue-on-error` via Task 1's `ALLOWED` and stays out of `auto-merge` `needs:`.
