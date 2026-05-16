# File Action Privileged Support — Design Spec

**Date:** 2026-05-15
**Status:** Approved

## Context

`file.chmod` gained `privileged: bool` support in PR #21. The remaining file actions (`file.chown`, `file.link`, `file.copy`) still have no sudo escalation path. Adding the field ad-hoc to each action doesn't enforce the pattern for future actions.

This spec adds privileged support to the three remaining actions and introduces a shared `FileActionConfig` struct embedded in all file actions so the compiler enforces the pattern going forward.

## Scope

**Modify:**

- `lib/src/actions/file/mod.rs` — add `FileActionConfig`, `get_false`, update `FileAction` trait
- `lib/src/actions/file/chmod.rs` — refactor to embed `FileActionConfig` (small change)
- `lib/src/actions/file/chown.rs` — add config, implement trait, add privileged path
- `lib/src/actions/file/link.rs` — add config, implement trait, add privileged path
- `lib/src/actions/file/copy.rs` — add config, implement trait, add privileged path
- `lib/src/actions/file/remove.rs` — add config, implement trait, return `Err` if privileged
- `lib/src/actions/file/download.rs` — add config, implement trait, return `Err` if privileged
- `lib/src/actions/file/unarchive.rs` — add config, implement trait, return `Err` if privileged

## Architecture: `FileActionConfig` + Required Trait Method

### Shared struct (in `lib/src/actions/file/mod.rs`)

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileActionConfig {
    #[serde(default = "get_false", alias = "sudo")]
    pub privileged: bool,
}

fn get_false() -> bool {
    false
}
```

### `FileAction` trait update

```rust
pub trait FileAction: Action {
    fn file_action_config(&self) -> &FileActionConfig;  // required — compiler-enforced

    fn resolve(&self, manifest: &Manifest, path: &str) -> anyhow::Result<PathBuf> { /* unchanged */ }
    fn load(&self, manifest: &Manifest, path: &str) -> Result<Vec<u8>> { /* unchanged */ }
}
```

The `file_action_config()` method is **required** — any new file action that forgets to embed `FileActionConfig` and implement the method will fail to compile.

### Embedding pattern (all file actions)

```rust
pub struct FileChmod {
    pub path: String,
    pub mode: String,
    #[serde(flatten)]
    pub config: FileActionConfig,
}

