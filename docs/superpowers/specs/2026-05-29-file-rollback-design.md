# File Rollback Design

> **Rewritten: 2026-06-12** — aligned with codebase as implemented (state manifest, history command, SetContents atom pattern, tarpaulin bifurcation, ETCH_STASH_DIR env var isolation).

## Overview

Before `file.copy` overwrites an existing file, stash the original to
`~/.local/share/etch/backups/<hex-of-path>/<rfc3339-timestamp>` with a
sidecar `.meta.yaml`. A new `etch rollback` subcommand lists stashes and
restores them on demand.

## Motivation

`etch apply` silently overwrites managed files. If the manifest contains a
bug or a merge conflict renders a config invalid, there is no fast path back.
Stashing originals before each write makes rollback instant without requiring
the user to maintain manual backups.

## CLI Invocation

```
etch rollback                            # list all stashed paths with timestamps; no restore
etch rollback --list                     # same as bare invocation (explicit form)
etch rollback --path ~/.zshrc            # restore latest stash for that path
etch rollback --path ~/.zshrc --dry-run  # show unified diff of stash vs current; no write
etch rollback --all                      # restore all paths to latest stash (requires confirmation)
etch rollback --all --yes                # non-interactive confirm for --all
```

Default (no flags): print the stash table and exit 0. Never restore without
an explicit `--path` or `--all`.

`--all` requires a `y/N` confirmation prompt unless `--yes` is passed. If
stdin is not a TTY and `--yes` is absent, `--all` errors.

`--dry-run` prints a unified diff (`--- current` / `+++ stash`) and exits 0.
Combine with `--path` only; `--all --dry-run` is an error.

## Stash Layout

```
~/.local/share/etch/backups/
    <hex>/                              # sha256(original_path) — stable per path
        2026-05-29T20:00:00Z            # stashed file content
        2026-05-29T20:00:00Z.meta.yaml  # sidecar metadata
        2026-05-28T10:00:00Z            # older stash
        2026-05-28T10:00:00Z.meta.yaml
```

Timestamps: RFC 3339 UTC, seconds precision (`%Y-%m-%dT%H:%M:%SZ`). Used as
both the stash filename and the `stashed_at` field in the sidecar.

**Hex computation:** `sha256::digest(original_path.to_string_lossy())` —
hex encoded, lowercase. Use the `sha256` crate (already in `lib/Cargo.toml`).
The path is the resolved destination path after tilde expansion, exactly as
stored on the `Stash` atom.

### Meta YAML Schema

```yaml
original_path: /home/bruce/.zshrc
stashed_at: "2026-05-29T20:00:00Z"
apply_manifest: /home/bruce/etch-config/mac-studio/core.yaml
sha256: "abc123..." # hex digest of stashed file content
```

`apply_manifest` is the manifest name from `manifest.name` (the manifest
file path). `sha256` is the digest of the file content at stash time.

## Prune Policy

After each stash, prune entries for that path to keep only the most recent N
(default 3). Configurable in `etch.yaml` as `rollback_keep: <n>`. Pruning
deletes both the stash file and its `.meta.yaml` sidecar.

## What Gets Stashed

| Context                      | Stash behavior                                                       |
| ---------------------------- | -------------------------------------------------------------------- |
| `file.copy` (non-privileged) | Stash if target exists and is a regular file; skip otherwise         |
| `file.copy` (privileged)     | Stash step runs before the sudo cp pipeline — same condition applies |
| `file.link`                  | Never stash — symlinks are trivially reversible (delete the link)    |
| All other actions            | Never stash                                                          |

Stash is best-effort. Failure to stash logs a warning and does not block apply.

## Architecture

### New files

#### `lib/src/rollback/mod.rs`

Mirrors `lib/src/state/mod.rs` in structure. Expose via `pub mod rollback;`
in `lib/src/lib.rs`.

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct StashMeta {
    pub original_path: String,
    pub stashed_at: DateTime<Utc>,
    pub apply_manifest: String,
    pub sha256: String,
}

#[derive(Debug)]
pub struct StashEntry {
    pub original_path: PathBuf,
    pub stashed_at: DateTime<Utc>,
    pub apply_manifest: String,
    pub sha256: String,
    pub stash_path: PathBuf,
    pub meta_path: PathBuf,
}

pub struct StashStore {
    base: PathBuf,  // ~/.local/share/etch/backups/
}

impl StashStore {
    /// Resolves `dirs_next::data_local_dir()/etch/backups/`.
    /// Override base dir with `ETCH_STASH_DIR` env var (integration tests).
    pub fn new() -> Self { ... }

    /// For unit tests — inject arbitrary base dir.
    pub fn with_base(base: PathBuf) -> Self { ... }

