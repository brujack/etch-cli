# etch update Command Design

## Overview

Add an `etch update` subcommand that runs all upgrade-capable atoms across all configured manifests in a single invocation. Only atoms that declare `upgradeable() -> bool` == `true` execute; all other atoms are skipped.

## Motivation

Users who want to keep their workstations current must today maintain a separate `updates.yaml` manifest containing only upgrade actions and run `etch apply updates.yaml` explicitly. This duplicates information already present in the full manifest set and requires users to manually synchronise two files. `etch update` eliminates the separate manifest by reading upgrade intent from the existing manifests directly.

## CLI Invocation

```
etch update [OPTIONS]

Options:
  --dry-run          Show which upgrade-capable atoms would run; do not execute
  --manifest <FILE>  Limit to a single manifest file (repeatable)
  --full             Run ALL atoms, not just upgrade-capable ones (equivalent to etch apply)
  --fail-fast        Stop on the first atom failure instead of continuing
  -v, --verbose      Show per-atom detail (mirrors etch apply -v)
```

```
# All upgrade-capable atoms across all configured manifests
etch update

# Preview only
etch update --dry-run

# Single manifest
etch update --manifest core.yaml

# Two specific manifests
etch update --manifest core.yaml --manifest dev-tools.yaml

# Full apply (escape hatch — prefer etch apply for day-to-day use)
etch update --full
```

`--full` disables the `upgradeable()` filter — all atoms from all manifests execute, identical to `etch apply`. It is documented as an escape hatch for users who want a single entry point for all operations, not as the primary use case.

## Output Format

```
Updating 3 manifests (12 upgrade-capable atoms found)

[core.yaml]
  ✓ brew.upgrade          formulae upgraded (4 changed)
  ✓ mas.upgrade           all apps current

[dev-tools.yaml]
  ✓ git.pull:dotfiles     already up to date
  ✗ git.pull:nvim-config  failed: network unreachable

[packages.yaml]
  ✓ package.upgrade:apt   6 packages upgraded

Update complete: 4 succeeded, 1 failed
```

`--dry-run` output replaces `✓`/`✗` with `·` (would run) and adds a `(dry run)` suffix to the summary line. `--verbose` adds per-atom output below each line, matching `etch apply -v` behaviour.

## Architecture

### New files

**`app/src/commands/update.rs`**

```rust
pub struct UpdateCommand {
    pub dry_run: bool,
    pub manifests: Vec<PathBuf>,   // empty == all configured manifests
    pub full: bool,
    pub fail_fast: bool,
    pub verbose: bool,
}
```

`execute()` logic:

1. Load config from `etch.yaml` (same path as `ApplyCommand`).
2. Resolve manifest list: if `--manifest` flags were given use those paths; otherwise use all paths from `config.manifest_paths`. Fail with a clear error if any path does not exist.
3. Load and DAG-resolve manifests in dependency order (reuse `manifests::resolve_manifests()` — same as `ApplyCommand`).
4. For each manifest, call `action.plan(context)` on all actions to get `Vec<Step>`.
5. Unless `--full`: filter the step list to steps whose atom returns `upgradeable() == true`.
6. If the filtered list is empty across all manifests: print `"Nothing to update."` and exit 0.
7. Print the summary header: `"Updating N manifests (M upgrade-capable atoms found)"`.
8. For each manifest (in DAG order), execute its upgrade steps sequentially:
    - Print per-atom result line as each atom completes.
    - On failure: log the error and continue to the next atom (best-effort). If `--fail-fast`: propagate the error and stop.
9. Print the final summary line: `"Update complete: N succeeded, M failed"`. Exit non-zero if any atom failed.

State recording: after each atom executes successfully, call `StateStore::record()` with the atom result — identical to `ApplyCommand` behaviour.

**`lib/src/atoms/mod.rs`** — add `upgradeable()` default to the `Atom` trait

```rust
pub trait Atom: fmt::Debug {
    fn plan(&self) -> anyhow::Result<()>;
    fn execute(&self) -> anyhow::Result<()>;
    fn output(&self) -> String;

    /// Returns true if this atom should run during `etch update`.
    /// Default is false; upgrade-capable atoms override to true.
    fn upgradeable(&self) -> bool {
        false
    }
}
```

### Modified files

**`app/src/commands/mod.rs`** — add `Update` variant to the `Commands` enum; add `pub mod update;`

**`app/src/config/mod.rs`** — add `Update(UpdateArgs)` to the `Commands` enum, where `UpdateArgs` is the clap struct for `update` subcommand options

**`app/src/main.rs`** — dispatch `Commands::Update` to `UpdateCommand::execute()`

**`lib/src/atoms/file/chflags.rs`** — no change (inherits `upgradeable() = false`)