impl FileAction for FileChmod {
    fn file_action_config(&self) -> &FileActionConfig {
        &self.config
    }
}
```

`self.privileged` references in `file.chmod` become `self.config.privileged`.

### Actions that don't yet support privileged (`remove`, `download`, `unarchive`)

Embed `FileActionConfig`, implement `file_action_config()`, and add this guard at the top of `plan()`:

```rust
if self.config.privileged {
    return Err(anyhow!("file.X does not support privileged mode"));
}
```

Fail fast rather than silently ignoring the flag.

## `file.chown` Privileged Behavior

Ownership string follows POSIX `chown` conventions:

- Both user and group: `"user:group"`
- User only: `"user"`
- Group only: `":group"`
- Neither: return empty steps (same as non-privileged)

```
Step 1: Exec { command: "chown", args: ["user:group", path], privileged: true }
```

## `file.link` Privileged Behavior

### Non-walk (`walk_dir: false`)

```
Step 1: Exec { command: "mkdir", args: ["-p", parent_of_target], privileged: true }
Step 2: Exec { command: "ln", args: ["-sf", source, target], privileged: true }
```

### Walk (`walk_dir: true`)

Directory walking happens at plan time (reading source dir contents — no privileges needed). For each item found:

```
Step 2k-1: Exec { command: "mkdir", args: ["-p", parent_of_item_target], privileged: true }
Step 2k:   Exec { command: "ln", args: ["-sf", source_item, target_item], privileged: true }
```

Total steps = 2 × N items.

## `file.copy` Privileged Behavior

### Non-template (`template: false`)

```
Step 1: Exec { command: "mkdir", args: ["-p", parent], privileged: true }
Step 2: Exec { command: "cp", args: [source, dest], privileged: true }
Step 3: Exec { command: "chmod", args: [format!("{:o}", self.chmod)  ← chmod field is u32, format as octal string, dest], privileged: true }
Step 4: Exec { command: "chown", args: ["user:group", dest], privileged: true }  ← only if owner_user AND owner_group specified
```

### Template (`template: true`)

Content is Tera-rendered in memory at plan time (unprivileged). Written to a deterministic tempfile, then moved:

```
Step 1: SetContents { path: /tmp/etch-<sha256-hex-of-dest>, contents: rendered_bytes }
Step 2: Exec { command: "mkdir", args: ["-p", parent], privileged: true }
Step 3: Exec { command: "cp", args: [/tmp/etch-<hash>, dest], privileged: true }
Step 4: Exec { command: "chmod", args: [format!("{:o}", self.chmod)  ← chmod field is u32, format as octal string, dest], privileged: true }
Step 5: Exec { command: "chown", args: ["user:group", dest], privileged: true }  ← optional
Step 6: Exec { command: "rm", args: [/tmp/etch-<hash>], privileged: false }
```

Tempfile path: a deterministic path derived from the destination path, e.g. `std::env::temp_dir().join(format!("etch-{}", sanitised_dest))` where `sanitised_dest` replaces path separators with `-`. Exact derivation is an implementation detail; the path must be unique per destination and stable across plan/execute.

**Passphrase-encrypted + privileged:** decrypt content to the same tempfile path (using the existing `Decrypt` atom logic), then follow the same sudo-cp chain from Step 2.

**Error handling:** if template rendering or content loading fails at plan time, `plan()` returns `Err` — identical to the non-privileged path, no new error cases.

## Testing

### `file.chmod` (refactor verification)

All 6 existing tests remain passing after `self.privileged` → `self.config.privileged` rename. No new tests.

### `file.chown` new tests

| Test                                     | Verifies                                                       |
| ---------------------------------------- | -------------------------------------------------------------- |
| `it_can_be_deserialized_with_privileged` | `privileged: true` and `sudo: true` both deserialize correctly |
| `plan_returns_exec_step_when_privileged` | 1 Exec step with correct chown args                            |
| `plan_privileged_with_group_only`        | ownership string is `":group"`                                 |
| `plan_still_works_when_not_privileged`   | existing non-privileged behavior unchanged                     |

### `file.link` new tests

| Test                                      | Verifies                                   |
| ----------------------------------------- | ------------------------------------------ |
| `it_can_be_deserialized_with_privileged`  | `privileged: true` deserializes            |
| `plan_returns_exec_steps_when_privileged` | 2 Exec steps: mkdir + ln -sf               |
| `plan_still_works_when_not_privileged`    | existing non-privileged behavior unchanged |

### `file.copy` new tests

| Test                                                          | Verifies                                                         |
| ------------------------------------------------------------- | ---------------------------------------------------------------- |
| `it_can_be_deserialized_with_privileged`                      | `privileged: true` deserializes                                  |
| `plan_returns_exec_steps_when_privileged_no_template`         | mkdir + cp + chmod Exec steps                                    |
| `plan_returns_setcontents_then_exec_when_privileged_template` | SetContents step uses `/tmp/etch-*` path; followed by Exec steps |
| `plan_still_works_when_not_privileged`                        | existing non-privileged behavior unchanged                       |

### `file.remove`, `file.download`, `file.unarchive`

| Test                                        | Verifies                                       |
| ------------------------------------------- | ---------------------------------------------- |
| `plan_errors_when_privileged_not_supported` | `plan()` returns `Err` when `privileged: true` |

## YAML Interface

All three actions gain the same new optional field:

```yaml
- action: file.chown
  path: /usr/local/bin/mytool
  user: root
  group: root
  privileged: true # or: sudo: true

- action: file.link
  source: /opt/myapp/bin/mytool
  target: /usr/local/bin/mytool
  privileged: true

- action: file.copy
  from: ./configs/nginx.conf
  to: /etc/nginx/nginx.conf
  chmod: "644"
  privileged: true
```

## What Is NOT in Scope

- `file.remove`, `file.download`, `file.unarchive` privileged support (returns error; future work)
- `file.copy` with both `template: true` and `passphrase` + `privileged: true` — passphrase decrypt is handled identically to the template path via the same temp-file approach
- `file.link walk_dir` idempotency improvements (out of scope)
