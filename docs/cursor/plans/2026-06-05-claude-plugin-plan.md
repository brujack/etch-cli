# claude.install / claude.upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `claude.install` and `claude.upgrade` actions to etch-cli, and update `etch update --claude` to auto-discover installed plugins instead of reading a static list.

**Architecture:** Two new actions in a new `claude` namespace under `lib/src/actions/claude/`. A shared parser in `mod.rs` extracts `name@marketplace` tokens from `claude plugins list` stdout. `ClaudeInstall` filters already-installed plugins at plan time (same pattern as `npm.install`). `ClaudeUpgrade` generates one upgrade step per installed plugin. `update_claude()` in `update.rs` switches from reading `config.plugins` to calling `capture("claude", &["plugins", "list"])` and parsing the output.

**Tech Stack:** Rust, serde_yaml_ng, anyhow, serial_test, tempfile

---

## File Locations

- Create: `lib/src/actions/claude/mod.rs`
- Create: `lib/src/actions/claude/install.rs`
- Create: `lib/src/actions/claude/upgrade.rs`
- Modify: `lib/src/actions/mod.rs` — 14 registration points (7 per action)
- Modify: `lib/src/config/mod.rs` — remove `plugins` field from `ClaudeUpdateConfig`
- Modify: `app/src/commands/update.rs` — auto-discover plugins in `update_claude()`

---

### Task 1: Shared plugin list parser in `lib/src/actions/claude/mod.rs`

**Files:**

- Create: `lib/src/actions/claude/mod.rs`

`claude plugins list` output looks like:

```
❯ superpowers@claude-plugins-official
❯ context7@upstash-context7
```

We need a pure function that takes the raw stdout string and returns full `name@marketplace` tokens.

- [ ] **Step 1: Write failing test**

Create `lib/src/actions/claude/mod.rs` with just the test module and a stub:

```rust
pub mod install;
pub mod upgrade;

pub(crate) use install::ClaudeInstall;
pub(crate) use upgrade::ClaudeUpgrade;

/// Parse installed plugin tokens from `claude plugins list` stdout.
/// Returns full `name@marketplace` tokens from lines starting with `❯ `.
pub(crate) fn parse_plugin_list(output: &str) -> Vec<String> {
    vec![] // stub
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_returns_full_tokens_from_list_output() {
        let output = "❯ superpowers@claude-plugins-official\n❯ context7@upstash-context7\n";
        let tokens = parse_plugin_list(output);
        assert_eq!(
            tokens,
            vec![
                "superpowers@claude-plugins-official",
                "context7@upstash-context7"
            ]
        );
    }
}
```

Note: `install` and `upgrade` modules don't exist yet — comment out those two lines to get the test file to compile with just the parser test. Or create empty stubs for install.rs and upgrade.rs.

- [ ] **Step 2: Create empty stubs for install.rs and upgrade.rs so mod.rs compiles**

`lib/src/actions/claude/install.rs`:

```rust
pub struct ClaudeInstall;
```

`lib/src/actions/claude/upgrade.rs`:

```rust
pub struct ClaudeUpgrade;
```

- [ ] **Step 3: Run test — confirm it fails**

```bash
cargo test -p etch-lib parse_returns_full_tokens 2>&1 | tail -20
```

Expected: FAIL — `parse_plugin_list` returns empty vec.

- [ ] **Step 4: Implement `parse_plugin_list`**

```rust
pub(crate) fn parse_plugin_list(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix('❯')
                .map(|rest| rest.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .collect()
}
```

- [ ] **Step 5: Run test — confirm pass**

```bash
cargo test -p etch-lib parse_returns_full_tokens 2>&1 | tail -10
```

Expected: PASS.

- [ ] **Step 6: Add boundary tests**

Add to the `mod tests` block:

```rust
#[test]
fn parse_returns_empty_for_empty_output() {
    assert!(parse_plugin_list("").is_empty());
}

#[test]
fn parse_skips_non_matching_lines() {
    let output = "Some header\n  other line\n❯ foo@bar\n";
    let tokens = parse_plugin_list(output);
    assert_eq!(tokens, vec!["foo@bar"]);
}
```

