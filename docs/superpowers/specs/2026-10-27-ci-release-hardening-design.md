# CI and release hardening (math learnings A–D)

- **Approved:** 2026-10-27 by the operator ("yes" to the A–D split, with A using per-finding Snyk ignores or, failing that, path exclusions)
- **Source:** the math session's CI and release learnings (math#159-#169), triaged against etch-cli on 2026-10-24

## Problem

Four defects found by checking math's CI and release learnings against this repo:

1. **Scans that cannot fail.** `ci.yml` sets `continue-on-error: true` on the gitleaks step and the Snyk step, so the required `Secret Scan` and `Snyk Security Scan` checks read green whatever the tools report. `SNYK_TOKEN` was unset until 2026-10-27, so every Snyk run had failed authentication (`SNYK-0005`, exit 2) behind the mask.
2. **Unverified downloads.** gitleaks (`wget`), Snyk (`curl` of `cli/latest`) and mdbook (`curl | tar`) run with no `sha256sum -c`, against core rule ci-002.
3. **Wrong releases.** `release.yml` pushes a CHANGELOG commit to protected `main`. The push is rejected under `continue-on-error`, and the tag is created on the unpushed commit, so v0.10.5 to v0.14.0 (seven tags) all point at commits outside `main`. The notes use `--latest` before the new tag exists, so each release body describes the previous release. `inputs.version` reaches the tag unvalidated and is never compared with `Cargo.toml`.
4. **Unpinned tools and a stray copy.** Eleven `cargo install --locked` calls name no version. `make build` and `make build-linux` copy the binary into `~/Downloads`.

## Requirements

- **R1.** No workflow step or job carries `continue-on-error` unless a test lists it with a reason; the test fails on an unlisted site and on a listed site that no longer exists.
- **R2.** The gitleaks and Snyk steps run unmasked and pass on `main`.
- **R3.** The three current Snyk Code findings (all LOW, all false positives) are excluded from the scan. Snyk Code cannot ignore a single finding from a repository file (Snyk docs, "Ignore issues", checked 2026-10-27; per-finding ignores live only in the Snyk web UI or the early-access `snyk ignore create`), so the fallback applies: a `.snyk` `exclude` naming the two files, `tests/test_automerge_bot_hold.py` and `scripts/test_metrics.py`.
- **R4.** Every direct download in a workflow is pinned to a version and verified with `sha256sum -c` against the downloaded file before it runs; a test enforces this.
- **R5.** `release.yml` validates `version` as `MAJOR.MINOR.PATCH`, equal to both crates' `Cargo.toml` version, with no existing tag of that name, dispatched from `refs/heads/main`. The validation runs before any other step reads the version.
- **R6.** `release.yml` pushes nothing but the tag. Release notes cover exactly the commits between the previous release and `HEAD`, including while the previous tag is off `main` (range base is `git merge-base <previous tag> HEAD`).
- **R7.** `CHANGELOG.md` is retired; release notes live in the GitHub release body only. An ADR records the reversal of ADR-0009's committed-changelog clause.
- **R8.** No workflow runs `cargo install`. Rust tools install through `taiki-e/install-action` pinned by commit SHA, with `tool: <name>@<X.Y.Z>` and `fallback: none`; a test enforces this.
- **R9.** `make build` and `make build-linux` write only inside the repository. `make install` copies the release binary to `${XDG_BIN_HOME:-$HOME/.local/bin}/etch`.
- **N1.** Re-pointing or deleting the seven off-`main` tags. That rewrites published history and is the operator's decision (Backlog P4).
- **N2.** Pinning GitHub Actions by SHA beyond the one new action. Renovate's `pinDigests` owns that, once Renovate runs (Backlog P2).
- **N3.** Making `semver-check` blocking (Backlog P3).

## Delivery

Four PRs, merged in order: A (R1-R4), B (R5-R7), C (R8), D (R9). Each later PR edits a file an earlier one changes, so each branches from `main` after its predecessor merges.

Pinned values, each at least 14 days old on 2026-10-27 and checked against the publisher's digest:

| Item                                       | Pin                               | SHA-256 or commit                                                  |
| ------------------------------------------ | --------------------------------- | ------------------------------------------------------------------ |
| gitleaks linux x64                         | 8.30.1                            | `551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb` |
| Snyk CLI linux                             | v1.1308.0 (2026-10-08)            | `a916d3ec9e6c27d56cf67664459885512758a1ae94418ba430fcb92e86c32ae1` |
| mdbook linux gnu                           | 0.5.3                             | `e2fd508a4fac06cbaa9f88b97d27bdc3b55a08946304ca845879fe26a3699e11` |
| taiki-e/install-action                     | v2.87.28 (2026-10-12)             | `1168f6810a3cb30a3514bf5756a3b7f0422c6df8`                         |
| cargo-machete / nextest / tarpaulin / deny | 0.9.2 / 0.9.148 / 0.37.5 / 0.20.2 | install-action manifest                                            |
| cargo-semver-checks / audit / mutants      | 0.51.0 / 0.22.2 / 27.1.0          | install-action manifest                                            |
