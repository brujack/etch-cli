# ADR-0019: Plugin Lua is restricted by default; only a real apply grants full authority

**Date:** 2026-11-21
**Status:** Accepted

## Context

The `plugin` action evaluates an operator-named `plugin.lua` to build its spec. Every evaluation used
`Lua::unsafe_new` with `StdLib::ALL`, so plugin code always had `io`, `os` and `debug`.

Evaluation happens at plan and summarize time, not only at execute time. `etch apply --dry-run` and
`etch status` both plan every action, so both evaluated plugin code with full authority. Operators run
those two commands precisely because they are believed to be read-only (audit 2026-11-16 F15).

Alternatives considered:

- **Skip plugin evaluation under dry-run.** This needs the action to know the run mode, and dry-run
  could then no longer report a plugin's steps or catch a plugin that fails to load.
- **Restrict plugin Lua everywhere.** This would break plugins that legitimately need the host during
  a real apply. Whether a real apply should keep `debug` is left open (Backlog).
- **Thread a flag through `Action::plan`.** This changes a trait every action implements, to serve
  one action.

## Decision

Plugin Lua is evaluated in a restricted state unless the process has been granted full authority.

- **Restricted:** `string`, `table`, `math`, `utf8` and `coroutine`, built with the safe
  `Lua::new_with`. `dofile` and `loadfile` are removed, and `load` is wrapped to accept source text
  only, since precompiled chunks can corrupt interpreter memory.
- **Full:** `Lua::unsafe_new` with `StdLib::ALL`.
- The authority is process-wide state in `etch_lib::actions` (`set_plugin_authority`,
  `plugin_authority`), and it defaults to Restricted.
- The one grant site is `Apply::enter_run_mode`, which grants Full only when `--dry-run` is absent.
- The plugin spec cache is keyed by `(source, authority)`, so a spec built in one state is never
  reused in the other.

## Consequences

- `etch apply --dry-run` and `etch status` can plan plugins without giving them the filesystem,
  processes or environment. Plugin code still runs in those commands; it just cannot reach the host.
- A plugin that uses `io` or `os` at load or plan time must guard those uses (`if io then ... end`).
  Otherwise its dry-run logs the action as `FAILED`, and `etch status` skips it with a warning. The
  dry-run still exits 0 in that case (a pre-existing behaviour, filed as a P2 Backlog row).
- The restriction is fail-closed. Any new command that plans actions gets Restricted unless it calls
  `set_plugin_authority`, and it must not call it unless it really executes plugin steps.
- The state is process-global, so lib tests that change it are `#[serial]` and restore Restricted
  through a drop guard.
- Not addressed here: `Repo` plugin sources still resolve mutable refs and cannot be pinned to a
  commit SHA, and a dry-run still fetches them over the network (Backlog).

## Related

- etch-cli#169 (`33cb972`): the implementation and its regression tests in `app/tests/plugin.rs`
- [docs/src/plugin.md](../src/plugin.md): "What plugin code can reach"
- [ADR-0018](0018-cargo-deny-unsafe-guard-edition-2024.md): the `unsafe_code` guard that the `Full`
  arm's `#[expect(unsafe_code, reason = ...)]` satisfies
