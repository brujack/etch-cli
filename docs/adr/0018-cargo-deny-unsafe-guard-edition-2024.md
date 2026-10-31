# ADR-0018: cargo-deny as the single advisory gate, an `unsafe` guard, and edition 2024

**Date:** 2026-10-31
**Status:** Accepted

## Context

The math repo's Rust standards work (math#187, math#188) prompted a re-measurement of etch-cli on `claude`:

1. **The `deny.toml` license, ban and source policy was not enforced anywhere.** The only cargo-deny call was `cargo deny check advisories`.
2. **Two advisory ignore lists had drifted.** `deny.toml` held 6 ignores and the scheduled `cargo-audit` workflow held 8, two of them stale.
3. **cargo-deny and cargo-audit did not report the same advisories.** cargo-deny's `unsound` default is `"workspace"`, which drops unsound advisories in transitive crates, so RUSTSEC-2026-0306 (faster-hex) and RUSTSEC-2026-0205 (scc) appeared only in cargo-audit. Retiring cargo-audit alone would have lost them.
4. **Nothing guarded new `unsafe`.**
5. **Edition 2024 was blocked by process-global env mutation, not syntax.** 159 `std::env::set_var`/`remove_var` calls (157 in tests, 2 in `command.run`'s env initializer and finalizer) become `unsafe` under 2024.
6. **`command.run`'s production env handling was defective.** It deleted inherited values instead of restoring them, leaked `env:` after a failed step, never passed `env:` to sudo, and printed values in `Debug`.

The full measurements are in `docs/cursor/specs/2026-10-31-rust-standards-edition-2024-spec.md`.

## Decision

- **cargo-deny is the only advisory tool.** The blocking `cargo-audit` job (job id kept as the required-check name) runs `cargo deny check advisories licenses bans sources`, and a weekly `cargo-deny-scheduled.yml` replaces `cargo-audit-scheduled.yml`. `deny.toml` sets `unmaintained = "all"` and `unsound = "all"` and holds the one ignore list. A test forbids any workflow from carrying `--ignore RUSTSEC`. The two unsound advisories were cleared by dependency bumps (`serial_test` 4, faster-hex 0.10.1), with no new ignore.
- **`unsafe_code = "deny"`** in `lib`, `app` and `jsonschemagen`. Each remaining `unsafe` block carries `#[expect(unsafe_code, reason = "...")]`; `#[allow(unsafe_code)]` is never used. `expect` makes a removed block fail lint instead of leaving a stale allow-path. `fuzz/` carries no lints.
- **Tests change the environment only through `test_helpers::EnvGuard`**, which holds the crate's only test `unsafe` block and restores on drop. `tests/test_env_guard_serial.py` requires every guard user to be `#[serial]`.
- **`command.run` passes `env:` to the child through `Exec.environment`** and never modifies etch's own environment. An unprivileged program is resolved on the step's own `PATH`.
- **`privileged: true` with a non-empty `env:` is refused at plan time.** The privilege provider would carry the values on its command line (`sudo env K=V …`), exposing them in `ps`, `/proc/<pid>/cmdline` and sudo logs. Refusing was chosen over `sudo --preserve-env` (rejected by default sudoers and not portable to `doas`/`run0`) and over passing them silently.
- **Edition 2024 in all four crates, no `rust-version`.** The crates ship as binaries, so nothing reads an MSRV, and a blocking MSRV job would stall grouped dependency updates. `style_edition` stays `"2021"` until a separate reformat-only PR.

## Consequences

- A GPL dependency, a disallowed source or a banned crate now fails a PR. Fixture crates in `tests/fixtures/deny/` prove the policy loads and rejects.
- An unsound advisory in any transitive crate fails the blocking job; it must be bumped or ignored with a dated reason in `deny.toml`.
- New process-env mutation in production code fails to compile (edition 2024 plus `unsafe_code = "deny"`), so the `command.run` defect class cannot return without a reviewed `#[expect]`.
- A manifest combining `privileged: true` and `env:` fails to plan. None existed in etch-config when this landed.
- Raw `cargo audit` is no longer run in CI; its GHSA-only blind spot is unchanged (Backlog P4).
- Under threaded runners (`cargo test`, tarpaulin), a non-`#[serial]` test that reads the environment still races with a guard holder. That residual race is unchanged from bare `set_var` and documented in `EnvGuard`'s SAFETY note.

## Related

- Spec: `docs/cursor/specs/2026-10-31-rust-standards-edition-2024-spec.md`
- Plan: `docs/cursor/plans/2026-10-31-rust-standards-edition-2024-plan.md`
- math ADR-0036 (edition 2024, no MSRV, `unsafe` forbid in math)
