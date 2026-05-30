# File Rollback Design

## Overview

Before `file.copy` overwrites an existing file, stash the original to `~/.local/share/etch/backups/<hex-of-path>/<rfc3339-timestamp>` with a sidecar `.meta.yaml`. A new `etch rollback` subcommand lists stashes and restores them on demand.

## Motivation

`etch apply` silently overwrites managed files. If the manifest contains a bug or a merge conflict renders a config invalid, there is no fast path back to the previous working state. Stashing originals before each write makes rollback instant and audit-ready without requiring the user to maintain manual backups.

## CLI Invocation

```
etch rollback                            # list all stashed paths with timestamps; no restore
etch rollback --list                     # same as bare invocation (explicit form)
etch rollback --path ~/.zshrc            # restore latest stash for that path
etch rollback --path ~/.zshrc --dry-run  # show unified diff of stash vs current; no write
etch rollback --all                      # restore all paths to latest stash (requires confirmation)
etch rollback --all --yes                # non-interactive confirm for --all
```

Default (no flags): print the stash table and exit 0. Never restore without an explicit `--path` or `--all`.

`--all` requires a `y/N` confirmation prompt at the terminal unless `--yes` is passed. If stdin is not a TTY and `--yes` is absent, `--all` errors.

`--dry-run` prints a unified diff (`--- current` / `+++ stash`) and exits 0. It can be combined with `--path` only; `--all --dry-run` is an error.

## Stash Layout

```
~/.local/share/etch/backups/
    <hex>/                              # sha256(original_path_string) — stable per path
        2026-05-29T20:00:00Z            # stashed file content
        2026-05-29T20:00:00Z.meta.yaml  # sidecar metadata
        2026-05-28T10:00:00Z            # older stash
        2026-05-28T10:00:00Z.meta.yaml
```

Timestamps in RFC 3339 UTC with seconds precision (`%Y-%m-%dT%H:%M:%SZ`). The timestamp is computed at stash time and used as both the stash file name and the `stashed_at` field in the sidecar.

**Hex computation:** `sha256(original_path.to_string_lossy())` — hex encoded, lowercase. The path is the resolved destination path before tilde expansion, exactly as stored in the `file.copy` action's `to`/`target` field after template rendering.

### Meta YAML Schema

```yaml
original_path: ~/.zshrc
stashed_at: "2026-05-29T20:00:00Z"
apply_manifest: ~/etch-config/mac-studio/core.yaml
sha256: "abc123..." # hex digest of stashed file content
```

`apply_manifest` is the path of the manifest that triggered the apply. `sha256` is the digest of the file content at stash time (not of the incoming file).

## Prune Policy

After stashing, prune stashes for that path to keep only the most recent N (default 3). Configurable in `etch.yaml` as `rollback_keep: <n>`. Pruning deletes both the stash file and its `.meta.yaml` sidecar. Prune runs after every successful stash, not as a separate command.

## What Gets Stashed

| Atom            | Stash behavior                                                                         |
| --------------- | -------------------------------------------------------------------------------------- |
| `file.copy`     | Stash if target exists and is a regular file; skip if target is missing or non-regular |
| `file.link`     | Never stash — symlinks are trivially reversible (delete the link)                      |
| All other atoms | Never stash                                                                            |

Stash is best-effort. Failure to stash logs a warning and does not block the apply.

## Architecture

### New files

**`lib/src/rollback/mod.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    pub stash_path: PathBuf,    // path of the stash file on disk
    pub meta_path: PathBuf,     // path of the .meta.yaml sidecar
}

pub struct StashStore {
    base: PathBuf,              // ~/.local/share/etch/backups/
}

impl StashStore {
    /// Resolves `dirs_next::data_local_dir()/etch/backups/`.
    /// Falls back to `~/.local/share/etch/backups/` if dirs_next returns None.
    pub fn new() -> Self { ... }

    /// Stash `path` if it exists and is a regular file. Creates hex subdir,
    /// copies current content, writes sidecar, then calls prune(path, keep).
    /// Returns Ok(true) if stash was created, Ok(false) if skipped (missing/non-regular).
    pub fn stash(&self, path: &Path, manifest: &str, keep: usize) -> anyhow::Result<bool> { ... }

    /// Return all stash entries grouped by original path, sorted by stashed_at descending.
    pub fn list(&self) -> anyhow::Result<Vec<(PathBuf, Vec<StashEntry>)>> { ... }

    /// Restore the latest stash for `path` to its original_path.
    /// If dry_run is true, print a unified diff to stdout and return without writing.
    pub fn restore(&self, path: &Path, dry_run: bool) -> anyhow::Result<()> { ... }

    /// Delete stash entries for `path` beyond the most recent `keep`.
    /// Both stash file and sidecar are removed.
    pub fn prune(&self, path: &Path, keep: usize) -> anyhow::Result<()> { ... }
}
```

**`lib/src/rollback/` is a new module** — add `pub mod rollback;` to `lib/src/lib.rs`.

**`app/src/commands/rollback.rs`**

```rust
#[derive(clap::Args, Debug)]
pub struct RollbackArgs {
    /// Restore latest stash for this path
    #[arg(long)]
    pub path: Option<PathBuf>,
    /// List all stashed paths with timestamps (default behavior)
    #[arg(long)]
    pub list: bool,
    /// Show diff of what would be restored; no write (requires --path)
    #[arg(long)]
    pub dry_run: bool,
    /// Restore all paths to their latest stash
    #[arg(long)]
    pub all: bool,
    /// Skip confirmation prompt for --all
    #[arg(long)]
    pub yes: bool,
}

pub struct RollbackCommand;

impl EtchCommand for RollbackCommand {
    type Args = RollbackArgs;
    fn execute(args: Self::Args, config: &Config) -> anyhow::Result<()> { ... }
}
```

