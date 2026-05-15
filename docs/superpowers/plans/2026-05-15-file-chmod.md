# file.chmod Action Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `file.chmod` action that sets file/directory permissions declaratively, with optional `privileged: true` for sudo escalation.

**Architecture:** Create `lib/src/actions/file/chmod.rs` following the `FileChown` pattern. Non-privileged path uses the existing `Chmod` atom (`lib/src/atoms/file/chmod.rs`); privileged path uses the `Exec` atom with `chmod` as the command (same pattern as `command.run`). Wire into the `Actions` enum in `lib/src/actions/mod.rs`.

**Tech Stack:** Rust, serde, schemars, existing `Chmod` and `Exec` atoms, `anyhow` for errors.

---

## Files

| File                            | Change                                                                               |
| ------------------------------- | ------------------------------------------------------------------------------------ |
| `lib/src/actions/file/chmod.rs` | **Create** — `FileChmod` struct + `Action` impl + 5 tests                            |
| `lib/src/actions/file/mod.rs`   | **Modify** — add `pub mod chmod;`                                                    |
| `lib/src/actions/mod.rs`        | **Modify** — import, enum variant, `inner_ref`, `Deref`, `Display`, round-trip tests |

---

### Task 1: Create `lib/src/actions/file/chmod.rs` (TDD)

**Files:**

- Create: `lib/src/actions/file/chmod.rs`

- [ ] **Step 1: Write the failing deserialization test**

Create `lib/src/actions/file/chmod.rs` with only the test module first:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn it_can_be_deserialized() {
        use crate::actions::Actions;
        let yaml = r#"
- action: file.chmod
  path: /tmp/testdir
  mode: "700"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileChmod(action)) => {
                assert_eq!("/tmp/testdir", action.action.path);
                assert_eq!("700", action.action.mode);
                assert!(!action.action.privileged);
            }
            _ => panic!("FileChmod didn't deserialize to the correct type"),
        }
    }
}
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cargo test -p etch-lib it_can_be_deserialized 2>&1 | grep -E "^error|FAILED|cannot find"
```

Expected: compile error — `FileChmod` doesn't exist yet.

- [ ] **Step 3: Add the struct and Action impl**

Replace the file content with the full implementation:

```rust
use crate::actions::Action;
use crate::atoms::file::Chmod;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::utilities;
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::FileAction;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChmod {
    pub path: String,
    pub mode: String,
    #[serde(default = "get_false", alias = "sudo")]
    pub privileged: bool,
}

fn get_false() -> bool {
    false
}

fn parse_mode(mode: &str) -> anyhow::Result<u32> {
    let stripped = mode.strip_prefix("0o").unwrap_or(mode);
    u32::from_str_radix(stripped, 8).map_err(|_| anyhow!("invalid mode: {}", mode))
}

impl FileAction for FileChmod {}

impl Action for FileChmod {
    fn summarize(&self) -> String {
        format!("Set permissions {} on {}", self.mode, self.path)
    }

