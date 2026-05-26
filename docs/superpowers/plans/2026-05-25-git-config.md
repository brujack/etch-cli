# git.config Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `git.config` action (alias `git.cfg`) that sets or unsets git configuration values at global, local, or system scope.

**Architecture:** New `GitConfig` action in `lib/src/actions/git/config.rs`. New `GitConfigUnset` atom in `lib/src/atoms/git/config_unset.rs` handles unset idempotency (exit code 5 when key absent). Set operations use the existing `Exec` atom directly. Action `plan()` is pure (no subprocess); atom `plan()` runs `git config --get` to decide whether to run. System scope auto-sets `privileged: true`.

**Tech Stack:** Rust; existing `Exec` atom; `indexmap = "2"` (new direct dep); `tempfile` (already a dev-dep) for atom tests.

---

### Task 1: Cargo deps + module scaffold

**Files:**

- Modify: `lib/Cargo.toml`
- Create: `lib/src/atoms/git/config_unset.rs`
- Modify: `lib/src/atoms/git/mod.rs`
- Create: `lib/src/actions/git/config.rs`
- Modify: `lib/src/actions/git/mod.rs`

- [ ] **Step 1: Add deps to `lib/Cargo.toml`**

Change the `schemars` line and add `indexmap` after the `zip` line:

```toml
# Change this line:
schemars = "1.2"
# To this:
schemars = { version = "1.2", features = ["indexmap2"] }

# Add after zip = "2":
indexmap = { version = "2", features = ["serde"] }
```

- [ ] **Step 2: Create atom stub `lib/src/atoms/git/config_unset.rs`**

```rust
use super::super::Atom;
use crate::atoms::Outcome;

pub struct GitConfigUnset {
    /// Args inserted between `git` and `--get`/`--unset`:
    /// global → ["config", "--global"]
    /// local  → ["-C", "/path", "config", "--local"]
    /// system → ["config", "--system"]
    pub config_args: Vec<String>,
    pub key: String,
    pub privileged: bool,
    pub privilege_provider: String,
}

impl std::fmt::Display for GitConfigUnset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GitConfigUnset key={}", self.key)
    }
}

impl Atom for GitConfigUnset {
    fn plan(&self) -> anyhow::Result<Outcome> {
        todo!()
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        todo!()
    }
}
```

- [ ] **Step 3: Update `lib/src/atoms/git/mod.rs`**

```rust
mod clone;
mod config_unset;
pub use clone::Clone;
pub use config_unset::GitConfigUnset;
```

- [ ] **Step 4: Create action stub `lib/src/actions/git/config.rs`**

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GitConfigScope {
    #[default]
    Global,
    Local,
    System,
}

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitConfig {
    pub scope: GitConfigScope,
    pub key: Option<String>,
    pub value: Option<String>,
    pub unset: Option<bool>,
    pub settings: Option<IndexMap<String, String>>,
    pub directory: Option<String>,
}

impl Action for GitConfig {
    fn summarize(&self) -> String {
        todo!()
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        todo!()
    }
}
```

- [ ] **Step 5: Update `lib/src/actions/git/mod.rs`**

```rust
mod clone;
mod config;
pub use clone::GitClone;
pub use config::GitConfig;
```

- [ ] **Step 6: Verify compilation**

```bash
cd lib && cargo build 2>&1 | grep -E "^error"
```

Expected: no output (no compile errors; `todo!()` panics are fine at compile time).

- [ ] **Step 7: Commit**

```bash
git add lib/Cargo.toml lib/Cargo.lock \
        lib/src/atoms/git/config_unset.rs lib/src/atoms/git/mod.rs \
        lib/src/actions/git/config.rs lib/src/actions/git/mod.rs