### Modified files

**`lib/src/atoms/file/copy.rs`** — in `execute()`, before writing the destination, call `StashStore::new().stash(&dest, manifest_path, config.rollback_keep())`. The manifest path is threaded through via a new field `pub manifest: String` on the `FileCopy` atom struct, set by `file.copy`'s `plan()`.

**`lib/src/actions/file/copy.rs`** — in `plan()`, populate `FileCopy::manifest` from the manifest path passed through the action context (already available as `manifest_path` in the action plan signature).

**`lib/Cargo.toml`** — add `similar = "2"` for unified diff generation in `restore --dry-run`.

**`app/src/config/mod.rs`** — add `Rollback(RollbackArgs)` to `Commands` enum; add `rollback_keep: Option<usize>` to `Config` struct (defaults to 3 if absent).

**`app/src/commands/mod.rs`** — register `rollback` subcommand.

## `rollback_keep` Config Field

```yaml
# ~/.config/etch/etch.yaml
rollback_keep: 5 # optional; default 3
```

`Config::rollback_keep()` returns `self.rollback_keep.unwrap_or(3)`.

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
-export PATH="..."   # current
+export PATH="..."   # stashed version
```

**`etch rollback --all`:**

```
Will restore 2 path(s) to their latest stash:
  ~/.zshrc            → stash 2026-05-29T20:00:00Z
  ~/.config/nvim/init.lua → stash 2026-05-28T10:00:00Z
Continue? [y/N] _
```

## Error Handling

| Condition                                          | Behavior                                                                             |
| -------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Target not a regular file (dir, symlink, missing)  | Skip stash silently; `stash()` returns `Ok(false)`                                   |
| Backup dir not creatable                           | Log warning: `"rollback: cannot create backup dir, skipping stash"`; apply continues |
| Stash file write fails                             | Log warning with error; apply continues (stash is best-effort)                       |
| `--path X` with no stash found for X               | Error: `"no stash found for <X>"`; exit 1                                            |
| Stash file content unreadable                      | Skip that entry with warning: `"rollback: corrupt stash at <path>, skipping"`        |
| Meta sidecar missing for an otherwise valid stash  | Skip that entry with warning                                                         |
| `--all` with no TTY and `--yes` absent             | Error: `"--all requires confirmation; pass --yes to skip prompt"`; exit 1            |
| `--dry-run` combined with `--all`                  | Error: `"--dry-run cannot be combined with --all"`; exit 1                           |
| Restore write fails (permissions, disk full, etc.) | Error with path and OS error; exit 1; original stash file is not deleted             |
| Empty stash store (no backups dir or empty)        | Print empty table with header; exit 0                                                |

## Testing

**Unit tests (`lib/src/rollback/mod.rs`):**

- `stash()` creates `<hex>/` dir, copies file content, writes `.meta.yaml` sidecar
- `stash()` on a path that does not exist returns `Ok(false)`, creates no files
- `stash()` on a directory (non-regular file) returns `Ok(false)`, creates no files
- `stash()` meta YAML roundtrip: `original_path`, `stashed_at`, `apply_manifest`, `sha256` fields all present and correct
- `prune()` with `keep = 2` and 4 stashes: deletes the two oldest stash files and their sidecars; leaves the two newest
- `prune()` with `keep = 3` and 2 stashes: no deletion
- `list()` returns entries grouped by path, sorted newest-first within each group
- `list()` on a non-existent backups dir returns `Ok(vec![])`
- `restore()` with `dry_run = false`: overwrites target with stash content; stash file is preserved
- `restore()` with `dry_run = true`: stdout contains `---` and `+++` diff markers; target file is unchanged
- `restore()` on a path with no stash returns `Err` with "no stash found" message

**Unit tests (`app/src/commands/rollback.rs`):**

- No flags: calls `list()`, prints table, exits 0
- `--path` without `--dry-run`: calls `restore(path, false)`, prints confirmation
- `--path --dry-run`: calls `restore(path, true)`
- `--all --dry-run`: returns error before any store call
- `--all` without TTY and without `--yes`: returns error before any store call

**Integration tests (`app/tests/integration.rs`):**

- Apply `file.copy` that overwrites a pre-existing file → assert `~/.local/share/etch/backups/<hex>/` dir exists with one stash file and one `.meta.yaml`
- Apply same `file.copy` a second time → stash count is 2 (not 1); content of first stash matches original pre-apply content
- Mutate the destination file after apply → run `etch rollback --path <dest>` → assert destination content matches the stash content (original pre-apply content)
- Apply `file.copy` 4 times with `rollback_keep: 3` configured → stash count for that path is 3 (oldest pruned)
- Apply `file.link`: assert no stash dir is created for the symlink target

## Dependencies

- `similar = "2"` — unified diff for `--dry-run` output (`lib/Cargo.toml`)
- `dirs-next = "2"` — base dir resolution (already added by state manifest spec)
- `chrono = { version = "0.4", features = ["serde"] }` — timestamps (already added by state manifest spec)
- `sha2 = "0.10"` — stash content digest (already added by state manifest spec)

## Relationship to State Manifest

The stash store and the state manifest (`~/.local/share/etch/state.yaml`) share the `~/.local/share/etch/` base directory but are independent structures. The `apply_manifest` field in `.meta.yaml` mirrors the `manifest` field in `StateEntry` for the same atom — they are redundant by design so rollback metadata is self-contained and readable without parsing `state.yaml`.
