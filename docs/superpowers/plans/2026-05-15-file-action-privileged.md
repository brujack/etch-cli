# File Action Privileged Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `privileged: bool` / `sudo: true` support to `file.chown`, `file.link`, and `file.copy`, enforce it on all future file actions via a shared `FileActionConfig` struct and required `FileAction` trait method, and add unsupported-error stubs to `file.remove`, `file.download`, and `file.unarchive`.

**Architecture:** A `FileActionConfig { privileged: bool }` struct is added to `file/mod.rs` and embedded via `#[serde(flatten)]` in every file action. The `FileAction` trait gains a required `file_action_config(&self) -> &FileActionConfig` method — a compile error for any new file action that forgets to implement it. Privileged paths delegate to `Exec` atoms running the corresponding shell command with the configured privilege provider. `file.copy` privileged writes content to a deterministic tempfile, then sudo-copies it to the destination.

**Tech Stack:** Rust, serde `#[serde(flatten)]`, existing `Exec` atom, `utilities::get_privilege_provider`, `anyhow`, `tempfile` via `std::env::temp_dir()`

---

## Files

| File                                | Change                                                               |
| ----------------------------------- | -------------------------------------------------------------------- |
| `lib/src/actions/file/mod.rs`       | Add `FileActionConfig`, `get_false`, update `FileAction` trait       |
| `lib/src/actions/file/chmod.rs`     | Refactor: `self.privileged` → `self.config.privileged`, embed struct |
| `lib/src/actions/file/chown.rs`     | Add config, impl trait method, add privileged plan path              |
| `lib/src/actions/file/link.rs`      | Add config, impl trait method, add privileged plan path              |
| `lib/src/actions/file/copy.rs`      | Add config, impl trait method, add privileged plan path              |
| `lib/src/actions/file/remove.rs`    | Add config, impl trait method, add unsupported error guard           |
| `lib/src/actions/file/download.rs`  | Add config, impl trait method, add unsupported error guard           |
| `lib/src/actions/file/unarchive.rs` | Add config, impl trait method, add unsupported error guard           |

---

### Task 1: FileActionConfig + trait + coordinated refactor of all file actions

This task modifies all 7 file actions at once because adding a required trait method is a breaking change — the project won't compile until every `impl FileAction` provides the new method. All changes are structural (no new behavior), so all existing tests must still pass.

**Files:**

- Modify: `lib/src/actions/file/mod.rs`
- Modify: `lib/src/actions/file/chmod.rs`
- Modify: `lib/src/actions/file/chown.rs`
- Modify: `lib/src/actions/file/link.rs`
- Modify: `lib/src/actions/file/copy.rs`
- Modify: `lib/src/actions/file/remove.rs`
- Modify: `lib/src/actions/file/download.rs`
- Modify: `lib/src/actions/file/unarchive.rs`

- [ ] **Step 1: Add `FileActionConfig` and update the `FileAction` trait in `lib/src/actions/file/mod.rs`**

Add after the existing imports, before `pub trait FileAction`:

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileActionConfig {
    #[serde(default = "get_false", alias = "sudo")]
    pub privileged: bool,
}

fn get_false() -> bool {
    false
}
```

Change the `FileAction` trait definition from:

```rust
pub trait FileAction: Action {
    fn resolve(...) { ... }
    fn load(...) { ... }
}
```

to:

```rust
pub trait FileAction: Action {
    fn file_action_config(&self) -> &FileActionConfig;

    fn resolve(&self, manifest: &Manifest, path: &str) -> anyhow::Result<PathBuf> {
        // existing body unchanged
    }

    fn load(&self, manifest: &Manifest, path: &str) -> Result<Vec<u8>> {
        // existing body unchanged
    }
}
```

- [ ] **Step 2: Refactor `lib/src/actions/file/chmod.rs`**

Remove the local `fn get_false() -> bool { false }`.

Change the struct from:

```rust
pub struct FileChmod {
    pub path: String,
    pub mode: String,
    #[serde(default = "get_false", alias = "sudo")]
    pub privileged: bool,
}
```

to:

```rust
use super::FileActionConfig;