git commit -m "chore(git-config): add indexmap dep and module stubs"
```

---

### Task 2: GitConfigUnset atom — plan()

**Files:**

- Modify: `lib/src/atoms/git/config_unset.rs`

- [ ] **Step 1: Write failing tests**

Add at the bottom of `lib/src/atoms/git/config_unset.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn setup_repo() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["-C", tmp.path().to_str().unwrap(), "init"])
            .status()
            .unwrap();
        tmp
    }

    fn local_config_args(path: &str) -> Vec<String> {
        vec![
            "-C".into(),
            path.into(),
            "config".into(),
            "--local".into(),
        ]
    }

    #[test]
    fn plan_should_run_false_when_key_absent() {
        let tmp = setup_repo();
        let path = tmp.path().to_str().unwrap();
        let atom = GitConfigUnset {
            config_args: local_config_args(path),
            key: "user.email".into(),
            privileged: false,
            privilege_provider: String::new(),
        };
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_run_true_when_key_present() {
        let tmp = setup_repo();
        let path = tmp.path().to_str().unwrap();
        Command::new("git")
            .args(["-C", path, "config", "--local", "user.email", "test@example.com"])
            .status()
            .unwrap();
        let atom = GitConfigUnset {
            config_args: local_config_args(path),
            key: "user.email".into(),
            privileged: false,
            privilege_provider: String::new(),
        };
        assert!(atom.plan().unwrap().should_run);
    }
}
```

- [ ] **Step 2: Run tests — expect failure**

```bash
cd lib && cargo nextest run atoms::git::config_unset 2>&1 | tail -5
```

Expected: FAIL (panics at `todo!()`)

- [ ] **Step 3: Implement `plan()`**

Replace the `plan()` body in `lib/src/atoms/git/config_unset.rs`:

```rust
fn plan(&self) -> anyhow::Result<Outcome> {
    let mut args = self.config_args.clone();
    args.push("--get".into());
    args.push(self.key.clone());
    let status = std::process::Command::new("git")
        .args(&args)
        .output()?
        .status;
    Ok(Outcome {
        side_effects: vec![],
        should_run: status.success(),
    })
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cd lib && cargo nextest run atoms::git::config_unset 2>&1 | tail -5
```

Expected: PASS (2 tests)

- [ ] **Step 5: Commit**

```bash
git add lib/src/atoms/git/config_unset.rs
git commit -m "feat(git-config): GitConfigUnset atom plan() checks key existence"
```

---

### Task 3: GitConfigUnset atom — execute() + Display

**Files:**

- Modify: `lib/src/atoms/git/config_unset.rs`

- [ ] **Step 1: Write failing tests**

Add to the `tests` module in `lib/src/atoms/git/config_unset.rs`:

```rust
    #[test]
    fn execute_removes_the_key() {
        let tmp = setup_repo();
        let path = tmp.path().to_str().unwrap();
        // Set the key first
        Command::new("git")
            .args(["-C", path, "config", "--local", "user.name", "Test User"])
            .status()
            .unwrap();
        // Verify it exists
        let before = Command::new("git")
            .args(["-C", path, "config", "--local", "--get", "user.name"])
            .status()
            .unwrap();
        assert!(before.success());

        let mut atom = GitConfigUnset {
            config_args: local_config_args(path),
            key: "user.name".into(),
            privileged: false,
            privilege_provider: String::new(),
        };
        atom.execute().unwrap();

        // Key should be gone
        let after = Command::new("git")
            .args(["-C", path, "config", "--local", "--get", "user.name"])
            .status()
            .unwrap();
        assert!(!after.success());
    }

    #[test]
    fn display_includes_key() {
        let atom = GitConfigUnset {
            config_args: vec!["config".into(), "--global".into()],
            key: "credential.helper".into(),
            privileged: false,
            privilege_provider: String::new(),
        };
        assert!(format!("{atom}").contains("credential.helper"));
    }
```

- [ ] **Step 2: Run tests — expect failure**

```bash
cd lib && cargo nextest run atoms::git::config_unset 2>&1 | tail -5
```

Expected: FAIL (execute panics at `todo!()`)

- [ ] **Step 3: Implement `execute()`**

Replace the `execute()` body in `lib/src/atoms/git/config_unset.rs`. Add the import at the top of the file:

```rust
use crate::atoms::command::Exec;
```

Then replace `execute()`:

```rust
fn execute(&mut self) -> anyhow::Result<()> {
    let mut args = self.config_args.clone();
    args.push("--unset".into());
    args.push(self.key.clone());
    let mut exec = Exec {
        command: "git".into(),
        arguments: args,
        privileged: self.privileged,
        privilege_provider: self.privilege_provider.clone(),
        ..Default::default()
    };
    exec.execute()
}
```

- [ ] **Step 4: Run all atom tests — expect pass**

```bash
cd lib && cargo nextest run atoms::git::config_unset 2>&1 | tail -5
```

Expected: PASS (4 tests)

- [ ] **Step 5: Commit**

```bash
git add lib/src/atoms/git/config_unset.rs
git commit -m "feat(git-config): GitConfigUnset atom execute() and Display"
```

---

### Task 4: GitConfig struct + validation errors

**Files:**

- Modify: `lib/src/actions/git/config.rs`

- [ ] **Step 1: Write failing tests**

Add at the bottom of `lib/src/actions/git/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    fn plan(action: GitConfig) -> anyhow::Result<Vec<Step>> {
        action.plan(&Manifest::default(), &Contexts::default())
    }

    #[test]
    fn deserialize_single_key_value() {
        let yaml = r#"
- action: git.config
  scope: global
  key: user.email
  value: test@example.com
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::GitConfig(a)) => {
                assert_eq!(a.action.key, Some("user.email".into()));
                assert_eq!(a.action.value, Some("test@example.com".into()));
                assert!(matches!(a.action.scope, GitConfigScope::Global));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn deserialize_unset() {
        let yaml = r#"
- action: git.config
  scope: global
  key: credential.helper
  unset: true
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::GitConfig(a)) => {
                assert_eq!(a.action.key, Some("credential.helper".into()));
                assert_eq!(a.action.unset, Some(true));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn deserialize_settings_map() {
        let yaml = r#"
- action: git.config
  scope: global
  settings:
    user.name: Bruce
    user.email: bruce@example.com
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::GitConfig(a)) => {
                let s = a.action.settings.unwrap();
                assert_eq!(s.len(), 2);
                assert_eq!(s["user.name"], "Bruce");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn deserialize_local_scope() {
        let yaml = r#"
- action: git.config
  scope: local
  directory: /tmp/repo
  key: user.email
  value: local@example.com
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::GitConfig(a)) => {
                assert!(matches!(a.action.scope, GitConfigScope::Local));
                assert_eq!(a.action.directory, Some("/tmp/repo".into()));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn error_key_and_settings_both_present() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            key: Some("user.email".into()),
            value: Some("foo@bar.com".into()),
            settings: Some({
                let mut m = IndexMap::new();
                m.insert("user.name".into(), "Foo".into());
                m
            }),
            ..Default::default()
        };
        assert!(plan(action).is_err());
    }

    #[test]
    fn error_neither_key_nor_settings() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            ..Default::default()
        };
        assert!(plan(action).is_err());
    }

    #[test]
    fn error_unset_with_settings() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            unset: Some(true),
            settings: Some({
                let mut m = IndexMap::new();
                m.insert("user.email".into(), "foo@bar.com".into());
                m
            }),
            ..Default::default()
        };
        assert!(plan(action).is_err());
    }

    #[test]
    fn error_unset_with_value() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            key: Some("user.email".into()),
            unset: Some(true),
            value: Some("foo@bar.com".into()),
            ..Default::default()
        };
        assert!(plan(action).is_err());
    }

    #[test]
    fn error_local_scope_without_directory() {
        let action = GitConfig {
            scope: GitConfigScope::Local,
            key: Some("user.email".into()),
            value: Some("foo@bar.com".into()),
            ..Default::default()
        };
        assert!(plan(action).is_err());
    }

    #[test]
    fn error_key_without_value_or_unset() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            key: Some("user.email".into()),
            ..Default::default()
        };
        assert!(plan(action).is_err());
    }
}
```

Note: these tests reference `Actions::GitConfig` which doesn't exist yet. They will fail to compile until Task 9. Comment out the `deserialize_*` tests for now with `// TODO: uncomment after registration in Task 9`:

```rust
// #[test]
// fn deserialize_single_key_value() { ... }
// (and all other deserialize_ tests)
```

Leave the validation tests (`error_*`) active — they only need `GitConfig` and `GitConfigScope` which already exist.

- [ ] **Step 2: Run tests — expect failure**

```bash
cd lib && cargo nextest run actions::git::config 2>&1 | tail -5
```

Expected: FAIL (`todo!()` in `plan()`)

- [ ] **Step 3: Implement validation in `plan()`**

Replace the `plan()` body in `lib/src/actions/git/config.rs`:

```rust
fn plan(&self, _manifest: &Manifest, _contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
    use anyhow::anyhow;

    // Validate mutually exclusive fields
    if self.key.is_some() && self.settings.is_some() {
        return Err(anyhow!("git.config: 'key' and 'settings' are mutually exclusive"));
    }
    if self.key.is_none() && self.settings.is_none() {
        return Err(anyhow!("git.config: one of 'key' or 'settings' is required"));
    }
    if self.unset == Some(true) && self.settings.is_some() {
        return Err(anyhow!("git.config: 'unset' cannot be used with 'settings'"));
    }
    if self.unset == Some(true) && self.value.is_some() {
        return Err(anyhow!("git.config: 'unset' and 'value' are mutually exclusive"));
    }
    if matches!(self.scope, GitConfigScope::Local) && self.directory.is_none() {
        return Err(anyhow!("git.config: 'directory' is required for scope 'local'"));
    }
    if self.key.is_some() && self.value.is_none() && self.unset != Some(true) {
        return Err(anyhow!(
            "git.config: 'key' requires either 'value' (to set) or 'unset: true'"
        ));
    }

    todo!("implement step generation in later tasks")
}
```

