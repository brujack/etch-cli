# terraform.tfenv Action — Design Spec

## Overview

Add a `terraform.tfenv` action that installs the tfenv Terraform version manager via git clone, optionally installs a specific Terraform version, and sets it as the global default.

## Motivation

`setup_env.sh` installs tfenv manually (git clone + symlinks). No etch-cli action exists. Users must shell out via `command.run`, which doesn't signal intent or handle idempotency.

## Design

### Action struct

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "terraform.tfenv")]
pub struct TerraformTfenv {
    /// Terraform version to install and set as default (e.g. "1.9.0").
    /// If omitted, only tfenv itself is installed.
    pub version: Option<String>,
}
```

### Plan output

**No `version` field → 1 step:**

| Step | Command                                                   | SkipIf                 |
| ---- | --------------------------------------------------------- | ---------------------- |
| 1    | `git clone https://github.com/tfutils/tfenv.git ~/.tfenv` | `FileExists(~/.tfenv)` |

**`version: "X.Y.Z"` → 3 steps:**

| Step | Command                                                   | SkipIf                                            |
| ---- | --------------------------------------------------------- | ------------------------------------------------- |
| 1    | `git clone https://github.com/tfutils/tfenv.git ~/.tfenv` | `FileExists(~/.tfenv)`                            |
| 2    | `~/.tfenv/bin/tfenv install X.Y.Z`                        | `FileExists(~/.tfenv/versions/X.Y.Z)`             |
| 3    | `~/.tfenv/bin/tfenv use X.Y.Z`                            | none — idempotent (writes `~/.terraform-version`) |

### Tilde expansion

All paths use `shellexpand::tilde(...).into_owned()` — same pattern as `ruby.chruby`. Applied to:

- Clone destination argument: `~/.tfenv`
- tfenv binary path in command: `~/.tfenv/bin/tfenv`
- SkipIf `FileExists` paths: `~/.tfenv` and `~/.tfenv/versions/<version>`

### Privilege

`privileged: false` — tfenv installs into `~/.tfenv` (user home). No root required.

### summarize()

```rust
match &self.version {
    Some(v) => format!("Installing tfenv and Terraform {v}"),
    None    => String::from("Installing tfenv"),
}
```

### PATH note

The action does NOT create symlinks or modify shell config. Users must add `~/.tfenv/bin` to PATH themselves (via `.zshrc`, `.bashrc`, or a separate `file.link` action). Documented in the example manifest.

### YAML name

`terraform.tfenv` — new action group `terraform`.

### Example manifest

```yaml
# Install tfenv and Terraform 1.9.0, set as global default.
# Add ~/.tfenv/bin to PATH separately (e.g. via shell config or file.link).

- action: terraform.tfenv
  version: "1.9.0"
  where: 'os.family == "linux" or os.name == "macos"'
```

## New module

Creates a new `terraform` action group — first action in this group:

| File                                 | Change                                            |
| ------------------------------------ | ------------------------------------------------- |
| `lib/src/actions/terraform/tfenv.rs` | new — action impl + unit tests                    |
| `lib/src/actions/terraform/mod.rs`   | new — `mod tfenv; pub use tfenv::TerraformTfenv;` |

## Registration

Follows the standard 6-edit pattern in `lib/src/actions/mod.rs`:

- `mod terraform;` declaration
- `use crate::actions::terraform::TerraformTfenv;` import
- Enum variant with serde rename `terraform.tfenv`
- Match arm in `inner_ref()`
- Match arm in `notify()`
- Match arm in `Deref`
- Match arm in `Display`

## Tests

Unit tests in `lib/src/actions/terraform/tfenv.rs`:

| Test                                     | Assertion                                                           |
| ---------------------------------------- | ------------------------------------------------------------------- |
| `it_can_be_deserialized_without_version` | deserializes to `TerraformTfenv { version: None }`                  |
| `it_can_be_deserialized_with_version`    | deserializes to `TerraformTfenv { version: Some("1.9.0") }`         |
| `summarize_without_version`              | returns `"Installing tfenv"`                                        |
| `summarize_with_version`                 | returns `"Installing tfenv and Terraform 1.9.0"`                    |
| `plan_without_version_emits_one_step`    | `plan()` returns 1 step                                             |
| `plan_with_version_emits_three_steps`    | `plan()` returns 3 steps                                            |
| `plan_step1_clones_tfenv`                | atom `to_string()` contains `"git"` and `"clone"` and `"tfenv"`     |
| `plan_step1_has_one_initializer`         | `steps[0].initializers.len() == 1`                                  |
| `plan_step2_runs_tfenv_install`          | atom `to_string()` contains `"tfenv"` and `"install"` and `"1.9.0"` |
| `plan_step2_has_one_initializer`         | `steps[1].initializers.len() == 1`                                  |
| `plan_step3_runs_tfenv_use`              | atom `to_string()` contains `"tfenv"` and `"use"` and `"1.9.0"`     |
| `plan_step3_has_no_initializers`         | `steps[2].initializers.len() == 0`                                  |

## Files changed

| File                                 | Change                                                   |
| ------------------------------------ | -------------------------------------------------------- |
| `lib/src/actions/terraform/tfenv.rs` | new — action impl + unit tests                           |
| `lib/src/actions/terraform/mod.rs`   | new                                                      |
| `lib/src/actions/mod.rs`             | register variant + 5 match arms + update test YAML lists |
| `examples/terraform/tfenv.yaml`      | new example manifest                                     |
| `docs/superpowers/README.md`         | add row, status In Progress → Done post-merge            |
| `README.md`                          | add `terraform.tfenv` to action catalog                  |