    /// Stash `path` if it is a regular file. Creates hex subdir, copies
    /// content, writes sidecar, then prunes. Returns Ok(true) if stashed,
    /// Ok(false) if skipped.
    pub fn stash(&self, path: &Path, manifest: &str, keep: usize) -> anyhow::Result<bool> { ... }

    /// Return all stash entries grouped by original path, newest-first within
    /// each group.
    pub fn list(&self) -> anyhow::Result<Vec<(PathBuf, Vec<StashEntry>)>> { ... }

    /// Restore latest stash for `path`. If dry_run, print unified diff and
    /// return without writing.
    pub fn restore(&self, path: &Path, dry_run: bool) -> anyhow::Result<()> { ... }

    /// Delete entries for `path` beyond the most recent `keep`.
    pub fn prune(&self, path: &Path, keep: usize) -> anyhow::Result<()> { ... }
}
```

#### `lib/src/atoms/file/stash.rs`

A new file atom. Add `mod stash; pub use stash::Stash;` to
`lib/src/atoms/file/mod.rs`.

```rust
pub struct Stash {
    pub path: PathBuf,
    pub manifest: String,
    pub keep: usize,
}

impl Atom for Stash {
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: self.path.is_file(),
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        match StashStore::new().stash(&self.path, &self.manifest, self.keep) {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::warn!("rollback: stash failed for {}: {e:#}", self.path.display());
                Ok(())  // best-effort; never block apply
            }
        }
    }

    fn status(&self) -> anyhow::Result<AtomStatus> {
        Ok(AtomStatus::Ok)  // stash has no drift concept
    }
}

impl std::fmt::Display for Stash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stash {} before overwrite", self.path.display())
    }
}
```

#### `app/src/commands/rollback.rs`

Follows the `History` command pattern exactly (bifurcated execute() for
tarpaulin, `_runtime: &Runtime`).

```rust
#[derive(clap::Args, Debug)]
pub(crate) struct Rollback {
    #[arg(long)]
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub list: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub all: bool,
    #[arg(long)]
    pub yes: bool,
}

impl EtchCommand for Rollback {
    #[cfg(not(tarpaulin_include))]
    fn execute(&self, _runtime: &Runtime) -> anyhow::Result<()> { ... }

    #[cfg(tarpaulin_include)]
    fn execute(&self, _runtime: &Runtime) -> anyhow::Result<()> { unreachable!() }
}
```

### Modified files

**`lib/src/actions/file/copy.rs`** — in the non-privileged `plan()` path,
prepend a `Stash` step before the `DirCreate` step:

```rust
steps.insert(0, Step {
    atom: Box::new(crate::atoms::file::Stash {
        path: path.clone(),
        manifest: manifest.name.clone().unwrap_or_default(),
        keep: 3,  // TODO: thread from Config when runtime is available at plan time
    }),
    initializers: vec![],
    finalizers: vec![],
});
```

For the privileged path, prepend a `Stash` step before the `DirCreate` Exec
step (same pattern — Stash runs non-privileged, before the sudo pipeline).

**`lib/src/lib.rs`** — add `pub mod rollback;`.

**`lib/src/atoms/file/mod.rs`** — add `mod stash; pub use stash::Stash;`.

**`app/src/config/mod.rs`** — add `Rollback(commands::Rollback)` to
`Commands` enum.

**`app/src/main.rs`** — add `Commands::Rollback(r) => r.execute(&runtime)`
to the `execute()` match.

**`app/src/commands/mod.rs`** — add `mod rollback; pub(crate) use rollback::Rollback;`.

**`lib/Cargo.toml`** — add `similar = "2"` for unified diff in
`restore --dry-run`.

### `rollback_keep` hardcoded at 3 for now

The `plan()` method does not have access to `Runtime` or `Config`. For the
initial implementation, hardcode `keep: 3` in the `Stash` atom. A follow-up
can thread the config value when the plan/execute split allows it. Do not add
`rollback_keep` to `Config` now — avoid premature config surface area.

## Output Format

**`etch rollback` / `etch rollback --list`:**

```
PATH                    STASHES  LATEST
~/.zshrc                2        2026-05-29 20:00:00 UTC
~/.config/nvim/init.lua 1        2026-05-28 10:00:00 UTC
```

**`etch rollback --path ~/.zshrc`:**

```
Restoring ~/.zshrc from stash 2026-05-29T20:00:00Z
Done.
```

**`etch rollback --path ~/.zshrc --dry-run`:**

```
--- current (~/.zshrc)
+++ stash (2026-05-29T20:00:00Z)
@@ -1,3 +1,3 @@
-export PATH="..."
+export PATH="..."
```

**`etch rollback --all`:**

```
Will restore 2 path(s) to their latest stash:
  ~/.zshrc            → stash 2026-05-29T20:00:00Z
  ~/.config/nvim/init.lua → stash 2026-05-28T10:00:00Z
