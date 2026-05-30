# State Manifest Design

## Overview

After each successful `etch apply`, write `~/.local/share/etch/state.yaml` recording every atom that was executed: which manifest triggered it, the action type, a canonical key (file path, package name, repo URL), whether it produced a change, and the SHA-256 digest for file atoms. A new `etch history` subcommand reads the state file and prints a human-readable or JSON summary.

## Motivation

etch-cli currently has no memory of what it has applied. This makes it hard to audit which manifests touched a file, answer "when was this last configured?", or feed a future drift-detection step with a known-good baseline. The state manifest is that persistent record.

## State File Location

`~/.local/share/etch/state.yaml` — resolved at runtime via `dirs_next::data_local_dir()`. XDG-compliant; survives home directory moves. Never committed to dotfiles.

## State File Schema (YAML)

```yaml
schema_version: 1
last_apply: "2026-05-29T20:00:00Z"
atoms:
    - manifest: ~/git-repos/personal/etch-config/mac-studio/core.yaml
      action: file.copy
      key: "~/.zshrc"
      applied_at: "2026-05-29T20:00:00Z"
      sha256: "abc123..." # hex digest of destination file after apply; null for non-file atoms
      changed: true # true when the atom mutated state in this run
    - manifest: ~/git-repos/personal/etch-config/mac-studio/tools.yaml
      action: git.clone
      key: "~/.oh-my-zsh"
      applied_at: "2026-05-29T20:00:00Z"
      sha256: null
      changed: false
```

Each row is identified by the triple `(manifest, action, key)`. A second `etch apply` on the same triple overwrites the existing row rather than appending a new one. The file captures the most-recent apply outcome per atom, not a full history log.

## CLI Invocation — `etch history`

```
etch history                          # table of all recorded atoms
etch history --manifest <path>        # filter by manifest path (substring match)
etch history --json                   # NDJSON, one object per atom
```

**Table output (default):**

```
MANIFEST                                              ACTION      KEY                 APPLIED AT            CHANGED
~/etch-config/mac-studio/core.yaml                    file.copy   ~/.zshrc            2026-05-29 20:00:00   yes
~/etch-config/mac-studio/tools.yaml                   git.clone   ~/.oh-my-zsh        2026-05-29 20:00:00   no
```

`--json` emits one JSON object per line in the same field order as the YAML schema.

Exit codes: 0 on success, 1 on state file read error.

## Architecture

### New files

**`lib/src/state/mod.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct StateEntry {
    pub manifest: String,
    pub action: String,
    pub key: String,
    pub applied_at: DateTime<Utc>,
    pub sha256: Option<String>,
    pub changed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub schema_version: u32,
    pub last_apply: DateTime<Utc>,
    pub atoms: Vec<StateEntry>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            schema_version: 1,
            last_apply: Utc::now(),
            atoms: Vec::new(),
        }
    }
}

pub struct StateStore {
    path: PathBuf,
}

impl StateStore {
    /// Resolves `dirs_next::data_local_dir()/etch/state.yaml`.
    /// Falls back to `~/.local/share/etch/state.yaml` if dirs_next returns None.
    pub fn new() -> Self { ... }

    /// Returns empty State if file is missing; treats corrupt YAML as empty.
    pub fn load(&self) -> anyhow::Result<State> { ... }

    /// Atomic write: serialize to `.state.yaml.tmp`, then `fs::rename`.
    pub fn save(&self, state: &State) -> anyhow::Result<()> { ... }

    /// Load, merge entries (replace same manifest+action+key triple), save.
    /// Updates `last_apply` to now. Never appends duplicates.
    pub fn record(&self, entries: Vec<StateEntry>) -> anyhow::Result<()> { ... }
}
```

**`lib/src/state/` is a new module** — add `pub mod state;` to `lib/src/lib.rs`.

### Modified files

**`lib/Cargo.toml`** — add `dirs-next = "2"`, `chrono = { version = "0.4", features = ["serde"] }`

**`lib/src/actions/<action>/mod.rs`** (all actions that produce atoms) — add a `state_key(&self) -> String` method returning the canonical identifier for that action type:

| Action type          | `state_key` value                   |
| -------------------- | ----------------------------------- |
| `file.copy`          | destination path (`to`/`target`)    |
| `file.link`          | symlink target path                 |
| `file.chmod`         | `path`                              |
| `file.chown`         | `path`                              |
| `file.flags`         | `path`                              |
| `directory.create`   | `path`                              |
| `directory.copy`     | destination path (`to`)             |
| `command.run`        | `command` joined with first arg     |
| `git.clone`          | `directory`                         |
| `git.pull`           | `directory`                         |
| `git.config`         | `key` (or `"bulk"` for `settings:`) |
| `package.install`    | `name` or first entry of `list`     |
| `package.repository` | `name`                              |
| `brew.bundle`        | `file`                              |
| `brew.upgrade`       | `"brew.upgrade"`                    |
| `brew.cleanup`       | `"brew.cleanup"`                    |
| `mas.install`        | `id` as string                      |
| `mas.upgrade`        | `id` as string, or `"all"`          |
| `macos.defaults`     | `"<domain>/<key>"`                  |
| `macos.service`      | `plist`                             |
| `systemd.service`    | `unit`                              |
| `binary.github`      | `name`                              |
| `binary.url`         | `name`                              |

**`app/src/commands/apply.rs`** — after all atoms succeed, build `Vec<StateEntry>` from the executed steps, call `StateStore::new().record(entries)`. Log a warning and continue if `record()` returns an error. Atoms that ran but produced no change are recorded with `changed: false`; atoms that were skipped entirely (plan returned empty) are not recorded.

**`app/src/commands/mod.rs`** — register `history` subcommand

**`app/src/commands/history.rs`** — new file; reads `StateStore::new().load()`, formats as table or NDJSON

**`app/src/config/mod.rs`** — add `History(HistoryArgs)` variant to `Commands` enum

```rust
#[derive(clap::Args, Debug)]
pub struct HistoryArgs {
    /// Filter entries by manifest path (substring match)
    #[arg(long)]
    pub manifest: Option<String>,
    /// Output as NDJSON
    #[arg(long)]
    pub json: bool,
}
```

### SHA-256 for file atoms

`lib/src/atoms/file/copy.rs` — after writing the destination file, compute SHA-256 of the destination bytes using `sha2::Digest`. Return the hex string as part of the atom's execute result.

Because the atom trait currently returns `anyhow::Result<bool>`, the SHA-256 is passed back to the action layer by storing it on the atom struct after execution (mutate a `pub sha256: Option<String>` field on `FileCopy` atom), then read by the caller in `apply.rs`.

**`lib/Cargo.toml`** — add `sha2 = "0.10"`

## Error Handling

| Condition                              | Behavior                                                                  |
| -------------------------------------- | ------------------------------------------------------------------------- |
| `data_local_dir()` returns `None`      | Fall back to `~/.local/share/etch/state.yaml`; log at debug level         |
| State directory not creatable          | Log warning: `"state: cannot create directory, skipping"`; apply succeeds |
| State file missing on load             | Treat as empty `State::default()`                                         |
| State file present but invalid YAML    | Log warning: `"state: corrupt file, resetting"`; treat as empty state     |
| Atomic rename fails (cross-device)     | Log warning with error; apply succeeds                                    |
| Partial apply (some atoms failed)      | Record only the atoms that succeeded; do not record failed atoms          |
| `etch history` — state file missing    | Print empty table with header; exit 0                                     |
| `etch history` — state file unreadable | Print error to stderr; exit 1                                             |
| `--manifest` filter matches nothing    | Print empty table with header; exit 0                                     |

State write is always best-effort — a state failure never blocks `etch apply`.

## Testing

**Unit tests (`lib/src/state/mod.rs`):**

- `save()` → `load()` roundtrip: write a `State`, load it back, assert all fields equal
- Atomic write: after `save()`, the `.state.yaml.tmp` file does not exist
- `record()` merge semantics: given two entries with the same `(manifest, action, key)` triple, a second `record()` call updates the row rather than appending; final `atoms` length is 1
- `record()` append semantics: entries with distinct triples accumulate; final `atoms` length equals number of distinct triples
- Missing file: `load()` on a non-existent path returns `State::default()`
- Corrupt file: `load()` on a file containing `"not: valid: yaml: {{{"` returns `State::default()` (no error propagated)
- `schema_version` roundtrips as `1`
- `last_apply` is updated to approximately now on each `record()` call

**Unit tests (`app/src/commands/history.rs`):**

- Table formatter: given a `Vec<StateEntry>`, output contains each key value and `yes`/`no` for `changed`
- `--manifest` filter: only matching rows appear in output
- `--json` flag: each line is valid JSON with the expected keys

**Integration tests (`app/tests/integration.rs`):**

- Run `etch apply` with a `file.copy` manifest → assert `~/.local/share/etch/state.yaml` exists and contains the destination path as `key`
- Run `etch apply` a second time (idempotent) → `atoms` length is still 1 (merge, not append); `changed: false`
- Run `etch history` → stdout contains the destination path
- Run `etch history --manifest <manifest-path>` → only the matching entry appears
- Run `etch history --json` → first line parses as JSON with `action` field set to `"file.copy"`
