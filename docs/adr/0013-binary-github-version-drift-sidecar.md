# ADR-0013: binary.github version drift detection via sidecar files

**Date:** 2026-06-09
**Status:** Accepted

## Context

`binary.github` downloads and installs pinned release binaries but had no way to
detect whether the installed binary matches the manifest pin or whether a newer
release is available. Without drift detection, `etch status` always reported
`Unchecked` for these actions regardless of what was actually installed.

Alternatives considered:

- **Parse `--version` output** — fragile; output format is not standardised across
  binaries and requires per-binary config to extract the version string.
- **Read file metadata** — file modification time and size carry no version signal.
- **Central version manifest** — a separate TOML or JSON file tracking all installed
  binaries would require a new persistence layer and out-of-band write path.

## Decision

Write a hidden sidecar file `{dir}/.{name}.version` alongside the binary on first
install. The sidecar contains the pinned tag string verbatim (e.g. `v1.5.0`). A
new `BinaryGitHubStatus` atom reads this file during `etch status`:

1. **Install mismatch** — sidecar content differs from the manifest pin (normalized,
   stripping any leading `v`) → `Drifted { expected: pin, actual: sidecar }`.
2. **Update available** — sidecar matches the pin but the latest GitHub release tag
   differs → `Drifted { expected: latest (latest), actual: pin (pinned) }`.
3. **Up to date** — sidecar matches pin and pin matches latest → `Ok`.
4. **No sidecar** — binary was installed before sidecar support or by other means;
   `plan()` writes the sidecar on the next `etch apply` run → `Unchecked` until
   then.

GitHub API calls in case 2 are cached for one hour at
`~/.cache/etch/github-versions/{owner}-{repo}.json` to avoid rate-limit
exhaustion on manifests with many pinned binaries.

Drift detection is only active when `version:` is a pinned tag. `version: latest`
and absent `version:` produce no status atom and skip sidecar writes.

## Consequences

- `etch status` can now report install mismatches and available updates for pinned
  GitHub binaries without network access on every run (cache TTL 1h).
- Sidecar files (`.{name}.version`) appear in install directories alongside the
  binary — operators should be aware of these hidden files.
- Binaries installed before this change (or outside etch) will show `Unchecked`
  until the next `etch apply` run writes the sidecar.
- Changing `version:` in the manifest does not cause `etch apply` to upgrade the
  binary — apply is idempotent and skips existing binaries. `etch status` will
  report the mismatch; the user must delete the binary to trigger a re-install.
- Every new binary-type action that supports version pinning should follow this
  sidecar pattern for consistency with `etch status` drift reporting.

## Related

- [ADR-0007](0007-etch-status-drift-detection-subcommand.md) — etch status subcommand architecture
- [binary-github-drift spec](../superpowers/specs/2026-06-09-binary-github-drift-design.md)
- [binary-github-drift plan](../superpowers/plans/2026-06-09-binary-github-drift-plan.md)
- PR #101 — implementation
