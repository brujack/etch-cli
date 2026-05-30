# Drift Detection Design (`etch status`)

## Overview

Add a working `etch status` command that compares the declared state in manifests against the current state of the machine and reports any drift. Each atom reports its own status via a new `status()` method on the `Atom` trait. Output is a colored table; exit code is 0 when everything matches, 1 when any drift is detected.

## Motivation

`etch apply` is idempotent but silent: running it after an accidental manual change silently re-applies. There is no way to answer "is my machine in the declared state?" without running apply and observing what changed.

`etch status` fills that gap. It reads manifests, calls each atom's `status()` without touching the filesystem, and reports discrepancies. This enables:

- Pre-apply review: see what is drifted before changing anything
- CI/scripting: `etch status && echo "machine clean"` — exit 1 on drift
- Foundation for future rollback: a drift report is the input to a rollback plan

Note: `Status(commands::Apply)` already exists as a stub in `app/src/config/mod.rs` with a placeholder `status()` body in `Apply`. This spec replaces that stub with a full implementation.

## CLI Invocation

```
# Check all configured manifests
etch status

# Scope to a single manifest by name
etch status my-dotfiles

# Machine-readable JSON output
etch status --json

# Scope + JSON
etch status my-dotfiles --json
```

`etch status` reuses the existing `Apply` struct's `manifests: Vec<String>` positional argument for scoping. The `--json` flag is new.

### Example output (terminal)

```
manifest: dotfiles
  file.link  ~/.zshrc                  ✓ ok
  file.link  ~/.gitconfig               ✗ drifted
                                         expected → /home/bruce/dotfiles/gitconfig
                                         actual   → /home/bruce/old/gitconfig
  file.copy  ~/.config/starship.toml   ✗ missing
  file.chmod ~/.ssh/id_ed25519          ✓ ok
  dir.create ~/bin                      ✓ ok

manifest: packages
  package    git                        ✓ ok (unchecked)
  package    ripgrep                    ✓ ok (unchecked)

Summary: 5 ok (1 unchecked), 1 drifted, 1 missing
Exit code: 1
```

### Example output (--json)

```json
{
    "manifests": [
        {
            "name": "dotfiles",
            "atoms": [
                {
                    "label": "file.link ~/.zshrc",
                    "status": "ok"
                },
                {
                    "label": "file.link ~/.gitconfig",
                    "status": "drifted",
                    "expected": "/home/bruce/dotfiles/gitconfig",
                    "actual": "/home/bruce/old/gitconfig"
                },
                {
                    "label": "file.copy ~/.config/starship.toml",
                    "status": "missing"
                }
            ]
        }
    ],
    "summary": {
        "ok": 5,
        "unchecked": 1,
        "drifted": 1,
        "missing": 1
    }
}
```

## Architecture

### New files

**`lib/src/atoms/status.rs`**

Defines `AtomStatus` and the `status()` trait method default.

```rust
/// The result of checking one atom against the current machine state.
#[derive(Debug, Clone, PartialEq)]
pub enum AtomStatus {
    /// Declared state matches actual state.
    Ok,
    /// Atom has never been applied (target does not exist).
    Missing,
    /// Declared state differs from actual state.
    Drifted {
        expected: String,
        actual: String,
    },
    /// This atom type cannot be checked (e.g. command.run, http.request).
    /// Reports ok in the exit-code sense; surfaced with dim styling.
    Unchecked,
}
```

`AtomStatus` derives `serde::Serialize` for `--json` output. The JSON tag for `Drifted` flattens to `{"status":"drifted","expected":"…","actual":"…"}`.

**`app/src/commands/status.rs`**

Drives the `etch status` workflow and owns the output formatter.

```rust
/// Per-atom result row collected during a status run.
pub struct AtomResult {
    pub label: String,
    pub status: AtomStatus,
}

/// Per-manifest result collected during a status run.
pub struct ManifestResult {
    pub name: String,
    pub atoms: Vec<AtomResult>,
}
```

The status runner:

1. Loads manifests via `load(manifest_path, contexts)?` (same as `apply.rs`)
2. For each manifest, for each action, calls `action.plan(manifest, contexts)?` → `Vec<Step>`
3. For each `Step`, calls `step.atom.status()?` and collects an `AtomResult`
4. Does **not** call `step.atom.execute()` — read-only
5. Renders output or serializes JSON, then returns `Ok(())` if all atoms are `Ok`/`Unchecked`, or `Err(anyhow!("drift detected"))` which maps to exit code 1

**`app/src/commands/status.rs`** also implements `EtchCommand` via the existing `Apply::status()` bridge:

```rust
impl Apply {
    pub fn status(&self, runtime: &Runtime) -> anyhow::Result<()> {
        // self.json controls --json flag (new field on Apply)
        run_status(self, runtime)
    }
}
```

### Modified files

**`lib/src/atoms/mod.rs`**

Add `pub mod status;` and add `status()` to the `Atom` trait with a default implementation that returns `AtomStatus::Unchecked`:

```rust
use crate::atoms::status::AtomStatus;

pub trait Atom: std::fmt::Display {
    fn plan(&self) -> anyhow::Result<Outcome>;
    fn execute(&mut self) -> anyhow::Result<()>;

    /// Check whether this atom's declared state matches the current machine state.
    /// Default: Unchecked (used for atoms whose state cannot be read, e.g. command.run).
    fn status(&self) -> anyhow::Result<AtomStatus> {
        Ok(AtomStatus::Unchecked)
    }

    fn output_string(&self) -> String { String::from("") }
    fn error_message(&self) -> String { String::from("") }
    fn status_code(&self) -> i32 { 0 }
}
```

**`lib/src/atoms/file/link.rs`** — add `status()`:

```rust
fn status(&self) -> anyhow::Result<AtomStatus> {
    match std::fs::read_link(&self.target) {
        Ok(actual) if actual == self.source => Ok(AtomStatus::Ok),
        Ok(actual) => Ok(AtomStatus::Drifted {
            expected: self.source.display().to_string(),
            actual: actual.display().to_string(),
        }),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(AtomStatus::Missing),
        Err(e) => Err(anyhow::Error::from(e)
            .context(format!("status: cannot read symlink {}", self.target.display()))),
    }
}
```

**`lib/src/atoms/file/copy.rs`** — add `status()`:

Check whether `self.to` exists and whether its SHA-256 matches `self.from`.

```rust
fn status(&self) -> anyhow::Result<AtomStatus> {
    if !self.to.exists() {
        return Ok(AtomStatus::Missing);
    }
    let src_hash = sha256_file(&self.from)
        .with_context(|| format!("status: cannot hash source {}", self.from.display()))?;
    let dst_hash = sha256_file(&self.to)
        .with_context(|| format!("status: cannot hash target {}", self.to.display()))?;
    if src_hash == dst_hash {
        Ok(AtomStatus::Ok)
    } else {
        Ok(AtomStatus::Drifted {
            expected: src_hash,
            actual: dst_hash,
        })
    }
}

fn sha256_file(path: &Path) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}
```

Add `sha2 = "0.10"` to `lib/Cargo.toml`. Add `sha2::Digest` import guarded under the `status()` implementation only (not the `execute()` path) to avoid pulling the dependency into the hot path.

**`lib/src/atoms/file/chmod.rs`** — add `status()`:

```rust
fn status(&self) -> anyhow::Result<AtomStatus> {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(&self.path) {
        Ok(meta) => {
            let actual = meta.permissions().mode() & 0o7777;
            if actual == self.mode {
                Ok(AtomStatus::Ok)
            } else {
                Ok(AtomStatus::Drifted {
                    expected: format!("{:04o}", self.mode),
                    actual: format!("{:04o}", actual),
                })
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(AtomStatus::Missing),
        Err(e) => Err(anyhow::Error::from(e)
            .context(format!("status: cannot stat {}", self.path.display()))),
    }
}
```

**`lib/src/atoms/directory/create.rs`** — add `status()`:

```rust
fn status(&self) -> anyhow::Result<AtomStatus> {
    match std::fs::metadata(&self.path) {
        Ok(meta) if meta.is_dir() => Ok(AtomStatus::Ok),
        Ok(_) => Ok(AtomStatus::Drifted {
            expected: "directory".to_string(),
            actual: "file".to_string(),
        }),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(AtomStatus::Missing),
        Err(e) => Err(anyhow::Error::from(e)
            .context(format!("status: cannot stat {}", self.path.display()))),
    }
}
```

**`app/src/commands/apply.rs`** — replace the placeholder `status()` stub body with `run_status(self, runtime)`.

**`app/src/commands/apply.rs`** — add `--json` flag to the `Apply` struct:

```rust
#[derive(Parser, Debug)]
pub(crate) struct Apply {
    /// Run a subset of your manifests, comma separated list.
    manifests: Vec<String>,

    /// Output status as JSON (status subcommand only)
    #[arg(long)]
    pub json: bool,

    // ... existing fields
}
```

**`app/src/commands/mod.rs`** — add `pub mod status; pub(crate) use status::run_status;`

**`lib/Cargo.toml`** — add `sha2 = "0.10"`

**`lib/src/atoms/file/mod.rs`** — no change (copy.rs and chmod.rs are already declared)

## Atom Status Coverage

| Atom                    | `status()` impl | Returns                                      |
| ----------------------- | --------------- | -------------------------------------------- |
| `file::Link`            | Yes             | Ok / Drifted (wrong target) / Missing        |
| `file::Copy`            | Yes             | Ok / Drifted (sha256 mismatch) / Missing     |
| `file::Chmod`           | Yes             | Ok / Drifted (mode mismatch) / Missing       |
| `directory::Create`     | Yes             | Ok / Drifted (not a dir) / Missing           |
| `package::*` (all)      | No — default    | Unchecked (version-pinning spec deferred)    |
| `command::Exec`         | No — default    | Unchecked (no stable observable side effect) |
| `http::*`               | No — default    | Unchecked (network; no local state to read)  |
| `git::*`                | No — default    | Unchecked (deferred to git-status spec)      |
| `file::Chflags` (macOS) | No — default    | Unchecked (deferred to flags-status spec)    |
| `file::Chmod` (chown)   | No — default    | Unchecked (separate chown atom)              |
| `macos::Defaults`       | No — default    | Unchecked (deferred)                         |
| `systemd::Service`      | No — default    | Unchecked (deferred)                         |
| `macos::Service`        | No — default    | Unchecked (deferred)                         |
| `binary::*`             | No — default    | Unchecked (deferred)                         |

