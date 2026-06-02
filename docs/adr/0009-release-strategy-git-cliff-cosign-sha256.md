# ADR-0009: Release strategy — git-cliff changelog, cosign v4, SHA256

**Date:** 2026-05-23
**Status:** Accepted

## Context

Before PR #58, the release workflow had three problems:

1. **Release notes** were generated with inline `git log` commands in the workflow YAML. The format was inconsistent — delimiter handling was fragile and required manual cleanup after each release.

2. **cosign signing** was using the v1 flag format (`--output-signature`/`--output-certificate`). cosign v4 changed to a bundle format (`.bundle`) — the old flags were silently deprecated and the `.sig`/`.pem` pair was not being produced. Consumers attempting to verify binaries were getting `file not found` errors.

3. **No SHA256 checksums** were published with releases, making integrity verification dependent on TLS alone.

Alternatives considered for changelogs: manual release notes (too slow), `github-changelog-generator` (Ruby dependency, inconsistent format), `conventional-changelog` (Node.js dependency). git-cliff is a single Rust binary, integrates directly with Conventional Commits, and has a maintained GitHub Action.

## Decision

Three changes shipped together in PR #58:

- **Changelogs:** `orhun/git-cliff-action@v4` (pinned SHA) generates `CHANGELOG.md` committed to master on every release. Release notes are extracted from the latest tag range and set as the GitHub release body — no manual editing.

- **Signing:** cosign v4 keyless signing via `sigstore/cosign-installer` (pinned to a specific tag — floating major tags do not exist for this action). Sign command: `cosign sign-blob --yes --bundle <name>.bundle`. Replaces the deprecated `.sig`/`.pem` pair. Verification uses `cosign verify-blob --bundle <name>.bundle --certificate-identity <workflow-ref> --certificate-oidc-issuer https://token.actions.githubusercontent.com`.

- **Checksums:** `sha256sum` (Linux) / `shasum -a 256` (macOS) generates `<binary>.sha256` for each platform artifact, uploaded to the release alongside the binary and `.bundle`.

The signing step is extracted into a separate reusable workflow (`release-sign.yml`) to keep the main release workflow focused on build and artifact upload.

## Consequences

- `CHANGELOG.md` is committed to master on every release — it is a living artifact, not a generated-only file.
- Consumers of etch-cli binaries must update their verification commands from `--output-signature`/`--output-certificate` to `--bundle`.
- `sigstore/cosign-installer` must be pinned to a specific version tag (e.g. `@v4.1.2`) — floating major tags fail with "unable to find version".
- SHA256 checksums enable integrity verification without trusting TLS alone, and are compatible with standard package manager tooling that validates checksums.
- The reusable `release-sign.yml` pattern allows the signing step to be audited and updated independently of the build workflow.

## Related

- PR #58 (release strategy overhaul)
- `release-sign.yml` — reusable signing workflow
- dotfiles `docs/adr/` — cosign v4 bundle format is documented as a cross-cutting pitfall