Also implement `summarize()` — replace `todo!()`:

```rust
fn summarize(&self) -> String {
    let scope = match self.scope {
        GitConfigScope::Global => "global",
        GitConfigScope::Local => "local",
        GitConfigScope::System => "system",
    };
    if let Some(ref key) = self.key {
        if self.unset == Some(true) {
            return format!("Unset git.{scope} {key}");
        }
        let val = self.value.as_deref().unwrap_or("(from settings)");
        return format!("Set git.{scope} {key} = {val}");
    }
    let count = self.settings.as_ref().map_or(0, |s| s.len());
    format!("Set {count} git.{scope} config values")
}
```

- [ ] **Step 4: Run validation tests — expect pass**

```bash
cd lib && cargo nextest run actions::git::config 2>&1 | tail -10
```

Expected: validation `error_*` tests PASS; `plan_*` tests not yet written.

- [ ] **Step 5: Commit**

```bash
git add lib/src/actions/git/config.rs
git commit -m "feat(git-config): GitConfig struct, scope enum, validation, summarize"
```

---

### Task 5: GitConfig plan() — global set

**Files:**

- Modify: `lib/src/actions/git/config.rs`

- [ ] **Step 1: Write failing test**

Add to the `tests` module in `lib/src/actions/git/config.rs`:

```rust
    #[test]
    fn plan_global_set_emits_one_exec_step() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            key: Some("user.email".into()),
            value: Some("test@example.com".into()),
            ..Default::default()
        };
        let steps = plan(action).unwrap();
        assert_eq!(steps.len(), 1);
        let display = steps[0].atom.to_string();
        assert!(display.contains("config"), "display: {display}");
        assert!(display.contains("--global"), "display: {display}");
        assert!(display.contains("user.email"), "display: {display}");
        assert!(display.contains("test@example.com"), "display: {display}");
        assert!(display.contains("privileged=false"), "display: {display}");
    }
```

- [ ] **Step 2: Run test — expect failure**

```bash
cd lib && cargo nextest run plan_global_set_emits_one_exec_step 2>&1 | tail -5
```

Expected: FAIL (panics at `todo!()`)

- [ ] **Step 3: Add imports and helper to `lib/src/actions/git/config.rs`**

Add at the top of the file (after the existing `use` lines):

```rust
use crate::atoms::command::Exec;
use crate::atoms::git::GitConfigUnset;
use crate::utilities;
```

Add the `config_args()` helper method inside an `impl GitConfig` block (before `impl Action for GitConfig`):

```rust
impl GitConfig {
    fn config_args(&self) -> Vec<String> {
        match &self.scope {
            GitConfigScope::Global => vec!["config".into(), "--global".into()],
            GitConfigScope::Local => {
                let dir = self.directory.as_deref().unwrap_or(".");
                vec!["-C".into(), dir.into(), "config".into(), "--local".into()]
            }
            GitConfigScope::System => vec!["config".into(), "--system".into()],
        }
    }
}
```

- [ ] **Step 4: Implement set for a single key in `plan()`**

Replace the final `todo!()` in `plan()` with the implementation. The full `plan()` at this stage:

```rust
fn plan(&self, _manifest: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
    use anyhow::anyhow;

    if self.key.is_some() && self.settings.is_some() {
        return Err(anyhow!("git.config: 'key' and 'settings' are mutually exclusive"));
    }
    if self.key.is_none() && self.settings.is_none() {
        return Err(anyhow!("git.config: one of 'key' or 'settings' is required"));
    }
    if self.unset == Some(true) && self.settings.is_some() {
        return Err(anyhow!("git.config: 'unset' cannot be used with 'settings'"));
    }
    if self.unset == Some(true) && self.value.is_some() {
        return Err(anyhow!("git.config: 'unset' and 'value' are mutually exclusive"));
    }
    if matches!(self.scope, GitConfigScope::Local) && self.directory.is_none() {
        return Err(anyhow!("git.config: 'directory' is required for scope 'local'"));
    }
    if self.key.is_some() && self.value.is_none() && self.unset != Some(true) {
        return Err(anyhow!(
            "git.config: 'key' requires either 'value' (to set) or 'unset: true'"
        ));
    }

    let config_args = self.config_args();
    let privileged = matches!(self.scope, GitConfigScope::System);
    let privilege_provider = utilities::get_privilege_provider(contexts)
        .unwrap_or_else(|| "sudo".to_string());

    // Single key set
    if let (Some(key), Some(value)) = (&self.key, &self.value) {
        let mut args = config_args;
        args.push(key.clone());
        args.push(value.clone());
        return Ok(vec![Step {
            atom: Box::new(Exec {
                command: "git".into(),
                arguments: args,
                privileged,
                privilege_provider,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]);
    }

    todo!("unset and settings — implemented in later tasks")
}
```

- [ ] **Step 5: Run test — expect pass**

```bash
cd lib && cargo nextest run plan_global_set_emits_one_exec_step 2>&1 | tail -5
```

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/git/config.rs
git commit -m "feat(git-config): plan() global set via Exec atom"
```

---

### Task 6: GitConfig plan() — local + system set

**Files:**

- Modify: `lib/src/actions/git/config.rs`

- [ ] **Step 1: Write failing tests**

Add to the `tests` module:

```rust
    #[test]
    fn plan_local_set_includes_dash_c_and_local_flag() {
        let action = GitConfig {
            scope: GitConfigScope::Local,
            directory: Some("/tmp/repo".into()),
            key: Some("user.email".into()),
            value: Some("local@example.com".into()),
            ..Default::default()
        };
        let steps = plan(action).unwrap();
        assert_eq!(steps.len(), 1);
        let display = steps[0].atom.to_string();
        assert!(display.contains("-C"), "display: {display}");
        assert!(display.contains("/tmp/repo"), "display: {display}");
        assert!(display.contains("--local"), "display: {display}");
    }

    #[test]
    fn plan_system_set_is_privileged() {
        let action = GitConfig {
            scope: GitConfigScope::System,
            key: Some("credential.helper".into()),
            value: Some("osxkeychain".into()),
            ..Default::default()
        };
        let steps = plan(action).unwrap();
        assert_eq!(steps.len(), 1);
        let display = steps[0].atom.to_string();
        assert!(display.contains("privileged=true"), "display: {display}");
        assert!(display.contains("--system"), "display: {display}");
    }
```

- [ ] **Step 2: Run tests — expect pass**

```bash
cd lib && cargo nextest run plan_local_set plan_system_set 2>&1 | tail -5
```

Expected: PASS — these test existing code paths in `config_args()` which already handles all three scopes.

- [ ] **Step 3: Commit**

```bash
git add lib/src/actions/git/config.rs
git commit -m "test(git-config): local and system set scope tests"
```

---

### Task 7: GitConfig plan() — unset

**Files:**

- Modify: `lib/src/actions/git/config.rs`

- [ ] **Step 1: Write failing test**

Add to the `tests` module:

```rust
    #[test]
    fn plan_unset_emits_git_config_unset_step() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            key: Some("credential.helper".into()),
            unset: Some(true),
            ..Default::default()
        };
        let steps = plan(action).unwrap();
        assert_eq!(steps.len(), 1);
        let display = steps[0].atom.to_string();
        assert!(display.contains("GitConfigUnset"), "display: {display}");
        assert!(display.contains("credential.helper"), "display: {display}");
    }

    #[test]
    fn plan_local_unset_includes_dir_in_config_args() {
        let action = GitConfig {
            scope: GitConfigScope::Local,
            directory: Some("/tmp/repo".into()),
            key: Some("user.email".into()),
            unset: Some(true),
            ..Default::default()
        };
        let steps = plan(action).unwrap();
        assert_eq!(steps.len(), 1);
        let display = steps[0].atom.to_string();
        assert!(display.contains("GitConfigUnset"), "display: {display}");
        assert!(display.contains("user.email"), "display: {display}");
    }