Continue? [y/N] _
```

## Error Handling

| Condition                                       | Behavior                                                                      |
| ----------------------------------------------- | ----------------------------------------------------------------------------- |
| Target not regular file (dir, symlink, missing) | Skip stash; `stash()` returns `Ok(false)`                                     |
| Backup dir not creatable                        | Warn: `"rollback: cannot create backup dir, skipping stash"`; apply continues |
| Stash file write fails                          | Warn with error; apply continues (best-effort)                                |
| `--path X` with no stash for X                  | Error: `"no stash found for <X>"`; exit 1                                     |
| Stash file unreadable                           | Skip entry with warn: `"rollback: corrupt stash at <path>, skipping"`         |
| Meta sidecar missing                            | Skip entry with warn                                                          |
| `--all` with no TTY and no `--yes`              | Error: `"--all requires confirmation; pass --yes to skip prompt"`; exit 1     |
| `--dry-run` combined with `--all`               | Error: `"--dry-run cannot be combined with --all"`; exit 1                    |
| Restore write fails                             | Error with path and OS error; exit 1; stash file preserved                    |
| Empty stash store                               | Print table header only; exit 0                                               |

## Dependencies

| Crate           | Already present?                 | Purpose                       |
| --------------- | -------------------------------- | ----------------------------- |
| `sha256`        | yes                              | path hex key + content digest |
| `chrono`        | yes                              | `DateTime<Utc>` timestamps    |
| `dirs-next`     | yes                              | base dir resolution           |
| `serde_yaml_ng` | yes                              | `.meta.yaml` serialization    |
| `similar`       | **no — add to `lib/Cargo.toml`** | unified diff in `--dry-run`   |

## Testing

### Unit tests — `lib/src/rollback/mod.rs`

Use `StashStore::with_base(tempdir)` for full isolation.

- `stash()` creates `<hex>/` dir, copies file content, writes `.meta.yaml`
- `stash()` on missing path returns `Ok(false)`, creates no files
- `stash()` on directory returns `Ok(false)`, creates no files
- `stash()` meta YAML roundtrip: all four fields present and correct
- `prune()` with `keep=2` and 4 stashes: deletes two oldest (file + sidecar); keeps two newest
- `prune()` with `keep=3` and 2 stashes: no deletion
- `list()` returns entries grouped by path, newest-first within each group
- `list()` on non-existent backups dir returns `Ok(vec![])`
- `restore(dry_run=false)`: overwrites target with stash content; stash file preserved
- `restore(dry_run=true)`: stdout contains `---` and `+++` diff markers; target unchanged
- `restore()` on unknown path returns `Err` with "no stash found"
- `new()` respects `ETCH_STASH_DIR` env var (same pattern as `StateStore`)

### Unit tests — `lib/src/atoms/file/stash.rs`

- `plan()` returns `should_run=true` when path is regular file
- `plan()` returns `should_run=false` when path is missing
- `plan()` returns `should_run=false` when path is directory
- `execute()` calls `StashStore::stash()` and returns `Ok(())` even on stash failure (best-effort)

### Unit tests — `app/src/commands/rollback.rs`

Test the logic-extraction pattern (same as `history.rs`'s `render_table`):

- No flags → list table printed to output, exit 0
- `--path` without `--dry-run` → restore called, confirmation printed
- `--path --dry-run` → dry-run diff printed, no write
- `--all --dry-run` → returns error before any store call
- `--all` without TTY and without `--yes` → returns error before any store call

### Integration tests — `app/tests/rollback.rs`

Use `ETCH_STASH_DIR` env var to redirect stash to a tempdir.

- Apply `file.copy` that overwrites a pre-existing file → assert
  `<stash_base>/<hex>/` dir exists with one stash file and one `.meta.yaml`
- Apply same `file.copy` a second time → stash count is 2; content of first
  stash matches the original pre-apply content
- Mutate destination after apply → run `etch rollback --path <dest>` →
  destination content matches stash content
- Apply `file.copy` 4 times → stash count for that path is 3 (oldest pruned)
- Apply `file.link` → assert no stash dir created for the symlink target

## Relationship to State Manifest

The stash store and `state.yaml` share `~/.local/share/etch/` but are
independent structures. `apply_manifest` in `.meta.yaml` mirrors
`StateEntry::manifest` — redundant by design so rollback metadata is
self-contained without parsing `state.yaml`.

## Tarpaulin Notes

- `Rollback::execute()` must be bifurcated with `#[cfg(not(tarpaulin_include))]`
  / `#[cfg(tarpaulin_include)]` (same pattern as `History::execute()`).
- `Stash::execute()` calls `StashStore::new()` which is tested via the
  `ETCH_STASH_DIR` env var — unit tests for the atom should use that var or
  mock via `with_base()` passed to a testable helper.
- Add `unexpected_cfgs` lint entry to `lib/Cargo.toml` under `[lints.rust]`
  if not already present.