- [ ] **Step 7: Run all mod tests**

```bash
cargo test -p etch-lib actions::claude::tests 2>&1 | tail -15
```

Expected: 3 tests pass.

- [ ] **Step 8: Commit**

```bash
git add lib/src/actions/claude/
git commit -m "feat(claude): add claude namespace with parse_plugin_list helper"
```

---

### Task 2: `ClaudeInstall` action in `lib/src/actions/claude/install.rs`

**Files:**

- Modify: `lib/src/actions/claude/install.rs` (replace stub)

This follows the exact `NpmInstall` pattern. Key differences:

- Checks `claude plugins list` (not `npm list`) for installed packages
- Strips `@marketplace` suffix from plugin names before comparing
- Uses `streaming: true` on install Exec steps
- Each uninstalled plugin gets its own separate Exec step (one per plugin, not batched — `claude plugins install` takes one name at a time)

- [ ] **Step 1: Write the first failing test**

Replace `lib/src/actions/claude/install.rs` with:

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeInstall {
    pub name: Option<String>,
    #[serde(default)]
    pub list: Vec<String>,
}

impl ClaudeInstall {
    fn plugin_names(&self) -> Vec<String> {
        if !self.list.is_empty() {
            self.list.clone()
        } else if let Some(name) = &self.name {
            vec![name.clone()]
        } else {
            vec![]
        }
    }

    /// Strip `@marketplace` suffix to get the base plugin name.
    fn base_name(plugin: &str) -> &str {
        plugin.split('@').next().unwrap_or(plugin)
    }