Atoms that don't implement `status()` use the trait default (`Unchecked`). They appear in the output with dim styling and do not contribute to the drift count.

## Error Handling

| Condition                               | Behavior                                                           |
| --------------------------------------- | ------------------------------------------------------------------ |
| Source file not readable (copy.rs)      | `status()` returns `Err` with source path in context               |
| Target not readable (permission denied) | `status()` returns `Err` with target path + OS error               |
| Target does not exist                   | `AtomStatus::Missing`                                              |
| Target exists but is wrong type         | `AtomStatus::Drifted { expected, actual }` (e.g. file vs dir)      |
| Atom type has no `status()` impl        | Trait default: `AtomStatus::Unchecked` — never errors              |
| `plan()` fails for an action            | Status runner skips that action, logs a warning, continues         |
| All atoms `Ok` or `Unchecked`           | Exit 0                                                             |
| Any atom `Drifted` or `Missing`         | Exit 1 (via `Err(anyhow!("drift detected"))`)                      |
| `--json` + terminal color conflict      | JSON output is always plain (no ANSI codes regardless of no_color) |

## Testing

### Unit tests — `lib/src/atoms/file/link.rs`

- Target does not exist → `Missing`
- Target is a symlink pointing to the declared source → `Ok`
- Target is a symlink pointing elsewhere → `Drifted` with correct expected/actual paths
- `read_link` fails with permission denied → propagated `Err`

### Unit tests — `lib/src/atoms/file/copy.rs`

- Target does not exist → `Missing`
- Target exists, SHA-256 matches source → `Ok`
- Target exists, SHA-256 differs → `Drifted` with hex hashes
- Source not readable → propagated `Err`
- Target not readable → propagated `Err`

### Unit tests — `lib/src/atoms/file/chmod.rs`

- Target does not exist → `Missing`
- Actual permissions match declared mode → `Ok`
- Actual permissions differ → `Drifted` with formatted octal strings
- `metadata()` fails with permission denied → propagated `Err`

### Unit tests — `lib/src/atoms/directory/create.rs`

- Path does not exist → `Missing`
- Path exists and is a directory → `Ok`
- Path exists and is a regular file → `Drifted { expected: "directory", actual: "file" }`

### Unit tests — `lib/src/atoms/mod.rs`

- `Echo` (default impl) returns `AtomStatus::Unchecked` — verifies the trait default

### Unit tests — `lib/src/atoms/status.rs`

- `AtomStatus` serializes to expected JSON shapes (serde_json round-trip)
- `AtomStatus::Drifted` serializes to `{"status":"drifted","expected":"…","actual":"…"}`

### Integration tests — `app/tests/integration.rs`

**Scenario: all clean**

1. Apply a manifest containing `file.link`, `file.copy`, `directory.create` into a tempdir
2. Run `etch status` against that manifest
3. Assert exit code 0
4. Assert stdout contains "✓ ok" for each atom

**Scenario: drifted symlink**

1. Apply a manifest with one `file.link`
2. Overwrite the symlink to point at a different target (`std::os::unix::fs::symlink`)
3. Run `etch status`
4. Assert exit code 1
5. Assert stdout contains "✗ drifted" and the expected/actual paths

**Scenario: missing file**

1. Apply a manifest with `file.copy`
2. Delete the copied target
3. Run `etch status`
4. Assert exit code 1
5. Assert stdout contains "✗ missing"

**Scenario: --json output**

1. Apply, then drift one symlink
2. Run `etch status --json`
3. Parse the JSON output
4. Assert `summary.drifted == 1` and the drifted atom has `"status": "drifted"` with non-empty `expected`/`actual`

**Scenario: unchecked atoms**

1. Apply a manifest containing only `command.run` actions
2. Run `etch status`
3. Assert exit code 0 (unchecked atoms do not cause drift)
4. Assert stdout contains "ok (unchecked)"

## Dependencies

- `sha2 = "0.10"` — SHA-256 for `file::Copy` status check (add to `lib/Cargo.toml`)
- `serde_json` — already in workspace (for `--json` serialization)
- No other new crates required

## Prerequisite for Future Features

- **State manifest:** `etch status --json > state.json` produces the input for a future `etch rollback` command
- **Scheduled drift alerts:** `etch status --json` output is machine-readable for cron/systemd alerting
- **File rollback:** the drifted atom list from `etch status` maps directly onto atoms to re-execute in a rollback