```

- [ ] **Step 2: Run tests — expect failure**

```bash
cd lib && cargo nextest run plan_unset 2>&1 | tail -5
```

Expected: FAIL (panics at `todo!()`)

- [ ] **Step 3: Implement unset arm in `plan()`**

Replace `todo!("unset and settings...")` with:

```rust
    // Single key unset
    if self.unset == Some(true) {
        let key = self.key.as_ref().unwrap().clone();
        return Ok(vec![Step {
            atom: Box::new(GitConfigUnset {
                config_args,
                key,
                privileged,
                privilege_provider,
            }),
            initializers: vec![],
            finalizers: vec![],
        }]);
    }

    todo!("settings map — implemented in Task 8")
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cd lib && cargo nextest run plan_unset 2>&1 | tail -5
```

Expected: PASS (2 tests)

- [ ] **Step 5: Commit**

```bash
git add lib/src/actions/git/config.rs
git commit -m "feat(git-config): plan() unset via GitConfigUnset atom"
```

---

### Task 8: GitConfig plan() — settings map + summarize tests

**Files:**

- Modify: `lib/src/actions/git/config.rs`

- [ ] **Step 1: Write failing tests**

Add to the `tests` module:

```rust
    #[test]
    fn plan_settings_map_emits_one_step_per_key() {
        let mut settings = IndexMap::new();
        settings.insert("user.name".into(), "Bruce".into());
        settings.insert("user.email".into(), "bruce@example.com".into());
        settings.insert("core.autocrlf".into(), "false".into());
        let action = GitConfig {
            scope: GitConfigScope::Global,
            settings: Some(settings),
            ..Default::default()
        };
        let steps = plan(action).unwrap();
        assert_eq!(steps.len(), 3);
        // Verify order preserved — user.name first
        assert!(steps[0].atom.to_string().contains("user.name"));
        assert!(steps[1].atom.to_string().contains("user.email"));
        assert!(steps[2].atom.to_string().contains("core.autocrlf"));
    }

    #[test]
    fn summarize_single_set() {
        let action = GitConfig {
            scope: GitConfigScope::Global,
            key: Some("user.email".into()),
            value: Some("foo@bar.com".into()),
            ..Default::default()
        };
        let s = action.summarize();
        assert!(s.contains("global"), "summary: {s}");
        assert!(s.contains("user.email"), "summary: {s}");
        assert!(s.contains("foo@bar.com"), "summary: {s}");
    }

    #[test]
    fn summarize_unset() {
        let action = GitConfig {
            scope: GitConfigScope::System,
            key: Some("credential.helper".into()),
            unset: Some(true),
            ..Default::default()
        };
        let s = action.summarize();
        assert!(s.contains("Unset"), "summary: {s}");
        assert!(s.contains("system"), "summary: {s}");
        assert!(s.contains("credential.helper"), "summary: {s}");
    }

    #[test]
    fn summarize_settings_map() {
        let mut settings = IndexMap::new();
        settings.insert("user.name".into(), "Bruce".into());
        settings.insert("user.email".into(), "bruce@example.com".into());
        let action = GitConfig {
            scope: GitConfigScope::Local,
            directory: Some("/tmp/repo".into()),
            settings: Some(settings),
            ..Default::default()
        };
        let s = action.summarize();
        assert!(s.contains("2"), "summary: {s}");
        assert!(s.contains("local"), "summary: {s}");
    }
