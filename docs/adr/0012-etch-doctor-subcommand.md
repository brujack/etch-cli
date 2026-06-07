# ADR-0012: etch doctor subcommand

**Date:** 2026-06-06
**Status:** Accepted

## Context

`etch status` reports manifest drift — whether the system matches what the manifests declare. It does not check system-level health invariants that manifests cannot express: are dotfile symlinks intact, are required tools in PATH, do credential directories have secure permissions, are pinned binary versions installed?

`setup_env.sh` had a `doctor` mode that checked some of these but it was not integrated into etch-cli. Adding the checks as a subcommand keeps all machine health in one tool.

Alternatives considered: extend `etch status` to include system checks (rejected — blurs the boundary between manifest drift and system health; status runs against manifests and has a different exit-code semantic), add checks as manifest actions (rejected — doctor runs checks without modifying the system; actions are designed to modify).

## Decision

Add `etch doctor` subcommand with four check categories:

- **Symlinks** — for each `file.link` action in loaded manifests, verify the target path exists and resolves (follows symlinks, detects dangling links)
- **Tools** — verify tools exist in PATH; inferred from manifest action types (e.g. `gem.install` implies `gem`) plus explicit `doctor.tools:` config
- **Credential dirs** — verify listed directories have mode 700; configured via `doctor.credential_dirs:` in `etch.yaml`; nonexistent dirs are skipped (machine may not have that credential type)
- **Versions** — for `binary.github`/`binary.url` atoms with a pinned version (not `latest`), run `<name> --version` and check output contains the version string; plus explicit `doctor.versions:` config pins

Exit code 0 = all checks pass; 1 = any check fails. Supports `--json` and `--missing-only`.

Security decision: binary names from manifest-controlled YAML are passed as `Command::new(binary_name)` (argv[0], no shell), not via `sh -c`, to prevent injection from hostile manifest `name:` fields. Explicit config pins (`command:` field in `doctor.versions:`) use `sh -c` because they are user-authored in `etch.yaml`.

Check implementations live in `lib/src/doctor/` (testable in isolation via a `DoctorCheck` trait); the command wiring lives in `app/src/commands/doctor.rs`, following the lib/app split established by `etch status`.

## Consequences

- System health validation is integrated into etch-cli, eliminating the separate `setup_env.sh doctor` mode.
- The `DoctorCheck` trait makes each check independently unit-testable without spawning the full binary.
- `execute()` on the Doctor command calls `std::process::exit(1)` on failure, which is excluded from tarpaulin coverage with `#[cfg(not(tarpaulin_include))]`, consistent with the existing pattern in `update.rs`.
- Version checks that run `<binary> --version` against installed binaries are structurally uncoverable in CI (binary not installed in the container), same ceiling as other external-tool checks.
