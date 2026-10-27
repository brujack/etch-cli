# ADR-0017: Release notes in the release body only

**Date:** 2026-10-27
**Status:** Accepted

## Context

ADR-0009 had the release workflow commit a regenerated `CHANGELOG.md` to the default branch on every release. That stopped working when `main` became protected:

1. **The push was rejected and nobody saw it.** The `Commit CHANGELOG` step pushed straight to `main`, branch protection rejected it, and the step's `continue-on-error: true` turned the failure green. The last CHANGELOG commit that reached `main` is from v0.10.4.
2. **Tags landed off `main`.** The tag step ran after the unpushed CHANGELOG commit, so it tagged that commit. `v0.10.5` through `v0.14.0` (seven tags, checked 2026-10-27 with `git merge-base --is-ancestor`) all point at `chore(changelog)` commits that are not in `main`'s history.
3. **Each release described the previous one.** The notes step used `git-cliff --latest` before the new tag existed, so it rendered the last tagged range: the v0.14.0 notes list `install cargo-machete`, which shipped in v0.13.0.
4. **The version was never checked.** `inputs.version` went straight into the tag, the release name and `release-sign`, and nothing compared it with `Cargo.toml`.

Pushing the CHANGELOG through a pull request instead would need the release to open a PR, wait for CI and auto-merge, and only then tag. That turns a one-job release into a multi-hour, multi-actor sequence to maintain a file whose content the GitHub release already carries.

## Decision

- **Release notes live in the GitHub release body only.** `CHANGELOG.md` and the `make changelog` target are deleted. The workflow pushes nothing but the tag.
- **The notes range is explicit.** A `range` step sets the base to `git merge-base <previous tag> HEAD`, and git-cliff renders `<base>..HEAD` under `--tag v<version>`. The merge-base is needed because the previous tags are off `main`: `--unreleased` would treat everything since v0.10.4, the last tag on `main`, as unreleased.
- **The version is validated first.** A `validate` step, before any other step reads `VERSION`, requires `MAJOR.MINOR.PATCH`, a dispatch from `refs/heads/main`, a match with both `app/Cargo.toml` and `lib/Cargo.toml`, and no existing `v<version>` tag.

The cosign and SHA-256 parts of ADR-0009 are unchanged.

## Consequences

- There is no changelog file in the repository. The history of changes is the list of GitHub releases, and `git-cliff` can still render any range locally.
- New tags are created on the dispatched `main` commit, so they are in `main`'s history.
- The seven existing off-`main` tags stay where they are. Re-pointing published tags rewrites history and is the operator's decision (Backlog P4). The merge-base range means release notes no longer depend on them.
- `tests/test_release_workflow.py` runs the `validate` and `range` scripts against fixture repositories and fails if the workflow regains `--latest`, a `CHANGELOG` step or a branch push.

## Related

- ADR-0009 (amended by this ADR for its changelog clause)
- `docs/superpowers/specs/2026-10-27-ci-release-hardening-design.md` (R5-R7)
- math#159, the same defect found in the math repo