pub struct FileChmod {
    pub path: String,
    pub mode: String,
    #[serde(flatten)]
    pub config: FileActionConfig,
}
```

Change `impl FileAction for FileChmod {}` to:

```rust
impl FileAction for FileChmod {
    fn file_action_config(&self) -> &FileActionConfig {
        &self.config
    }
}
```

In `plan()`, replace every `self.privileged` with `self.config.privileged`.

- [ ] **Step 3: Add `FileActionConfig` to `lib/src/actions/file/chown.rs`**

Add import at top: `use super::FileActionConfig;`

Change struct from:

```rust
pub struct FileChown {
    pub path: String,
    pub user: Option<String>,
    pub group: Option<String>,
}
```

to:

```rust
pub struct FileChown {
    pub path: String,
    pub user: Option<String>,
    pub group: Option<String>,
    #[serde(flatten)]
    pub config: FileActionConfig,
}
```

Change `impl FileAction for FileChown {}` to:

```rust
impl FileAction for FileChown {
    fn file_action_config(&self) -> &FileActionConfig {
        &self.config
    }
}
```

No changes to `plan()` yet — privileged path is added in Task 2.

- [ ] **Step 4: Add `FileActionConfig` to `lib/src/actions/file/link.rs`**

Add import at top: `use super::FileActionConfig;`

Add field to struct:

```rust
pub struct FileLink {
    pub from: Option<String>,
    pub source: Option<String>,
    pub target: Option<String>,
    pub to: Option<String>,
    #[serde(default = "walk_dir_default")]
    pub walk_dir: bool,
    #[serde(flatten)]
    pub config: FileActionConfig,
}
```

Change `impl FileAction for FileLink {}` to:

```rust
impl FileAction for FileLink {
    fn file_action_config(&self) -> &FileActionConfig {
        &self.config
    }
}
```

No changes to `plan()` yet.

- [ ] **Step 5: Add `FileActionConfig` to `lib/src/actions/file/copy.rs`**

Add import at top: `use super::FileActionConfig;`

Add field to struct:

```rust
pub struct FileCopy {
    #[serde(alias = "source")]
    pub from: String,
    #[serde(alias = "target")]
    pub to: String,
    #[serde(default = "default_chmod", deserialize_with = "from_octal")]
    pub chmod: u32,
    #[serde(default = "default_template")]
    pub template: bool,
    pub passphrase: Option<String>,
    #[serde(rename = "owned_by_user")]
    pub owner_user: Option<String>,
    #[serde(rename = "owned_by_group")]
    pub owner_group: Option<String>,
    #[serde(flatten)]
    pub config: FileActionConfig,
}
```

Change `impl FileAction for FileCopy {}` to:

```rust
impl FileAction for FileCopy {
    fn file_action_config(&self) -> &FileActionConfig {
        &self.config
    }
}
```

No changes to `plan()` yet.

- [ ] **Step 6: Add `FileActionConfig` + unsupported error to `lib/src/actions/file/remove.rs`**

Add import: `use super::FileActionConfig;`
Also add: `use anyhow::anyhow;`

Change struct:

```rust
pub struct FileRemove {
    pub target: String,
    #[serde(flatten)]
    pub config: FileActionConfig,
}
```

Change `impl FileAction for FileRemove {}` to:

```rust
impl FileAction for FileRemove {
    fn file_action_config(&self) -> &FileActionConfig {
        &self.config
    }
}
```

At the top of `plan()`, add:

```rust
if self.config.privileged {
    return Err(anyhow!("file.remove does not support privileged mode"));
}
```

Add test inside `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn plan_errors_when_privileged_not_supported() {
        use super::FileRemove;
        use crate::actions::Action;
        use crate::actions::file::FileActionConfig;
        let action = FileRemove {
            target: String::from("/tmp/file"),
            config: FileActionConfig { privileged: true },
        };
        assert!(action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .is_err());
    }
```

- [ ] **Step 7: Apply the same pattern to `lib/src/actions/file/download.rs` and `lib/src/actions/file/unarchive.rs`**

**`lib/src/actions/file/download.rs`** — add `use super::FileActionConfig;` and `use anyhow::anyhow;`. Add field to the struct (after `owner_group`):

```rust
    #[serde(flatten)]
    pub config: FileActionConfig,
```

Change `impl FileAction for FileDownload {}` to:

```rust
impl FileAction for FileDownload {
    fn file_action_config(&self) -> &FileActionConfig { &self.config }
}
```

Add at the top of `plan()`:

```rust
if self.config.privileged {
    return Err(anyhow!("file.download does not support privileged mode"));
}
```

Add test:

```rust
    #[test]
    fn plan_errors_when_privileged_not_supported() {
        use super::FileDownload;
        use crate::actions::Action;
        use crate::actions::file::FileActionConfig;
        let action = FileDownload {
            from: "https://example.com/file".to_string(),
            to: "/tmp/file".to_string(),
            config: FileActionConfig { privileged: true },
            ..Default::default()
        };
        assert!(action
            .plan(&crate::manifests::Manifest::default(), &crate::contexts::Contexts::default())
            .is_err());
    }