```

- [ ] **Step 2: Run tests — expect failure**

```bash
cd lib && cargo nextest run plan_settings summarize 2>&1 | tail -5
```

Expected: FAIL (settings panics at `todo!()`)

- [ ] **Step 3: Implement settings arm in `plan()`**

Replace `todo!("settings map...")` with:

```rust
    // Bulk settings map
    if let Some(ref settings) = self.settings {
        let steps = settings
            .iter()
            .map(|(key, value)| {
                let mut args = config_args.clone();
                args.push(key.clone());
                args.push(value.clone());
                Step {
                    atom: Box::new(Exec {
                        command: "git".into(),
                        arguments: args,
                        privileged,
                        privilege_provider: privilege_provider.clone(),
                        ..Default::default()
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                }
            })
            .collect();
        return Ok(steps);
    }

    // Unreachable: all branches above handle every valid combination.
    unreachable!("git.config: unhandled field combination (validation missed a case)")
```

- [ ] **Step 4: Run all action tests — expect pass**

```bash
cd lib && cargo nextest run actions::git::config 2>&1 | tail -10
```

Expected: all `error_*`, `plan_*`, and `summarize_*` tests PASS.

- [ ] **Step 5: Commit**

```bash
git add lib/src/actions/git/config.rs
git commit -m "feat(git-config): plan() settings map and summarize tests"
```

---

### Task 9: Register GitConfig in actions/mod.rs

**Files:**

- Modify: `lib/src/actions/mod.rs`

- [ ] **Step 1: Write failing tests**

In `lib/src/actions/mod.rs`, in the existing `tests` module, add to `all_major_action_variants_can_be_deserialized` (add the new entry and increment the count):

Add this entry to the YAML string before the closing `"#`:

```yaml
- action: git.config
  scope: global
  key: user.email
  value: test@example.com
```

Change `assert_eq!(20, manifest.actions.len());` to `assert_eq!(21, manifest.actions.len());`.

Also add a new test:

```rust
    #[test]
    fn git_config_alias_deserializes() {
        let yaml = r#"
actions:
  - action: git.cfg
    scope: global
    key: user.name
    value: Bruce
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(1, manifest.actions.len());
        assert!(matches!(manifest.actions[0], Actions::GitConfig(_)));
    }

    #[test]
    fn git_config_display_name() {
        let yaml = r#"
actions:
  - action: git.config
    scope: global
    key: user.email
    value: foo@bar.com
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(format!("{}", manifest.actions[0]), "git.config");
    }
```

Also uncomment the `deserialize_*` tests in `lib/src/actions/git/config.rs` that were commented in Task 4.

- [ ] **Step 2: Run tests — expect failure**

```bash
cd lib && cargo nextest run actions::mod 2>&1 | grep FAIL | head -5
```

Expected: compilation error — `Actions::GitConfig` doesn't exist yet.

- [ ] **Step 3: Register GitConfig in `lib/src/actions/mod.rs`**

Add the import alongside `git::GitClone`:

```rust
use git::{GitClone, GitConfig};
```

Add to the `Actions` enum after `GitClone`:

```rust
    #[serde(rename = "git.config", alias = "git.cfg")]
    GitConfig(ConditionalVariantAction<GitConfig>),
```

Add to `inner_ref()` after `Actions::GitClone(a) => a,`:

```rust
            Actions::GitConfig(a) => a,
```

Add to `Deref` after `Actions::GitClone(a) => a,`:

```rust
            Actions::GitConfig(a) => a,
```

Add to `Display` after `Actions::GitClone(_) => "git.clone",`:

```rust
            Actions::GitConfig(_) => "git.config",
```

- [ ] **Step 4: Run all tests — expect pass**

```bash
cd lib && cargo nextest run 2>&1 | tail -5
```

Expected: PASS (all tests)

- [ ] **Step 5: Run full test suite**

```bash
make test 2>&1 | tail -10
```

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/mod.rs lib/src/actions/git/config.rs
git commit -m "feat(git-config): register git.config / git.cfg in Actions enum"
```

---

### Task 10: Post-merge docs on main

> **Do NOT do these inside the worktree. Commit directly on main after the PR auto-merges.**

- [ ] **Step 1: Update `docs/superpowers/README.md`**

Change the `git-config` row in All Plans:

```
| 2026-05-25 | [git-config](plans/2026-05-25-git-config.md) | [spec](specs/2026-05-25-git-config-design.md) | Done |
```

- [ ] **Step 2: Add DONE banner to plan file**

Add at the top of `docs/superpowers/plans/2026-05-25-git-config.md`:

```
> **Status: DONE** — Implemented in PR #XX (2026-05-25)
```

- [ ] **Step 3: Update `CLAUDE.md` action catalog**

Add `git.config` row to the action table:

```
| `git.config`         | Set or unset gitconfig values | `scope` (`global`/`local`/`system`), `key` (Option), `value` (Option), `unset` (Option<bool>), `settings` (Option — ordered map of key/value pairs), `directory` (Option — required for `local` scope) |
```

- [ ] **Step 4: Commit on main**

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-05-25-git-config.md CLAUDE.md
git commit -m "docs: mark git-config Done, update action catalog"
git push origin main
```