    fn plan(&self, _: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        if self.privileged {
            use crate::atoms::command::Exec;
            let privilege_provider = utilities::get_privilege_provider(contexts)
                .unwrap_or_else(|| "sudo".to_string());
            return Ok(vec![Step {
                atom: Box::new(Exec {
                    command: "chmod".into(),
                    arguments: vec![self.mode.clone(), self.path.clone()],
                    privileged: true,
                    privilege_provider,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            }]);
        }

        let mode = parse_mode(&self.mode)?;
        Ok(vec![Step {
            atom: Box::new(Chmod {
                path: self.path.clone().parse()?,
                mode,
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_can_be_deserialized() {
        use crate::actions::Actions;
        let yaml = r#"
- action: file.chmod
  path: /tmp/testdir
  mode: "700"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileChmod(action)) => {
                assert_eq!("/tmp/testdir", action.action.path);
                assert_eq!("700", action.action.mode);
                assert!(!action.action.privileged);
            }
            _ => panic!("FileChmod didn't deserialize to the correct type"),
        }
    }
}
```

Note: This will fail to compile until Task 2 registers `FileChmod` in the `Actions` enum. That is expected — proceed to Task 2, then run the tests.

- [ ] **Step 4: Add the remaining 4 tests**

Append inside the `#[cfg(test)] mod tests { ... }` block, after `it_can_be_deserialized`:

```rust
    #[test]
    fn plan_returns_chmod_step() {
        use super::FileChmod;
        use crate::actions::Action;
        let action = FileChmod {
            path: String::from("/tmp/testdir"),
            mode: String::from("700"),
            privileged: false,
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        // Chmod atom Display: "The permissions on <path> need to be set to <mode>"
        assert!(steps[0].atom.to_string().contains("need to be set"));
    }

    #[test]
    fn plan_errors_on_invalid_mode() {
        use super::FileChmod;
        use crate::actions::Action;
        let action = FileChmod {
            path: String::from("/tmp/testdir"),
            mode: String::from("xyz"),
            privileged: false,
        };
        assert!(action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .is_err());
    }

    #[test]
    fn plan_returns_exec_step_when_privileged() {
        use super::FileChmod;
        use crate::actions::Action;
        let action = FileChmod {
            path: String::from("/tmp/testdir"),
            mode: String::from("700"),
            privileged: true,
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        // Privileged path uses Exec atom — Display does NOT contain "need to be set"
        assert!(!steps[0].atom.to_string().contains("need to be set"));
    }

    #[test]
    fn summarize_includes_path_and_mode() {
        use super::FileChmod;
        use crate::actions::Action;
        let action = FileChmod {
            path: String::from("/tmp/testdir"),
            mode: String::from("755"),
            privileged: false,
        };
        let summary = action.summarize();
        assert!(summary.contains("/tmp/testdir"));
        assert!(summary.contains("755"));
    }
```

---

### Task 2: Register in mod files and update round-trip tests

**Files:**

- Modify: `lib/src/actions/file/mod.rs:1`
- Modify: `lib/src/actions/mod.rs`

- [ ] **Step 1: Add module to `lib/src/actions/file/mod.rs`**

The file currently starts with:

```rust
pub mod chown;
pub mod copy;
```

Add `pub mod chmod;` as the first line (alphabetical order):

```rust
pub mod chmod;
pub mod chown;
pub mod copy;
pub mod download;
pub mod link;
pub mod remove;
pub mod unarchive;
```

- [ ] **Step 2: Add import to `lib/src/actions/mod.rs`**

Find the import block (around line 18):

```rust
use file::chown::FileChown;
use file::copy::FileCopy;
```

Add immediately before `use file::chown::FileChown;`:

```rust
use file::chmod::FileChmod;
```

- [ ] **Step 3: Add enum variant to `lib/src/actions/mod.rs`**

Find the `Actions` enum. Between `FileCopy` and `FileChown` variants (around line 124):

```rust
    #[serde(rename = "file.copy")]
    FileCopy(ConditionalVariantAction<FileCopy>),

    #[serde(rename = "file.chown")]
    FileChown(ConditionalVariantAction<FileChown>),
```

Add the new variant between them:

```rust
    #[serde(rename = "file.copy")]
    FileCopy(ConditionalVariantAction<FileCopy>),

    #[serde(rename = "file.chmod")]
    FileChmod(ConditionalVariantAction<FileChmod>),

    #[serde(rename = "file.chown")]
    FileChown(ConditionalVariantAction<FileChown>),
```

- [ ] **Step 4: Add `FileChmod` arm to `inner_ref()` in `lib/src/actions/mod.rs`**

Find `impl Actions { pub fn inner_ref(&self)` (around line 178). Add after `Actions::FileCopy(a) => a,`:

```rust
            Actions::FileChmod(a) => a,
```

- [ ] **Step 5: Add `FileChmod` arm to `Deref::deref()` in `lib/src/actions/mod.rs`**

Find `impl Deref for Actions` (around line 204). Add after `Actions::FileCopy(a) => a,`:

```rust
            Actions::FileChmod(a) => a,
```

- [ ] **Step 6: Add `FileChmod` arm to `Display::fmt()` in `lib/src/actions/mod.rs`**

Find `impl Display for Actions` (around line 231). Add after `Actions::FileCopy(_) => "file.copy",`:

```rust
            Actions::FileChmod(_) => "file.chmod",
```

- [ ] **Step 7: Update `all_major_action_variants_can_be_deserialized` test**

Find the test (around line 334). Add `file.chmod` after the `file.chown` entry in the YAML:

```yaml
- action: file.chown
  path: /tmp/f
- action: file.chmod
  path: /tmp/f
  mode: "700"
```

Update the assertion from `assert_eq!(14, ...)` to `assert_eq!(15, manifest.actions.len())`.

- [ ] **Step 8: Update `actions_display_names` test**

Find the test (around line 378). Add `file.chmod` after the `file.chown` entry in the YAML:

```yaml
- action: file.chown
  path: /tmp/f
- action: file.chmod
  path: /tmp/f
  mode: "700"
```

Add `"file.chmod"` after `"file.chown"` in the `expected_names` array:

```rust
        let expected_names = [
            "command.run",
            "directory.copy",
            "directory.create",
            "directory.remove",
            "file.copy",
            "file.chown",
            "file.chmod",
            "file.download",
            // ... rest unchanged
        ];
```

- [ ] **Step 9: Run full test suite and confirm all tests pass**

```bash
make test 2>&1 | tail -10
```

Expected: all tests pass including the 5 new `actions::file::chmod::tests::*` tests.

- [ ] **Step 10: Commit**

```bash
git add lib/src/actions/file/chmod.rs \
        lib/src/actions/file/mod.rs \
        lib/src/actions/mod.rs
git commit -m "feat: add file.chmod action

Declarative chmod with optional privileged (sudo) escalation.
Non-privileged uses Chmod atom; privileged uses Exec atom with chmod.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 3: Update docs

**Files:**

- Modify: `docs/superpowers/README.md`

- [ ] **Step 1: Mark the spec as Done in README**

In `docs/superpowers/README.md`, change the `file-chmod` row from `Pending` to `Done`:

```markdown
| 2026-05-15 | [file-chmod](plans/2026-05-15-file-chmod.md) | [file-chmod](specs/2026-05-15-file-chmod-design.md) | Done |
```

Also add a `> **Status: DONE**` banner at the top of this plan file once Task 2 is complete.

- [ ] **Step 2: Update the dotfiles core.yaml to remove the command.run chmod workarounds**

In `~/git-repos/personal/dotfiles/manifests/dotfiles/core.yaml`, replace the 4× `directory.create` + `command.run chmod 700` pairs with `file.chmod` actions:

```yaml
- action: directory.create
  path: "{{ user.home_dir }}/.ssh"
- action: file.chmod
  path: "{{ user.home_dir }}/.ssh"
  mode: "700"
```

Repeat for `.warp`, `.tf_creds`, `.tsh`. (The `command.run` workaround lines are removed.)

- [ ] **Step 3: Commit docs and dotfiles**

```bash
# etch-cli docs
git -C /Users/bruce/git-repos/personal/etch-cli \
    add docs/superpowers/README.md \
       docs/superpowers/plans/2026-05-15-file-chmod.md
git -C /Users/bruce/git-repos/personal/etch-cli commit -m "docs: mark file-chmod Done"

# dotfiles — update manifests to use file.chmod
git -C /Users/bruce/git-repos/personal/dotfiles \
    add manifests/dotfiles/core.yaml
git -C /Users/bruce/git-repos/personal/dotfiles \
    commit -m "feat(etch): replace command.run chmod with file.chmod action"
```