```

**`lib/src/actions/file/unarchive.rs`** — add `use super::FileActionConfig;` and `use anyhow::anyhow;`. Add field to the struct (after `force`):

```rust
    #[serde(flatten)]
    pub config: FileActionConfig,
```

Change `impl FileAction for FileUnarchive {}` to:

```rust
impl FileAction for FileUnarchive {
    fn file_action_config(&self) -> &FileActionConfig { &self.config }
}
```

Add at the top of `plan()`:

```rust
if self.config.privileged {
    return Err(anyhow!("file.unarchive does not support privileged mode"));
}
```

Add test:

```rust
    #[test]
    fn plan_errors_when_privileged_not_supported() {
        use super::FileUnarchive;
        use crate::actions::Action;
        use crate::actions::file::FileActionConfig;
        let action = FileUnarchive {
            from: "/tmp/archive.tar.gz".to_string(),
            to: "/tmp/dest".to_string(),
            force: None,
            config: FileActionConfig { privileged: true },
        };
        assert!(action
            .plan(&crate::manifests::Manifest::default(), &crate::contexts::Contexts::default())
            .is_err());
    }
```

- [ ] **Step 8: Verify compilation and run full test suite**

```bash
make test 2>&1 | tail -10
```

Expected: all existing tests pass. The test count will include the 2 new stub tests from remove.rs and similar tests from download.rs and unarchive.rs.

- [ ] **Step 9: Commit**

```bash
git add lib/src/actions/file/
git commit -m "refactor: add FileActionConfig + required trait method to all file actions

Embeds shared privileged field via #[serde(flatten)] in all 7 file
actions. file.remove, file.download, file.unarchive return Err if
privileged. file.chmod refactored to use shared config.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 2: file.chown privileged path

**Files:**

- Modify: `lib/src/actions/file/chown.rs`

- [ ] **Step 1: Write failing tests**

Add inside the existing `#[cfg(test)] mod tests { ... }` block:

```rust
    #[test]
    fn it_can_be_deserialized_with_privileged() {
        use crate::actions::Actions;
        let yaml = r#"
- action: file.chown
  path: /tmp/file
  user: root
  sudo: true
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileChown(action)) => {
                assert!(action.action.config.privileged);
            }
            _ => panic!("FileChown didn't deserialize"),
        }
    }

    #[test]
    fn plan_returns_exec_step_when_privileged() {
        use super::FileChown;
        use crate::actions::Action;
        use crate::actions::file::FileActionConfig;
        let action = FileChown {
            path: String::from("/tmp/file"),
            user: Some(String::from("root")),
            group: Some(String::from("root")),
            config: FileActionConfig { privileged: true },
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        // Exec atom display contains "chown" not the Chown atom text
        assert!(!steps[0].atom.to_string().contains("change ownership"));
    }

    #[test]
    fn plan_privileged_group_only() {
        use super::FileChown;
        use crate::actions::Action;
        use crate::actions::file::FileActionConfig;
        let action = FileChown {
            path: String::from("/tmp/file"),
            user: None,
            group: Some(String::from("staff")),
            config: FileActionConfig { privileged: true },
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
    }
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test -p etch-lib actions::file::chown::tests::plan_returns_exec_step_when_privileged 2>&1 | tail -5
```

Expected: FAIL — no privileged path in plan() yet.

- [ ] **Step 3: Implement the privileged path in `lib/src/actions/file/chown.rs`**

Add `use crate::utilities;` to the top imports.

In `plan()`, add at the top (before the existing Chown atom construction):

```rust
if self.config.privileged {
    use crate::atoms::command::Exec;
    let privilege_provider = utilities::get_privilege_provider(contexts)
        .unwrap_or_else(|| "sudo".to_string());
    let ownership = match (&self.user, &self.group) {
        (Some(u), Some(g)) => format!("{}:{}", u, g),
        (Some(u), None) => u.clone(),
        (None, Some(g)) => format!(":{}", g),
        (None, None) => return Ok(vec![]),
    };
    return Ok(vec![crate::steps::Step {
        atom: Box::new(Exec {
            command: "chown".into(),
            arguments: vec![ownership, self.path.clone()],
            privileged: true,
            privilege_provider,
            ..Default::default()
        }),
        initializers: vec![],
        finalizers: vec![],
    }]);
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cargo test -p etch-lib actions::file::chown 2>&1 | tail -10
```

