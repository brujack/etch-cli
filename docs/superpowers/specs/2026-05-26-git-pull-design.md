# git.pull Action Design

## Overview

Add a `git.pull` action that ensures a git repository is present and current. If the target directory does not exist, it clones the repository. If the directory already exists, it pulls the latest changes. This replaces the common pattern of using `command.run` with `git clone` or `git pull`.

## Manifest Interface

```yaml
- action: git.pull
  repo_url: https://github.com/example/dotfiles.git
  directory: /home/user/git-repos/personal/dotfiles
```

### Fields

| Field       | Type   | Required | Description                                     |
| ----------- | ------ | -------- | ----------------------------------------------- |
| `repo_url`  | String | Yes      | Remote URL to clone from (if directory missing) |
| `directory` | String | Yes      | Local path to the repo checkout                 |

No `rebase` field — pull behavior (merge vs rebase vs fast-forward-only) is deferred to the repo's git config and the user's global git config.

## Behavior

### `plan()`

1. Parse `repo_url` with `gix::url::parse` — return `Err` immediately if invalid.
2. Return one `Step` wrapping a `Pull` atom. The atom's `plan()` returns `should_run: true` unconditionally — whether a pull is needed cannot be determined without a network round-trip, so always execute.

### `execute()`

Delegates to the `Pull` atom, which spawns the system `git` binary:

- If `directory` does not exist: `git clone <repo_url> <directory>`
- If `directory` exists: `git -C <directory> pull`

Both use `std::process::Command`. Non-zero exit code propagates as an `anyhow::Error`.

### Idempotency

Running twice is safe:

- First run on a missing directory: clones the repo.
- Second run: pulls (no-op if already up to date, fast-forward otherwise).
- First run on an existing directory: pulls.

### Error Cases

| Condition                                             | Behavior                                              |
| ----------------------------------------------------- | ----------------------------------------------------- |
| Invalid `repo_url`                                    | `plan()` returns `Err`                                |
| `git clone` fails (network, auth, bad URL at runtime) | `execute()` returns `Err`                             |
| `directory` exists but is not a git repo              | `git pull` exits non-zero → `execute()` returns `Err` |

## Implementation

### New Files

- `lib/src/actions/git/pull.rs` — `GitPull` struct + `Action` impl
- `lib/src/atoms/git/pull.rs` — `Pull` atom + `Atom` impl

### Modified Files

- `lib/src/actions/git/mod.rs` — add `pub mod pull; pub use pull::GitPull;`
- `lib/src/actions/mod.rs` — add `GitPull` to `use git::{...}`, `Actions` enum, and all match arms
- `lib/src/atoms/git/mod.rs` — add `mod pull; pub use pull::Pull;`

### Action Struct

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitPull {
    pub repo_url: String,
    pub directory: String,
}
```

### Atom Struct

```rust
pub struct Pull {
    pub repository: String,
    pub directory: PathBuf,
}
```

`execute()` logic:

```rust
fn execute(&mut self) -> anyhow::Result<()> {
    if self.directory.exists() {
        let status = std::process::Command::new("git")
            .args(["-C", &self.directory.to_string_lossy(), "pull"])
            .status()?;
        if !status.success() {
            anyhow::bail!("git pull failed in {}", self.directory.display());
        }
    } else {
        let status = std::process::Command::new("git")
            .args(["clone", &self.repository, &self.directory.to_string_lossy()])
            .status()?;
        if !status.success() {
            anyhow::bail!("git clone failed for {}", self.repository);
        }
    }
    Ok(())
}
```

## Tests

### `actions/git/pull.rs`

| Test                           | Verifies                                         |
| ------------------------------ | ------------------------------------------------ |
| `it_can_be_deserialized`       | YAML round-trip; both fields populate correctly  |
| `plan_always_returns_one_step` | `should_run: true` regardless of directory state |
| `plan_errors_on_invalid_url`   | Invalid URL → `plan()` returns `Err`             |

### `atoms/git/pull.rs`

| Test                                    | Verifies                                                    |
| --------------------------------------- | ----------------------------------------------------------- |
| `display_format`                        | Display string includes repo URL and directory              |
| `plan_always_should_run`                | `should_run: true`                                          |
| `execute_clones_when_directory_missing` | Spawns `git clone <url> <dir>` when dir absent (mock `git`) |
| `execute_pulls_when_directory_exists`   | Spawns `git -C <dir> pull` when dir present (mock `git`)    |
| `execute_propagates_clone_failure`      | Non-zero exit from clone → `Err`                            |
| `execute_propagates_pull_failure`       | Non-zero exit from pull → `Err`                             |

Mock pattern: PATH-injected `git` script that records args to a temp file and exits with a configurable code.
