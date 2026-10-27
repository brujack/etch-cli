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

## Amendments

- finding R5 (2026-10-27, reviewer both): NOT-BUILT — Looked for changes to release.yml that validate `version` as MAJOR.MINOR.PATCH against both Cargo.toml files, check for an existing tag and the refs/heads/main ref, ahead of other steps. release.yml does not appear in the diff.
- R5 reviewed: not in scope for PR A (plan Tasks 1-3, R1-R4); delivered by PR B (plan Task 4) of the same plan — the spec delivers in four ordered PRs
- finding R6 (2026-10-27, reviewer both): NOT-BUILT — release.yml is unchanged in the diff. The 'Commit CHANGELOG' push step is still present (ALLOWED keeps it, 'removed by PR B'), and no notes range using `git merge-base <previous tag> HEAD` appears.
- R6 reviewed: not in scope for PR A (plan Tasks 1-3, R1-R4); delivered by PR B (plan Task 4) of the same plan — the spec delivers in four ordered PRs
- finding R7 (2026-10-27, reviewer both): NOT-BUILT — The diff neither deletes CHANGELOG.md nor adds an ADR reversing ADR-0009's committed-changelog clause.
- R7 reviewed: not in scope for PR A (plan Tasks 1-3, R1-R4); delivered by PR B (plan Task 5) of the same plan — the spec delivers in four ordered PRs
- finding R8 (2026-10-27, reviewer both): NOT-BUILT — The diff has no taiki-e/install-action step, no removal of `cargo install`, and no test enforcing `tool: <name>@<X.Y.Z>` / `fallback: none`.
- R8 reviewed: not in scope for PR A (plan Tasks 1-3, R1-R4); delivered by PR C (plan Task 6) of the same plan — the spec delivers in four ordered PRs
- finding R9 (2026-10-27, reviewer both): NOT-BUILT — The Makefile is not in the diff, so there is no change to build or build-linux output locations and no `make install` target writing to ${XDG_BIN_HOME:-$HOME/.local/bin}/etch.
- R9 reviewed: not in scope for PR A (plan Tasks 1-3, R1-R4); delivered by PR D (plan Task 7) of the same plan — the spec delivers in four ordered PRs
- finding R1 (2026-10-27, reviewer both): NOT-BUILT — The diff only removes the stale ("workflows/release.yml", "Commit CHANGELOG") entry from ALLOWED in tests/test_continue_on_error.py (and the matching continue-on-error site in release.yml); the test's logic that fails on unlisted and on vanished listed sites is not in the diff, so there is no evidence here that it does either.
- R1 reviewed: delivered by PR A (#148, merged to main before this branch; tests/test_continue_on_error.py), so it is in the base, not this diff — the spec delivers in four ordered PRs
- finding R2 (2026-10-27, reviewer both): NOT-BUILT — No change to the gitleaks or Snyk steps (no ci.yml hunk) appears in the diff.
- R2 reviewed: delivered by PR A (#148, merged); its CI run showed gitleaks and Snyk unmasked and passing — the spec delivers in four ordered PRs
- finding R3 (2026-10-27, reviewer both): NOT-BUILT — Looked for a .snyk file with an exclude naming tests/test_automerge_bot_hold.py and scripts/test_metrics.py; none is in the diff.
- R3 reviewed: delivered by PR A (#148, merged; .snyk) — the spec delivers in four ordered PRs
- finding R4 (2026-10-27, reviewer both): NOT-BUILT — No workflow download, sha256sum -c line, or test enforcing download pinning/verification appears in the diff.
- R4 reviewed: delivered by PR A (#148, merged; tests/test_workflow_downloads.py and the pinned ci.yml steps) — the spec delivers in four ordered PRs
- finding R8 (2026-10-27, reviewer both): NOT-BUILT — Looked for removal of cargo install, taiki-e/install-action pinned by SHA with tool: <name>@<X.Y.Z> and fallback: none, and an enforcing test; none is in the diff.
- R8 reviewed: not in scope for PR B (plan Tasks 4-5, R5-R7); delivered by PR C (plan Task 6) — the spec delivers in four ordered PRs
- finding R9 (2026-10-27, reviewer both): NOT-BUILT — The Makefile hunks only drop the changelog target; build/build-linux output locations and a make install target copying to ${XDG_BIN_HOME:-$HOME/.local/bin}/etch are not in the diff.
- R9 reviewed: not in scope for PR B; delivered by PR D (plan Task 7) — the spec delivers in four ordered PRs

## Spec alignment (2026-10-27)

- spec: docs/superpowers/specs/2026-10-27-ci-release-hardening-design.md
- anchor: 7aed05129a66996511e6f4e5b7cb1b0e075e03d8
- in scope: R1, R2, R3, R4, R5, R6, R7, R8, R9
- out of scope: none

### Findings

| ID | Reviewer | Verdict | Reason | Amendment |
| --- | --- | --- | --- | --- |
| R1 | both | NOT-BUILT | The diff only removes the stale ("workflows/release.yml", "Commit CHANGELOG") entry from ALLOWED in tests/test_continue_on_error.py (and the matching continue-on-error site in release.yml); the test's logic that fails on unlisted and on vanished listed sites is not in the diff, so there is no evidence here that it does either. | - R1 reviewed: delivered by PR A (#148, merged to main before this branch; tests/test_continue_on_error.py), so it is in the base, not this diff — the spec delivers in four ordered PRs |
| R2 | both | NOT-BUILT | No change to the gitleaks or Snyk steps (no ci.yml hunk) appears in the diff. | - R2 reviewed: delivered by PR A (#148, merged); its CI run showed gitleaks and Snyk unmasked and passing — the spec delivers in four ordered PRs |
| R3 | both | NOT-BUILT | Looked for a .snyk file with an exclude naming tests/test_automerge_bot_hold.py and scripts/test_metrics.py; none is in the diff. | - R3 reviewed: delivered by PR A (#148, merged; .snyk) — the spec delivers in four ordered PRs |
| R4 | both | NOT-BUILT | No workflow download, sha256sum -c line, or test enforcing download pinning/verification appears in the diff. | - R4 reviewed: delivered by PR A (#148, merged; tests/test_workflow_downloads.py and the pinned ci.yml steps) — the spec delivers in four ordered PRs |
| R8 | both | NOT-BUILT | Looked for removal of cargo install, taiki-e/install-action pinned by SHA with tool: <name>@<X.Y.Z> and fallback: none, and an enforcing test; none is in the diff. | - R8 reviewed: not in scope for PR B (plan Tasks 4-5, R5-R7); delivered by PR C (plan Task 6) — the spec delivers in four ordered PRs |
| R9 | both | NOT-BUILT | The Makefile hunks only drop the changelog target; build/build-linux output locations and a make install target copying to ${XDG_BIN_HOME:-$HOME/.local/bin}/etch are not in the diff. | - R9 reviewed: not in scope for PR B; delivered by PR D (plan Task 7) — the spec delivers in four ordered PRs |

### Reviewed

- R5 reviewed: not in scope for PR A (plan Tasks 1-3, R1-R4); delivered by PR B (plan Task 4) of the same plan — the spec delivers in four ordered PRs
- R6 reviewed: not in scope for PR A (plan Tasks 1-3, R1-R4); delivered by PR B (plan Task 4) of the same plan — the spec delivers in four ordered PRs
- R7 reviewed: not in scope for PR A (plan Tasks 1-3, R1-R4); delivered by PR B (plan Task 5) of the same plan — the spec delivers in four ordered PRs
- R8 reviewed: not in scope for PR A (plan Tasks 1-3, R1-R4); delivered by PR C (plan Task 6) of the same plan — the spec delivers in four ordered PRs
- R9 reviewed: not in scope for PR A (plan Tasks 1-3, R1-R4); delivered by PR D (plan Task 7) of the same plan — the spec delivers in four ordered PRs
- R1 reviewed: delivered by PR A (#148, merged to main before this branch; tests/test_continue_on_error.py), so it is in the base, not this diff — the spec delivers in four ordered PRs
- R2 reviewed: delivered by PR A (#148, merged); its CI run showed gitleaks and Snyk unmasked and passing — the spec delivers in four ordered PRs
- R3 reviewed: delivered by PR A (#148, merged; .snyk) — the spec delivers in four ordered PRs
- R4 reviewed: delivered by PR A (#148, merged; tests/test_workflow_downloads.py and the pinned ci.yml steps) — the spec delivers in four ordered PRs
- R8 reviewed: not in scope for PR B (plan Tasks 4-5, R5-R7); delivered by PR C (plan Task 6) — the spec delivers in four ordered PRs
- R9 reviewed: not in scope for PR B; delivered by PR D (plan Task 7) — the spec delivers in four ordered PRs

### Verifications

- none