Expected: all chown tests pass.

- [ ] **Step 5: Run full suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/file/chown.rs
git commit -m "feat: add privileged path to file.chown

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 3: file.link privileged path

**Files:**

- Modify: `lib/src/actions/file/link.rs`

- [ ] **Step 1: Write failing tests**

Add inside the existing `#[cfg(test)] mod tests { ... }` block:

```rust
    #[test]
    fn it_can_be_deserialized_with_privileged() {
        use crate::actions::Actions;
        let yaml = r#"
- action: file.link
  source: /opt/bin/tool
  target: /usr/local/bin/tool
  privileged: true
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileLink(action)) => {
                assert!(action.action.config.privileged);
            }
            _ => panic!("FileLink didn't deserialize"),
        }
    }

    #[test]
    fn plan_returns_exec_steps_when_privileged() {
        use super::FileLink;
        use crate::actions::Action;
        use crate::actions::file::FileActionConfig;
        use crate::config::Config;
        use crate::contexts::build_contexts;

        let tmp = tempfile::tempdir().unwrap();
        let real_tmp = tmp.path().canonicalize().unwrap();
        let files_dir = real_tmp.join("files");
        std::fs::create_dir_all(&files_dir).unwrap();
        let source_file = files_dir.join("mytool");
        std::fs::write(&source_file, b"binary").unwrap();

        let manifest = crate::manifests::Manifest {
            root_dir: Some(real_tmp.clone()),
            ..Default::default()
        };
        let contexts = build_contexts(&Config::default());

        let action = FileLink {
            source: Some("mytool".to_string()),
            target: Some(real_tmp.join("linked").display().to_string()),
            config: FileActionConfig { privileged: true },
            ..Default::default()
        };
        let steps = action.plan(&manifest, &contexts).unwrap();
        // 2 steps: mkdir -p + ln -sf
        assert_eq!(2, steps.len());
        // Neither step is a Link atom (Display would contain "need to be linked")
        assert!(!steps[0].atom.to_string().contains("need to be linked"));
        assert!(!steps[1].atom.to_string().contains("need to be linked"));
    }
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test -p etch-lib actions::file::link::tests::plan_returns_exec_steps_when_privileged 2>&1 | tail -5
```

Expected: FAIL — no privileged path yet.

- [ ] **Step 3: Implement the privileged path in `lib/src/actions/file/link.rs`**

Add `use crate::utilities;` to imports.

Add a new static method to `impl FileLink`:

```rust
pub fn plan_privileged(from: PathBuf, to: PathBuf, privilege_provider: &str, walk_dir: bool) -> Vec<Step> {
    use crate::atoms::command::Exec;

    let make_mkdir = |parent: &std::path::Path| -> Step {
        Step {
            atom: Box::new(Exec {
                command: "mkdir".into(),
                arguments: vec!["-p".to_string(), parent.display().to_string()],
                privileged: true,
                privilege_provider: privilege_provider.to_string(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }
    };

    let make_ln = |src: &std::path::Path, tgt: &std::path::Path| -> Step {
        Step {
            atom: Box::new(Exec {
                command: "ln".into(),
                arguments: vec![
                    "-sf".to_string(),
                    src.display().to_string(),
                    tgt.display().to_string(),
                ],
                privileged: true,
                privilege_provider: privilege_provider.to_string(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }
    };

    if from.is_file() || !walk_dir {
        // Single symlink
        match to.parent() {
            Some(parent) => vec![make_mkdir(parent), make_ln(&from, &to)],
            None => vec![],
        }
    } else {
        // Walk directory
        let mut steps = vec![];
        if let Ok(entries) = std::fs::read_dir(&from) {
            for entry in entries.flatten() {
                let src_item = entry.path();
                if let Some(file_name) = src_item.file_name() {
                    let tgt_item = to.join(file_name);
                    if let Some(parent) = tgt_item.parent() {
                        steps.push(make_mkdir(parent));
                    }
                    steps.push(make_ln(&src_item, &tgt_item));
                }
            }
        }
        steps
    }
}
```

In `plan()`, after the `let to = ...` line, add the privileged check:

```rust
if self.config.privileged {
    let privilege_provider = utilities::get_privilege_provider(contexts)
        .unwrap_or_else(|| "sudo".to_string());
    let walk = self.walk_dir && !from.is_file();
    return Ok(FileLink::plan_privileged(from, to, &privilege_provider, walk));
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cargo test -p etch-lib actions::file::link 2>&1 | tail -10
```

Expected: all link tests pass.

- [ ] **Step 5: Run full suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/file/link.rs
git commit -m "feat: add privileged path to file.link

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 4: file.copy privileged path

**Files:**

- Modify: `lib/src/actions/file/copy.rs`

- [ ] **Step 1: Write failing tests**

Add inside the existing `#[cfg(test)] mod tests { ... }` block:

```rust
    #[test]
    fn it_can_be_deserialized_with_privileged() {
        use crate::actions::Actions;
        let yaml = r#"
- action: file.copy
  from: a
  to: /etc/b
  privileged: true
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileCopy(action)) => {
                assert!(action.action.config.privileged);
            }
            _ => panic!("FileCopy didn't deserialize"),
        }
    }

    #[test]
    fn plan_returns_exec_steps_when_privileged_no_template() {
        use super::FileCopy;
        use crate::actions::Action;
        use crate::actions::file::FileActionConfig;

        let tmp = tempfile::tempdir().unwrap();
        let real_tmp = tmp.path().canonicalize().unwrap();
        let files_dir = real_tmp.join("files");
        std::fs::create_dir_all(&files_dir).unwrap();
        std::fs::write(files_dir.join("source.txt"), b"content").unwrap();

        let manifest = crate::manifests::Manifest {
            root_dir: Some(real_tmp.clone()),
            ..Default::default()
        };

        let action = FileCopy {
            from: "source.txt".to_string(),
            to: real_tmp.join("dest.txt").display().to_string(),
            config: FileActionConfig { privileged: true },
            ..Default::default()
        };
        let steps = action
            .plan(&manifest, &crate::contexts::Contexts::default())
            .unwrap();
        // SetContents(tempfile) + mkdir + cp + chmod + rm = 5 steps
        assert_eq!(5, steps.len());
    }

    #[test]
    fn plan_returns_setcontents_then_exec_when_privileged_template() {
        use super::FileCopy;
        use crate::actions::Action;
        use crate::actions::file::FileActionConfig;

        let tmp = tempfile::tempdir().unwrap();
        let real_tmp = tmp.path().canonicalize().unwrap();
        let files_dir = real_tmp.join("files");
        std::fs::create_dir_all(&files_dir).unwrap();
        std::fs::write(files_dir.join("tmpl.txt"), b"hello {{ user.username }}").unwrap();

        let manifest = crate::manifests::Manifest {
            root_dir: Some(real_tmp.clone()),
            ..Default::default()
        };

        let action = FileCopy {
            from: "tmpl.txt".to_string(),
            to: real_tmp.join("out.txt").display().to_string(),
            template: true,
            config: FileActionConfig { privileged: true },
            ..Default::default()
        };
        let steps = action
            .plan(&manifest, &crate::contexts::Contexts::default())
            .unwrap();
        // SetContents(tempfile) + mkdir + cp + chmod + rm = 5 steps
        assert_eq!(5, steps.len());
        // First step writes to a tempfile path (contains "etch-")
        assert!(steps[0].atom.to_string().contains("etch-"));
    }
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test -p etch-lib actions::file::copy::tests::plan_returns_exec_steps_when_privileged_no_template 2>&1 | tail -5
```

Expected: FAIL — no privileged path yet.

- [ ] **Step 3: Implement the privileged path in `lib/src/actions/file/copy.rs`**

Add `use crate::utilities;` to imports.

In `plan()`, after the `contents` variable is computed and `path` is resolved, add the privileged block (immediately before the existing `let parent = path.clone(); let mut steps = vec![...]`):