**`lib/src/atoms/brew/upgrade.rs`** — override `upgradeable()` to return `true`

**`lib/src/atoms/mas/upgrade.rs`** — override `upgradeable()` to return `true`

**`lib/src/atoms/git/pull.rs`** — override `upgradeable()` to return `true`

**`lib/src/actions/package/upgrade.rs`** — atom already declares `upgradeable()` returning `true` per the `package.upgrade` spec; no additional change

**`lib/src/atoms/binary/url.rs`** — override `upgradeable()` to return `true` only when the installed version differs from the declared version (version check happens at `plan()` time; the atom records whether an upgrade is needed; `upgradeable()` returns that flag)

Non-upgrade atoms (`Exec`, `FileCopy`, `FileLink`, `Chmod`, `Chown`, `Chflags`, `CreateDirectory`, `Http`) inherit the default `false` and require no changes.

## Atom upgradeable() Summary

| Atom                               | `upgradeable()` | Reason                                                            |
| ---------------------------------- | --------------- | ----------------------------------------------------------------- |
| `brew::Upgrade`                    | `true`          | Core upgrade atom for Homebrew                                    |
| `mas::Upgrade`                     | `true`          | Core upgrade atom for Mac App Store                               |
| `git::Pull`                        | `true`          | Always-execute pull semantics; designed for keeping repos current |
| `package::upgrade::PackageUpgrade` | `true`          | Explicit upgrade action for apt/snap                              |
| `binary::url::BinaryUrl`           | `true`          | Returns `true` when plan determines installed != declared version |
| `Exec`                             | `false`         | Arbitrary commands — unsafe to re-run without explicit intent     |
| `FileCopy`                         | `false`         | File presence action — not an upgrade                             |
| `FileLink`                         | `false`         | Symlink action — not an upgrade                                   |
| `Chmod` / `Chown`                  | `false`         | Permission actions — not upgrades                                 |
| `Chflags`                          | `false`         | BSD flag action — not an upgrade                                  |
| `CreateDirectory`                  | `false`         | Directory creation — not an upgrade                               |
| `Http`                             | `false`         | HTTP download — not an upgrade                                    |

## Error Handling

| Condition                                | Behavior                                                                  |
| ---------------------------------------- | ------------------------------------------------------------------------- |
| Named manifest path does not exist       | Error before execution: `"manifest not found: <path>"`                    |
| Manifest YAML parse error                | Error before execution, same as `etch apply`                              |
| DAG cycle detected                       | Error before execution, same as `etch apply`                              |
| No upgrade-capable atoms in any manifest | Print `"Nothing to update."`, exit 0                                      |
| Atom execution fails (default)           | Log error with atom label, continue remaining atoms, exit non-zero at end |
| Atom execution fails (`--fail-fast`)     | Log error, stop immediately, exit non-zero                                |
| `--manifest` and `--full` combined       | Allowed — limits manifests but runs all atoms within those manifests      |
| `etch.yaml` not found                    | Error before execution, same as `etch apply`                              |
| `StateStore::record()` fails             | Log warning, continue — state recording failure does not abort the update |

## Testing

**Unit tests (`lib/src/atoms/mod.rs`):**

- Default `upgradeable()` returns `false` — verified on a concrete non-upgrade atom (`Exec`)
- `brew::Upgrade::upgradeable()` returns `true`
- `mas::Upgrade::upgradeable()` returns `true`
- `git::Pull::upgradeable()` returns `true`
- `PackageUpgrade::upgradeable()` returns `true`
- `BinaryUrl::upgradeable()` returns `false` when installed version matches declared; `true` when it differs

**Unit tests (`app/src/commands/update.rs`):**

- Filter logic: given a `Vec<Step>` with mixed upgrade-capable and non-upgrade atoms, only upgrade-capable steps pass through
- `--full` disables filter — all steps pass through regardless of `upgradeable()`
- Empty result after filter → returns early with "Nothing to update" message
- `--fail-fast`: when second atom fails, third atom does not execute
- Default (no `--fail-fast`): when second atom fails, third atom still executes; exit code is non-zero

**Integration tests (`app/tests/integration.rs`):**

- Apply a manifest containing `file.link` + `git.pull` steps; run `etch update --dry-run`; verify output lists only `git.pull` (the upgrade-capable atom) and does not execute it
- Apply a manifest containing only non-upgrade atoms; run `etch update`; verify `"Nothing to update."` output and exit 0
- `etch update --full` on a manifest containing `file.link` + `command.run`; verify both atoms execute (same as `etch apply`)

**Non-goals:**

- Scheduling or background execution — `etch update` is always an explicit user invocation
- Network connectivity pre-check — provider errors surface per-atom as they do in `etch apply`
- Parallel atom execution — steps execute sequentially within each manifest, manifests execute in DAG order
