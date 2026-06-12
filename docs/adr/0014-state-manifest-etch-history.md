# ADR-0014: State Manifest and etch history Subcommand

**Date:** 2026-06-12
**Status:** Accepted

## Context

etch-cli had no memory of what it applied. After `etch apply` ran, there was no
record of which manifests touched which files, when they last ran, or whether a
given atom changed state or was a no-op. This made auditing impossible and gave
drift-detection (ADR-0007) no known-good baseline to compare against.

Alternatives considered:

- **SQLite database** — richer querying but heavyweight for a personal tool; adds
  a C dependency; overkill for the single-machine, single-user use case.
- **Append-only log** — simpler but unbounded growth and no easy "current state"
  view; requires compaction logic.
- **In-manifest state** — storing state alongside the manifest files would couple
  two concerns and violate the dotfiles-as-source-of-truth principle.

## Decision

After each successful `etch apply`, write `~/.local/share/etch/state.yaml`
(XDG-compliant via `dirs_next::data_local_dir()`) recording every atom that
executed. The file is a **current-state snapshot**, not a history log: a second
apply on the same `(manifest, action, key)` triple overwrites the row rather than
appending a new one.

Key design choices:

- **XDG data dir** — survives home directory moves; consistent with freedesktop
  conventions. Override via `ETCH_STATE_DIR` env var for test isolation.
- **Merge semantics** — the file records most-recent outcome per
  `(manifest, action, key)` triple. Full history would require log rotation;
  current-state is sufficient for drift detection and auditing.
- **Best-effort write** — state failure logs a warning and never blocks a
  successful apply. Users should not lose apply results because of a disk issue.
- **Atomic write** — serialize to `.state.yaml.tmp` then `fs::rename` to avoid
  corrupt reads if the process is interrupted mid-write.
- **`state_key` per action type** — each action provides a canonical identifier
  (destination path for file actions, package name for package actions, etc.) via
  the `Action::state_key()` trait method. This enables deduplication without
  encoding action semantics in the state module.
- **`etch history` subcommand** — reads the state file, supports `--manifest`
  substring filter and `--json` (NDJSON) output. Exit 1 on unreadable state file;
  empty table on missing file.

## Consequences

**Easier:**

- Auditing — "which manifest last touched `~/.zshrc`?" is answerable.
- Drift detection baseline — a future `etch drift` command can diff the state
  file against live filesystem state.
- Debugging — `etch history --json` is machine-readable for scripts.

**Harder / Required:**

- Every new action type must implement `state_key()` to be recorded with a
  meaningful key; the default returns `""` which silently skips recording.
- The state file is not versioned; `schema_version: 1` allows future migration
  but no migration code exists yet.
- History is per-machine, not per-manifest-repo — it does not travel with
  dotfiles.

## Related

- [ADR-0007](0007-etch-status-drift-detection-subcommand.md) — `etch status`
  drift detection, which this state file is designed to serve as baseline for.
- [specs/2026-05-29-state-manifest-design.md](../superpowers/specs/2026-05-29-state-manifest-design.md)