```rust
if self.config.privileged {
    use crate::atoms::command::Exec;
    use crate::atoms::file::SetContents;
    let privilege_provider = utilities::get_privilege_provider(contexts)
        .unwrap_or_else(|| "sudo".to_string());

    // Deterministic tempfile path: /tmp/etch-<dest-with-slashes-as-dashes>
    let temp_name = format!(
        "etch-{}",
        path.display()
            .to_string()
            .replace('/', "-")
            .trim_matches('-')
    );
    let temp_path = std::env::temp_dir().join(&temp_name);

    let dest_parent = path
        .parent()
        .ok_or_else(|| anyhow!("Failed to get parent directory for FileCopy action"))?
        .to_path_buf();

    let mut steps = vec![];

    // Write content to tempfile (non-privileged)
    if let Some(passphrase) = self.passphrase.clone() {
        steps.push(crate::steps::Step {
            atom: Box::new(Decrypt {
                encrypted_content: contents,
                path: temp_path.clone(),
                passphrase,
            }),
            initializers: vec![],
            finalizers: vec![],
        });
    } else {
        steps.push(crate::steps::Step {
            atom: Box::new(SetContents {
                path: temp_path.clone(),
                contents,
            }),
            initializers: vec![],
            finalizers: vec![],
        });
    }

    // sudo mkdir -p dest_parent
    steps.push(crate::steps::Step {
        atom: Box::new(Exec {
            command: "mkdir".into(),
            arguments: vec!["-p".to_string(), dest_parent.display().to_string()],
            privileged: true,
            privilege_provider: privilege_provider.clone(),
            ..Default::default()
        }),
        initializers: vec![],
        finalizers: vec![],
    });

    // sudo cp tempfile dest
    steps.push(crate::steps::Step {
        atom: Box::new(Exec {
            command: "cp".into(),
            arguments: vec![temp_path.display().to_string(), path.display().to_string()],
            privileged: true,
            privilege_provider: privilege_provider.clone(),
            ..Default::default()
        }),
        initializers: vec![],
        finalizers: vec![],
    });

    // sudo chmod mode dest
    steps.push(crate::steps::Step {
        atom: Box::new(Exec {
            command: "chmod".into(),
            arguments: vec![format!("{:o}", self.chmod), path.display().to_string()],
            privileged: true,
            privilege_provider: privilege_provider.clone(),
            ..Default::default()
        }),
        initializers: vec![],
        finalizers: vec![],
    });

    // sudo chown (only if both user AND group specified)
    #[cfg(unix)]
    if let (Some(user), Some(group)) = (&self.owner_user, &self.owner_group) {
        steps.push(crate::steps::Step {
            atom: Box::new(Exec {
                command: "chown".into(),
                arguments: vec![
                    format!("{}:{}", user, group),
                    path.display().to_string(),
                ],
                privileged: true,
                privilege_provider: privilege_provider.clone(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        });
    }

    // rm tempfile (cleanup, non-privileged)
    steps.push(crate::steps::Step {
        atom: Box::new(Exec {
            command: "rm".into(),
            arguments: vec!["-f".to_string(), temp_path.display().to_string()],
            privileged: false,
            privilege_provider,
            ..Default::default()
        }),
        initializers: vec![],
        finalizers: vec![],
    });

    return Ok(steps);
}
```

Note: `SetContents` is already imported in the existing code block below (`use crate::atoms::file::{Chmod, Create, SetContents};`). Move that import to the top of the method body so it's available in the privileged block too, or use the full path `crate::atoms::file::SetContents` as shown above.

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cargo test -p etch-lib actions::file::copy 2>&1 | tail -10
```

Expected: all copy tests pass.

- [ ] **Step 5: Run full suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/file/copy.rs
git commit -m "feat: add privileged path to file.copy

Template rendering writes to tempfile; sudo cp moves to destination.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 5: Update docs

**Files:**

- Modify: `docs/superpowers/README.md` (etch-cli repo — post-merge on main, not in worktree)

Note: Per the worktree-docs-conflict memory, docs status updates belong on main after merge, not inside the worktree.

- [ ] **Step 1: Update plan status in README after the PR merges**

After the PR is merged to main and main is pulled locally, change the `file-action-privileged` row in `docs/superpowers/README.md` from `Pending` to `Done`.

Also add `> **Status: DONE**` banner at the top of this plan file (after the `# File Action Privileged Support Implementation Plan` heading).

- [ ] **Step 2: Update CLAUDE.md action catalog**

In `CLAUDE.md`, update the three action rows to reflect the new `privileged` field:

| Action       | Add to Key fields       |
| ------------ | ----------------------- |
| `file.chown` | add `privileged` (bool) |
| `file.link`  | add `privileged` (bool) |
| `file.copy`  | add `privileged` (bool) |

- [ ] **Step 3: Commit directly to main (docs-only exception)**

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-05-15-file-action-privileged.md CLAUDE.md
git commit -m "docs: mark file-action-privileged Done; update action catalog

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
git push origin main
```