    /// Return base names of currently installed plugins.
    /// Returns empty set if `claude plugins list` fails (fail-safe).
    fn installed_base_names() -> std::collections::HashSet<String> {
        std::process::Command::new("claude")
            .args(["plugins", "list"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
                super::parse_plugin_list(&stdout)
                    .into_iter()
                    .map(|tok| Self::base_name(&tok).to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Action for ClaudeInstall {
    fn summarize(&self) -> String {
        let names = self.plugin_names();
        if names.is_empty() {
            return String::from("Installing Claude plugins");
        }
        format!("Installing Claude plugin(s): {}", names.join(", "))
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let names = self.plugin_names();
        if names.is_empty() {
            bail!("claude.install requires either 'name' or 'list'");
        }

        let installed = Self::installed_base_names();
        let steps = names
            .into_iter()
            .filter(|n| !installed.contains(Self::base_name(n)))
            .map(|name| Step {
                atom: Box::new(Exec {
                    command: String::from("claude"),
                    arguments: vec![
                        String::from("plugins"),
                        String::from("install"),
                        name,
                    ],
                    streaming: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            })
            .collect();

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;
    use serial_test::serial;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = "- action: claude.install\n  name: superpowers\n";
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::ClaudeInstall(a)) => {
                assert_eq!(Some("superpowers".to_string()), a.action.name);
                assert!(a.action.list.is_empty());
            }
            _ => panic!("expected ClaudeInstall"),
        }
    }
}
```

Note: `Actions::ClaudeInstall` doesn't exist yet — the test will fail to compile. That's expected; we register the enum variant in Task 4. For now, run the tests that don't require the enum.

To avoid compile errors, skip the deserialization test for now and only test the pure helpers. Replace the test with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn base_name_strips_marketplace() {
        assert_eq!("superpowers", ClaudeInstall::base_name("superpowers@claude-plugins-official"));
        assert_eq!("foo", ClaudeInstall::base_name("foo@bar"));
        assert_eq!("plain", ClaudeInstall::base_name("plain"));
    }
}
```

- [ ] **Step 2: Run test — confirm fail**

```bash
cargo test -p etch-lib base_name_strips_marketplace 2>&1 | tail -15
```

Expected: compile error (module doesn't have Action impl yet) or assertion failure on stub.

Actually, the struct and impl are fully written now. Expected: PASS (base_name is a pure function).

- [ ] **Step 3: Run it**

```bash
cargo test -p etch-lib base_name_strips_marketplace 2>&1 | tail -10
```

Expected: 1 test passes.

- [ ] **Step 4: Add summarize tests**

```rust
#[test]
fn summarize_includes_plugin_name() {
    let action = ClaudeInstall {
        name: Some(String::from("superpowers")),
        list: vec![],
    };
    let s = action.summarize();
    assert!(s.contains("superpowers"), "expected 'superpowers' in: {s}");
}

#[test]
fn summarize_includes_all_list_plugins() {
    let action = ClaudeInstall {
        name: None,
        list: vec![String::from("superpowers"), String::from("context7")],
    };
    let s = action.summarize();
    assert!(s.contains("superpowers"), "got: {s}");
    assert!(s.contains("context7"), "got: {s}");
}

#[test]
fn summarize_with_no_plugins_returns_generic() {
    let s = ClaudeInstall::default().summarize();
    assert!(s.to_lowercase().contains("claude"), "got: {s}");
}
```

Run: `cargo test -p etch-lib summarize 2>&1 | tail -15` — expect 3 pass.

- [ ] **Step 5: Add plan error test and plan step tests**

```rust
#[test]
fn plan_errors_without_name_or_list() {
    let result = ClaudeInstall::default().plan(&Manifest::default(), &Contexts::default());
    assert!(result.is_err());
    let msg = result.err().unwrap().to_string();
    assert!(msg.contains("name") || msg.contains("list"), "got: {msg}");
}

#[test]
#[serial]
fn plan_returns_exec_for_uninstalled_plugin() {
    // Use a name that will never appear in `claude plugins list`
    let fake = "etch_cli_fake_plugin_zyx_xyz_test";
    let action = ClaudeInstall {
        name: Some(String::from(fake)),
        list: vec![],
    };
    let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
    assert_eq!(1, steps.len());
    let display = steps[0].atom.to_string();
    assert!(display.contains("claude"), "got: {display}");
    assert!(display.contains("install"), "got: {display}");
    assert!(display.contains(fake), "got: {display}");
}

#[test]
#[serial]
fn plan_generates_step_when_claude_not_in_path() {
    let old = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", "/nonexistent");
    let action = ClaudeInstall {
        name: Some(String::from("superpowers")),
        list: vec![],
    };
    let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
    std::env::set_var("PATH", old);
    // fail-safe: claude not found → treat all as uninstalled → generate step
    assert_eq!(1, steps.len());
}
```

Run: `cargo test -p etch-lib plan_returns_exec_for_uninstalled_plugin plan_errors_without_name_or_list plan_generates_step 2>&1 | tail -15`

- [ ] **Step 6: Add fake-claude skip test**

```rust
#[test]
#[serial]
fn plan_skips_already_installed_plugin() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    // Fake claude that reports "❯ superpowers@claude-plugins-official"
    let fake = tmp.path().join("claude");
    std::fs::write(&fake, concat!(
        "#!/bin/sh\n",
        "printf '❯ superpowers@claude-plugins-official\\n'\n",
        "exit 0\n",
    )).unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

    let old = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{old}", tmp.path().display()));

    let action = ClaudeInstall {
        name: Some(String::from("superpowers")),
        list: vec![],
    };
    let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
    std::env::set_var("PATH", old);

    assert!(steps.is_empty(), "expected no steps — plugin already installed");
}

#[test]
#[serial]
fn plan_returns_empty_when_all_installed() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let fake = tmp.path().join("claude");
    std::fs::write(&fake, concat!(
        "#!/bin/sh\n",
        "printf '❯ superpowers@official\\n❯ context7@upstash\\n'\n",
        "exit 0\n",
    )).unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

    let old = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{old}", tmp.path().display()));

    let action = ClaudeInstall {
        name: None,
        list: vec![String::from("superpowers"), String::from("context7")],
    };
    let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
    std::env::set_var("PATH", old);

    assert!(steps.is_empty(), "expected no steps — all installed");
}

#[test]
#[serial]
fn plan_handles_marketplace_suffix_in_name() {
    // name: foo@bar → base name "foo" compared against installed base names
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let fake = tmp.path().join("claude");
    std::fs::write(&fake, concat!(
        "#!/bin/sh\n",
        "printf '❯ foo@bar\\n'\n",
        "exit 0\n",
    )).unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

    let old = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{old}", tmp.path().display()));

    let action = ClaudeInstall {
        name: Some(String::from("foo@bar")),
        list: vec![],
    };
    let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
    std::env::set_var("PATH", old);

    assert!(steps.is_empty(), "expected no steps — foo@bar already installed as foo");
}
```

- [ ] **Step 7: Run all install tests**

```bash
cargo test -p etch-lib actions::claude::install 2>&1 | tail -20
```

Expected: all 11 tests pass (the deserialization test will be added after Task 4).

- [ ] **Step 8: Commit**

```bash
git add lib/src/actions/claude/install.rs
git commit -m "feat(claude): add ClaudeInstall action with plan-time idempotency"
```

---

### Task 3: `ClaudeUpgrade` action in `lib/src/actions/claude/upgrade.rs`

**Files:**

- Modify: `lib/src/actions/claude/upgrade.rs` (replace stub)

`ClaudeUpgrade` has no fields. It discovers installed plugins at plan time and generates one Exec step per plugin.

- [ ] **Step 1: Write first failing test**

Replace `lib/src/actions/claude/upgrade.rs` with:

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeUpgrade {}

impl ClaudeUpgrade {
    fn installed_plugins() -> Vec<String> {
        std::process::Command::new("claude")
            .args(["plugins", "list"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
                super::parse_plugin_list(&stdout)
            })
            .unwrap_or_default()
    }
}

impl Action for ClaudeUpgrade {
    fn summarize(&self) -> String {
        String::from("Upgrading all installed Claude plugins")
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let plugins = Self::installed_plugins();
        let steps = plugins
            .into_iter()
            .map(|token| Step {
                atom: Box::new(Exec {
                    command: String::from("claude"),
                    arguments: vec![
                        String::from("plugins"),
                        String::from("update"),
                        token,
                    ],
                    streaming: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            })
            .collect();

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn summarize_returns_string() {
        let s = ClaudeUpgrade::default().summarize();
        assert!(!s.is_empty());
        assert!(s.to_lowercase().contains("claude"), "got: {s}");
    }

    #[test]
    #[serial]
    fn plan_returns_exec_for_each_installed_plugin() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake = tmp.path().join("claude");
        std::fs::write(&fake, concat!(
            "#!/bin/sh\n",
            "printf '❯ superpowers@official\\n❯ context7@upstash\\n'\n",
            "exit 0\n",
        )).unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old}", tmp.path().display()));

        let steps = ClaudeUpgrade::default()
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old);

        assert_eq!(2, steps.len());
        let d0 = steps[0].atom.to_string();
        let d1 = steps[1].atom.to_string();
        assert!(d0.contains("superpowers@official"), "got: {d0}");
        assert!(d1.contains("context7@upstash"), "got: {d1}");
        assert!(d0.contains("update"), "got: {d0}");
    }

    #[test]
    #[serial]
    fn plan_returns_empty_when_none_installed() {
        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", "/nonexistent");

        let steps = ClaudeUpgrade::default()
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old);

        assert!(steps.is_empty(), "expected no steps when claude not found");
    }

    #[test]
    #[serial]
    fn plan_returns_empty_when_claude_not_in_path() {
        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", "/nonexistent");

        let steps = ClaudeUpgrade::default()
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old);

        assert!(steps.is_empty(), "expected no steps when claude not found");
    }
}
```

- [ ] **Step 2: Run failing test**

```bash
cargo test -p etch-lib summarize_returns_string 2>&1 | tail -10
```

Expected: compile error because `ClaudeUpgrade` stub is `pub struct ClaudeUpgrade;` not `pub struct ClaudeUpgrade {}`. Fix the stub and run again.

Actually the full implementation is already written above — this should pass immediately once the file is in place.

- [ ] **Step 3: Run all upgrade tests**

```bash
cargo test -p etch-lib actions::claude::upgrade 2>&1 | tail -20
```

Expected: 4 tests pass.

- [ ] **Step 4: Commit**

```bash
git add lib/src/actions/claude/upgrade.rs
git commit -m "feat(claude): add ClaudeUpgrade action"
```

---

### Task 4: Register `ClaudeInstall` and `ClaudeUpgrade` in `lib/src/actions/mod.rs`

**Files:**

- Modify: `lib/src/actions/mod.rs`

Seven registration points per action (14 total). `claude` is a new namespace — add `mod claude;` and `use` line.

- [ ] **Step 1: Write failing deserialization tests** (in `install.rs` and `upgrade.rs`)

Add to `install.rs` tests:

```rust
#[test]
fn it_can_be_deserialized() {
    let yaml = "- action: claude.install\n  name: superpowers\n";
    let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
    match actions.pop() {
        Some(Actions::ClaudeInstall(a)) => {
            assert_eq!(Some("superpowers".to_string()), a.action.name);
            assert!(a.action.list.is_empty());
        }
        _ => panic!("expected ClaudeInstall"),
    }
}

#[test]
fn it_can_be_deserialized_with_list() {
    let yaml = concat!(
        "- action: claude.install\n",
        "  list:\n",
        "    - superpowers\n",
        "    - context7\n",
    );
    let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
    match actions.pop() {
        Some(Actions::ClaudeInstall(a)) => {
            assert_eq!(vec!["superpowers", "context7"], a.action.list);
            assert!(a.action.name.is_none());
        }
        _ => panic!("expected ClaudeInstall"),
    }
}
```

Add to `upgrade.rs` tests:

```rust
#[test]
fn it_can_be_deserialized() {
    use crate::actions::Actions;
    let yaml = "- action: claude.upgrade\n";
    let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
    match actions.pop() {
        Some(Actions::ClaudeUpgrade(_)) => {}
        _ => panic!("expected ClaudeUpgrade"),
    }
}
```

- [ ] **Step 2: Run — confirm compile error (Actions::ClaudeInstall doesn't exist)**

```bash
cargo test -p etch-lib it_can_be_deserialized 2>&1 | head -20
```

Expected: compile error `no variant ClaudeInstall on Actions`.

- [ ] **Step 3: Add the 7 registration points for `ClaudeInstall` in `mod.rs`**

In `lib/src/actions/mod.rs`:

**1.** Add module declaration near other `mod` lines (alphabetical order):

```rust
mod claude;
```

**2.** Add import (find where other `use` lines are for action structs):

```rust
use claude::{ClaudeInstall, ClaudeUpgrade};
```

**3.** Add enum variant (find the `Actions` enum, add near other `claude`-adjacent entries):

```rust
#[serde(rename = "claude.install")]
ClaudeInstall(ConditionalVariantAction<ClaudeInstall>),
```

**4.** Add `inner_ref()` match arm:

```rust
Actions::ClaudeInstall(a) => a,
```

**5.** Add `notify()` match arm:

```rust
Actions::ClaudeInstall(a) => &a.notify,
```

**6.** Add `Deref` match arm:

```rust
Actions::ClaudeInstall(a) => a,
```

**7.** Add `Display::fmt` match arm:

```rust
Actions::ClaudeInstall(_) => "claude.install",
```

Then add the same 7 points for `ClaudeUpgrade`:

```rust
// enum variant:
#[serde(rename = "claude.upgrade")]
ClaudeUpgrade(ConditionalVariantAction<ClaudeUpgrade>),
// inner_ref:
Actions::ClaudeUpgrade(a) => a,
// notify:
Actions::ClaudeUpgrade(a) => &a.notify,
// Deref:
Actions::ClaudeUpgrade(a) => a,
// Display:
Actions::ClaudeUpgrade(_) => "claude.upgrade",
```

- [ ] **Step 4: Run deserialization tests**

```bash
cargo test -p etch-lib it_can_be_deserialized 2>&1 | tail -15
```

Expected: 3 tests pass (install×2, upgrade×1).

- [ ] **Step 5: Run full lib test suite**

```bash
cargo test -p etch-lib 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/mod.rs lib/src/actions/claude/install.rs lib/src/actions/claude/upgrade.rs
git commit -m "feat(claude): register ClaudeInstall and ClaudeUpgrade in Actions enum"
```

---

### Task 5: Update `ClaudeUpdateConfig` in `lib/src/config/mod.rs`

**Files:**

- Modify: `lib/src/config/mod.rs`

`ClaudeUpdateConfig` does not use `#[serde(deny_unknown_fields)]` and neither does `UpdateConfig` or `Config`. So `plugins:` in an existing `etch.yaml` will be silently ignored after removal (serde's default behavior). We can delete the field outright.

- [ ] **Step 1: Write a test that will fail if `plugins` field is still treated as meaningful**

Add to `lib/src/config/mod.rs` tests:

```rust
#[test]
fn claude_update_config_has_no_plugins_field_effect() {
    // After removal, ClaudeUpdateConfig should deserialize without plugins:
    // and existing etch.yaml files with plugins: should not error (no deny_unknown_fields).
    let yaml = r#"
update:
    claude:
        npm_globals:
            - firecrawl-cli
"#;
    let config: Config = serde_yaml_ng::from_str(yaml).unwrap();
    let claude = config.update.claude.as_ref().unwrap();
    assert_eq!(claude.npm_globals, vec!["firecrawl-cli"]);
}
```

Run: `cargo test -p etch-lib claude_update_config_has_no_plugins_field_effect 2>&1 | tail -10`
Expected: PASS (this test passes even before the field is removed — it's a regression test).

- [ ] **Step 2: Remove `plugins` field from `ClaudeUpdateConfig`**

In `lib/src/config/mod.rs`:

```rust
// Before:
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ClaudeUpdateConfig {
    #[serde(default)]
    pub plugins: Vec<String>,
    #[serde(default)]
    pub npm_globals: Vec<String>,
}

// After:
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ClaudeUpdateConfig {
    #[serde(default)]
    pub npm_globals: Vec<String>,
}
```

- [ ] **Step 3: Fix broken tests**

Existing tests reference `claude.plugins` — update them:

`claude_update_config_default_has_empty_vecs`: remove the `assert!(c.plugins.is_empty())` line.

`update_config_deserialize_full_yaml`: remove the `plugins:` from the YAML and the assertions on `claude.plugins`.

- [ ] **Step 4: Run config tests**

```bash
cargo test -p etch-lib config 2>&1 | tail -15
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add lib/src/config/mod.rs
git commit -m "feat(claude): remove plugins field from ClaudeUpdateConfig"
```

---

### Task 6: Auto-discover plugins in `update_claude()` in `app/src/commands/update.rs`

**Files:**

- Modify: `app/src/commands/update.rs`

Replace the static `config.plugins` iteration with runtime discovery via `claude plugins list`.

- [ ] **Step 1: Write a test for the parsing logic**

The parser is pure — extract it as a testable helper. Add to `update.rs` (outside the `#[cfg(not(tarpaulin_include))]` block):

```rust
/// Extract full `name@marketplace` tokens from `claude plugins list` stdout.
/// Lines matching `❯ <token>` are kept; everything else is ignored.
fn parse_claude_plugin_tokens(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix('❯')
                .map(|rest| rest.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .collect()
}
```

Add test to `update.rs` test module:

```rust
#[test]
fn parse_claude_plugin_tokens_extracts_tokens() {
    let output = "❯ superpowers@official\n❯ context7@upstash\nsome other line\n";
    let tokens = parse_claude_plugin_tokens(output);
    assert_eq!(
        tokens,
        vec!["superpowers@official", "context7@upstash"]
    );
}

#[test]
fn parse_claude_plugin_tokens_empty_output() {
    assert!(parse_claude_plugin_tokens("").is_empty());
}
```

Run: `cargo test -p etch-app parse_claude_plugin_tokens 2>&1 | tail -10`
Expected: 2 tests pass.

- [ ] **Step 2: Rewrite `update_claude()`**

```rust
#[cfg(not(tarpaulin_include))]
fn update_claude(_config: &ClaudeUpdateConfig) -> UpdateStepResult {
    if !has_cmd("claude") {
        return skip_result("claude", "claude not installed");
    }

    let list_output = capture("claude", &["plugins", "list"]);
    let plugins = parse_claude_plugin_tokens(&list_output.join("\n"));

    if plugins.is_empty() {
        return UpdateStepResult {
            name: "claude",
            status: StepStatus::Ok("no plugins installed".to_string()),
        };
    }

    let pre_versions: Vec<String> = list_output
        .iter()
        .filter(|l| l.contains("Version:"))
        .cloned()
        .collect();

    let mut fail_count = 0usize;
    for plugin in &plugins {
        let exit = run_cmd("claude", &["plugins", "update", plugin.as_str()]);
        if exit != 0 {
            warn!("claude plugin update failed for {plugin}");
            fail_count += 1;
        }
    }

    let post_versions: Vec<String> = capture("claude", &["plugins", "list"])
        .into_iter()
        .filter(|l| l.contains("Version:"))
        .collect();
    let updated_count = diff_lines(&pre_versions, &post_versions).len();

    if fail_count > 0 && updated_count == 0 {
        return UpdateStepResult {
            name: "claude",
            status: StepStatus::Fail(format!("{fail_count} plugin(s) failed")),
        };
    }

    let detail = if updated_count > 0 {
        format!("{updated_count} plugin(s) updated")
    } else {
        "no changes".to_string()
    };
    UpdateStepResult {
        name: "claude",
        status: StepStatus::Ok(detail),
    }
}
```

Note: `capture()` returns `Vec<String>` of non-empty lines. Join with `\n` to feed to the pure parser.

Also update the `capture()` signature usage — `capture()` is already `#[cfg(not(tarpaulin_include))]`. The pure `parse_claude_plugin_tokens` is NOT guarded so it stays testable.

- [ ] **Step 3: Fix `update_claude` call site**

The call site passes `claude_cfg: &ClaudeUpdateConfig`. The function signature still takes `_config: &ClaudeUpdateConfig` — no change needed at the call site. The `_config` is accepted but plugins are discovered at runtime instead.

- [ ] **Step 4: Run full app tests**

```bash
cargo test -p etch-app 2>&1 | tail -20
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add app/src/commands/update.rs
git commit -m "feat(claude): update_claude() discovers plugins via claude plugins list"
```

---

### Task 7: Pre-PR verification

**Files:** none — verification only

- [ ] **Step 1: Run full test suite**

```bash
make test 2>&1 | tail -30
```

Expected: all tests pass, lint clean.

- [ ] **Step 2: Run coverage check**

```bash
cargo tarpaulin -p etch-lib --exclude-files 'jsonschemagen/*' 2>/dev/null | tail -5
```

Verify macOS number ≥ 83%. Linux CI gate is 81% — new tests should push coverage up, not down.

- [ ] **Step 3: Run pr-review skill**

Invoke the `pr-review` skill — verdict must be PASS before pushing.

- [ ] **Step 4: Push and open PR**

```bash
git push origin feat/claude-plugin
gh pr create --repo brujack/etch-cli \
  --title "feat: add claude.install and claude.upgrade actions" \
  --body "..."
```

---

### Task 8: Post-merge closeout (on master — NOT in worktree)

_Do this directly on master after the PR merges — not inside the worktree._

- [ ] Update `docs/cursor/README.md` — mark row Done
- [ ] Add `> **Status: DONE**` banner to `docs/cursor/plans/2026-06-05-claude-plugin-plan.md`
- [ ] Update `docs/superpowers/README.md` — move `claude.plugin action` backlog item to All Plans as Done
- [ ] Add `claude.install` and `claude.upgrade` to README.md action catalog
- [ ] Run `docs` skill then `learnings` skill
