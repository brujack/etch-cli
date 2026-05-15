# file.chmod Action — Design Spec

**Date:** 2026-05-15
**Status:** Approved

## Context

The dotfiles Phase 2 migration surfaces `chmod 700` as a recurring pattern (four credential directories: `.ssh`, `.warp`, `.tf_creds`, `.tsh`). Currently, `file.chmod` is missing as a declarative action — the workaround is `command.run: chmod 700 <dir>`. The `Chmod` atom (`lib/src/atoms/file/chmod.rs`) already exists and is fully implemented. This spec defines only the action wrapper.

The `privileged` gap for other file actions (`file.link`, `file.copy`, `file.chown`) is tracked separately in the backlog.

## Scope

**Create:** `lib/src/actions/file/chmod.rs`

**Modify:**

- `lib/src/actions/file/mod.rs` — add `pub mod chmod;`
- `lib/src/actions/mod.rs` — add `FileChmod` variant + `inner_ref` arm

**No changes to:**

- `lib/src/atoms/file/chmod.rs` — atom is complete
- `lib/src/atoms/command/exec.rs` — used as-is for privileged path

## YAML Interface

```yaml
- action: file.chmod
  path: "{{ user.home_dir }}/.ssh"
  mode: "700"
  # privileged: true   # optional, default false; alias: sudo
```

`path` is a Tera-rendered string (rendering happens at manifest load time before `plan()` is called). `mode` is a string parsed as octal in `plan()`. `privileged` defaults to `false`; accepts `sudo: true` as an alias (matching `command.run` convention).

## Struct Definition

```rust
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct FileChmod {
    pub path: String,
    pub mode: String,
    #[serde(default = "get_false", alias = "sudo")]
    pub privileged: bool,
}
```

`get_false` is the same helper already used by `command/run.rs`.

## plan() Logic

**Non-privileged (`privileged: false`):**

1. Strip optional `0o` or leading `0` prefix from `mode` string
2. Parse remaining digits as octal `u32` — return `Err` if invalid
3. Return `vec![Step { atom: Box::new(Chmod { path: PathBuf::from(&self.path), mode }) }]`

**Privileged (`privileged: true`):**

1. Extract the privilege provider from `Contexts` (same pattern as `command/run.rs`)
2. Return `vec![Step { atom: Box::new(Exec { command: "chmod".into(), arguments: vec![self.mode.clone(), self.path.clone()], privileged: true, privilege_provider, ..Default::default() }) }]`

No initializers or finalizers on either step — consistent with `file.chown`.

`summarize()` returns: `format!("Set permissions {} on {}", self.mode, self.path)`

## Mode String Parsing

Accept any of: `"700"`, `"0700"`, `"0o700"`. Algorithm:

1. If string starts with `"0o"`, strip those two chars.
2. Parse the result with `u32::from_str_radix(s, 8)`.

`u32::from_str_radix` accepts leading zeros natively, so `"0700"` → `0o700` without extra stripping. Return `anyhow::Err("invalid mode: ...")` if parsing fails.

## Registration

**`lib/src/actions/file/mod.rs`** — add after existing module declarations:

```rust
pub mod chmod;
```

**`lib/src/actions/mod.rs`** — import:

```rust
use file::chmod::FileChmod;
```

Enum variant (alphabetical with other `File*` variants):

```rust
#[serde(rename = "file.chmod")]
FileChmod(ConditionalVariantAction<FileChmod>),
```

`inner_ref` match arm:

```rust
Actions::FileChmod(a) => a,
```

## Testing

Five tests in `lib/src/actions/file/chmod.rs`:

| Test                                     | What it verifies                                                                        |
| ---------------------------------------- | --------------------------------------------------------------------------------------- |
| `it_can_be_deserialized`                 | YAML `file.chmod` deserializes to correct struct (path, mode, privileged default false) |
| `plan_returns_chmod_step`                | Non-privileged → 1 step with `Chmod` atom                                               |
| `plan_errors_on_invalid_mode`            | `mode: "xyz"` → `plan()` returns `Err`                                                  |
| `plan_returns_exec_step_when_privileged` | `privileged: true` → 1 step (verify it's an `Exec` via `output_string` or Display)      |
| `summarize_includes_path_and_mode`       | `summarize()` contains path and mode string                                             |

No changes to `atoms/file/chmod.rs` — existing 4 atom tests remain intact.
