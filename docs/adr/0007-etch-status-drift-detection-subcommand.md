# ADR-0007: etch status drift detection subcommand

**Date:** 2026-05-19
**Status:** Accepted

## Context

`etch apply` is idempotent but blind — it runs every action and reports success whether or not anything changed. No way to ask "is this machine in the desired state?" without running apply and inspecting its output.

This became a problem when using etch in a monitoring context: a cron job or CI check needs a non-zero exit code when the machine has drifted from the manifest, without actually applying changes. Running `etch apply` as a drift check has unacceptable side effects (package installs, file writes, group changes).

Alternatives considered: add a `--dry-run` flag to `apply` (hard to implement correctly — dry-run semantics differ per action type), parse `apply` output for "changed" markers (fragile, format not guaranteed), use a separate external script to check state (defeats the purpose of a declarative tool).

## Decision

Add `etch status` subcommand that runs the same manifest resolution as apply but in read-only mode, reporting whether each action is:

- `ok` — current machine state matches desired state
- `drifted` — current state differs from desired state
- `unknown` — status cannot be determined without running the action

Each action's `status()` method checks the current state using the same logic as its idempotency guard. Results are aggregated and printed as a table. Exit code is 0 when all actions are `ok`; non-zero when any action is `drifted` or `unknown`.

Output format is JSON (with a `--json` flag) and human-readable table (default). 7 integration tests added (PR #73) verify JSON output format and exit codes.

## Consequences

- Drift detection is safe to run in monitoring, cron, and CI contexts without side effects.
- Each new action added to etch-lib must implement a `status()` method in addition to `execute()`. This is a new contract on the action trait.
- Status subcommand shares struct/config with apply — any new action needs both `execute()` and `status()` before it can be used with `etch status`.
- Integration tests require real filesystem setup (tempdir) and JSON output parsing — they are slower than unit tests and run in the `integration-test` CI job separately from the main test suite.

## Related

- PR #73 (etch status implementation and integration tests)
- ADR-0006: native action expansion strategy (status() added to each action)
- `etch-lib/src/actions/mod.rs` — action trait definition
