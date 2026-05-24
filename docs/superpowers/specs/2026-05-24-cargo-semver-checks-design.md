# cargo semver-checks Design

## Goal

Detect unintentional breaking API changes in `etch-lib` before they ship. Advisory on PRs (earliest signal), blocking at release time (hard gate).

## Background

`etch-lib` is a library crate (v0.9.3) consumed by `etch-cli` and potentially external users. Breaking API changes without a major version bump violate semver. The current pipeline has no mechanism to detect this — a renamed `pub` function or removed trait impl would silently ship.

`cargo-semver-checks` statically compares two versions of a crate's public API via rustdoc JSON. It catches: removed/renamed `pub` items, changed function signatures, removed trait impls, changed type layouts. It does not catch behavior changes (tests and mutation testing cover that).

## Scope

`etch-lib` (`lib/`) only.

- `jsonschemagen` is v0.1.0 — pre-release by semver convention; breaking changes are allowed and not worth gating.
- `app` is a binary crate with no public API.

## Two Integration Points

### 1. CI Job — Advisory

**File:** `.github/workflows/ci.yml`

New `semver-check` job. Runs in parallel with existing jobs. `continue-on-error: true` — never blocks auto-merge; findings appear as a warning in the CI check list.

**Baseline:** `origin/main`. Answers "did this PR introduce a breaking change relative to what's on main?"

Requires one extra fetch step after checkout (default checkout only fetches the PR branch):

```yaml
- name: Fetch main for baseline
  run: git fetch origin main --depth=1
```

Then:

```yaml
- name: Install cargo-semver-checks
  run: cargo install cargo-semver-checks --locked

- name: Check semver compatibility
  run: cargo semver-checks check-release -p etch-lib --baseline-rev origin/main
```

Job is NOT added to the `needs` list of the `auto-merge` job.

### 2. Release Workflow Step — Blocking

**File:** `.github/workflows/release.yml`

New step inserted after `make test` passes and before `cargo build --release`. No `continue-on-error` — a violation stops the release.

**Baseline:** previous git tag. The release workflow already uses `fetch-depth: 0` so full tag history is available.

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

Gracefully skips on first-ever release (no previous `v*` tag).

### 3. Makefile Target

```makefile
semver:
	cargo semver-checks check-release -p etch-lib --baseline-rev origin/main
```

Local developer check — same logic as the CI job. Added alongside `mutants` and `bench`.

## Installation

`cargo install cargo-semver-checks --locked` — consistent with the existing pattern for nextest, tarpaulin, and cargo-deny. No new GitHub Action dependency.

## What It Catches

- Removed or renamed `pub` functions, types, traits, constants
- Changed function signatures (parameter types, return types)
- Removed trait implementations
- Changed type layouts (`pub` struct fields removed or reordered)
- Sealed traits becoming unsealed or vice versa

## What It Does Not Catch

- Behavior changes (covered by tests and mutation testing)
- Performance regressions (covered by Criterion benchmarks)
- Dependency version changes

## Behaviour Change Requiring Major Bump

If a PR intentionally introduces a breaking API change (e.g. renaming a function for clarity before v1.0), the developer must bump the major version in `lib/Cargo.toml` before the CI advisory check will pass. The release workflow gate enforces this — the release cannot proceed until the version reflects the break.

## Files Modified

| File                            | Change                                         |
| ------------------------------- | ---------------------------------------------- |
| `.github/workflows/ci.yml`      | Add `semver-check` job (advisory)              |
| `.github/workflows/release.yml` | Add semver-check step (blocking, before build) |
| `Makefile`                      | Add `semver` target                            |
| `docs/superpowers/README.md`    | Update backlog → In Progress                   |
