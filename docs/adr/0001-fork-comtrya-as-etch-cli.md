# ADR-0001: Fork comtrya as etch-cli

**Date:** 2026-05-02
**Status:** Accepted

## Context

[comtrya](https://github.com/comtrya/comtrya) is a declarative, manifest-driven configuration management tool for personal workstations. It was archived by its maintainers in April 2026. A fork was needed to continue development for managing personal machines (Mac Studio M1 Ultra and a Linux AMD Ryzen workstation).

Options considered: contribute upstream (project archived, no upstream), adopt a different tool (Ansible is too heavy for single-host workstation management, Nix is too steep a learning curve), fork and maintain independently.

## Decision

Fork comtrya at the archive point, rename the binary to `etch` and the crate to `etch-cli`. Maintain the existing YAML manifest format and action model. Add features required for dotfiles migration (brew.bundle, mas.install, cask installs, file permissions, tilde expansion in manifest_paths, privileged file actions).

The fork is personal infrastructure — no intent to publish to crates.io or maintain a public community.

## Consequences

- Full ownership of the codebase — can add features, remove dead code, prune unsupported platforms without coordinating with upstream.
- Must maintain CI, dependency updates, and security patches independently.
- Existing comtrya manifests remain compatible (same YAML format, same action names).
- No upstream to pull security patches from — `cargo audit` and Snyk scanning are the backstop.

## Related

- [Phase 1 plan](../superpowers/plans/2026-05-02-etch-cli-phase1.md)
- `deny.toml` documents the 3 unfixable advisories inherited from the fork
