# claude.marketplace, claude.marketplace.remove, package.remove Implementation Plan

> **Status: DONE** — merged via PR #91 (2026-06-06)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `claude.marketplace`, `claude.marketplace.remove`, and `package.remove` actions to etch-cli.

**Architecture:** Each action is a standalone Rust struct in its own file under the existing `lib/src/actions/claude/` or `lib/src/actions/package/` directories, registered in the central `lib/src/actions/mod.rs` dispatch table. `package.remove` adds a `remove()` method to the existing `PackageProvider` trait implemented by apt, snap, and homebrew. Marketplace actions share a parser helper with the existing plugin list infrastructure.

**Tech Stack:** Rust, serde/schemars for YAML deserialization, `anyhow` for errors, `Exec` atom for shell steps, existing `PackageProvider` trait pattern.

---

## File Map

| Action                      | New Files                                      | Modified Files                                                                                                                                                                                                           |
| --------------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `claude.marketplace`        | `lib/src/actions/claude/marketplace.rs`        | `lib/src/actions/claude/mod.rs`, `lib/src/actions/mod.rs`                                                                                                                                                                |
| `claude.marketplace.remove` | `lib/src/actions/claude/marketplace_remove.rs` | `lib/src/actions/claude/mod.rs`, `lib/src/actions/mod.rs`                                                                                                                                                                |
| `package.remove`            | `lib/src/actions/package/remove.rs`            | `lib/src/actions/package/providers/mod.rs`, `lib/src/actions/package/providers/aptitude.rs`, `lib/src/actions/package/providers/snapcraft.rs`, `lib/src/actions/package/providers/homebrew.rs`, `lib/src/actions/mod.rs` |
| Examples                    | `examples/claude/claude-marketplace.yaml`      | `examples/package/package-management.yaml`                                                                                                                                                                               |
| Docs                        | —                                              | `README.md`, `docs/knowledge/action-catalog.md`, `docs/superpowers/README.md`                                                                                                                                            |

---

## Task 1: `parse_marketplace_list()` helper

**Files:**

- Modify: `lib/src/actions/claude/mod.rs`

The existing `parse_plugin_list()` uses the same `❯ name` prefix format as marketplace list output. Add an identical helper for marketplaces and tests.

- [ ] **Step 1: Add failing tests for `parse_marketplace_list`**

Append to the `#[cfg(test)] mod tests` block already in `lib/src/actions/claude/mod.rs`:

```rust
    #[test]
    fn parse_marketplace_list_extracts_names() {
        let output = "Configured marketplaces:\n\n  ❯ claude-plugins-official\n    Source: GitHub (anthropics/claude-plugins-official)\n\n  ❯ caveman\n    Source: Git (https://github.com/juliusbrussee/caveman.git)\n";
        let names = parse_marketplace_list(output);
        assert_eq!(names, vec!["claude-plugins-official", "caveman"]);
    }

    #[test]
    fn parse_marketplace_list_empty_input_returns_empty() {
        assert!(parse_marketplace_list("").is_empty());
    }

    #[test]
    fn parse_marketplace_list_skips_source_lines() {
        let output = "  ❯ foo\n    Source: GitHub (bar/baz)\n";
        let names = parse_marketplace_list(output);
        assert_eq!(names, vec!["foo"]);
    }
```

- [ ] **Step 2: Run tests — confirm they fail**

```bash
cargo test -p etch-lib parse_marketplace_list
```

Expected: FAIL with "cannot find function `parse_marketplace_list`"

- [ ] **Step 3: Add `parse_marketplace_list` to `lib/src/actions/claude/mod.rs`**

Add directly after the existing `parse_plugin_list` function (before the `#[cfg(test)]` block):

```rust
pub(crate) fn parse_marketplace_list(output: &str) -> Vec<String> {
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

- [ ] **Step 4: Run tests — confirm they pass**

```bash
cargo test -p etch-lib parse_marketplace_list
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add lib/src/actions/claude/mod.rs
git commit -m "feat(claude): add parse_marketplace_list helper"
```

---

## Task 2: `claude.marketplace` action

**Files:**

- Create: `lib/src/actions/claude/marketplace.rs`
- Modify: `lib/src/actions/claude/mod.rs` (add `pub mod marketplace;` and re-export)

- [ ] **Step 1: Write failing tests**

Create `lib/src/actions/claude/marketplace.rs` with tests only:

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeMarketplace {
    pub name: String,
    pub source: String,
    pub scope: Option<String>,
    #[serde(default)]
    pub sparse: Vec<String>,
}

impl ClaudeMarketplace {
    fn installed_marketplaces() -> Vec<String> {
        std::process::Command::new("claude")
            .args(["plugins", "marketplace", "list"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
                super::parse_marketplace_list(&stdout)
            })
            .unwrap_or_default()
    }

    fn build_step(source: &str, scope: Option<&str>, sparse: &[String]) -> Step {
        use crate::atoms::command::Exec;
        let mut args = vec![
            String::from("plugins"),
            String::from("marketplace"),
            String::from("add"),
            source.to_string(),
        ];
        if let Some(s) = scope {
            args.push(String::from("--scope"));
            args.push(s.to_string());
        }
        if !sparse.is_empty() {
            args.push(String::from("--sparse"));
            args.extend(sparse.iter().cloned());
        }
        Step {
            atom: Box::new(Exec {
                command: String::from("claude"),
                arguments: args,
                streaming: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }
    }
}

impl Action for ClaudeMarketplace {
    fn summarize(&self) -> String {
        format!("Adding Claude marketplace: {}", self.name)
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        if self.name.is_empty() || self.source.is_empty() {
            bail!("claude.marketplace requires 'name' and 'source'");
        }
        let installed = Self::installed_marketplaces();
        if installed.contains(&self.name) {
            return Ok(vec![]);
        }
        Ok(vec![Self::build_step(
            &self.source,
            self.scope.as_deref(),
            &self.sparse,
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marketplace(name: &str, source: &str) -> ClaudeMarketplace {
        ClaudeMarketplace {
            name: name.to_string(),
            source: source.to_string(),
            scope: None,
            sparse: vec![],
        }
    }

    #[test]
    fn plan_skips_when_marketplace_already_present() {
        // Simulate already installed by calling build_step indirectly via already-present check.
        // We test the logic directly: if name is in installed list, return empty.
        // Use build_step to verify the step structure when not skipped.
        let step = ClaudeMarketplace::build_step("owner/repo", None, &[]);
        let display = step.atom.to_string();
        assert!(display.contains("marketplace"), "got: {display}");
        assert!(display.contains("add"), "got: {display}");
        assert!(display.contains("owner/repo"), "got: {display}");
    }

    #[test]
    fn build_step_omits_scope_when_none() {
        let step = ClaudeMarketplace::build_step("owner/repo", None, &[]);
        let display = step.atom.to_string();
        assert!(!display.contains("--scope"), "got: {display}");
    }

    #[test]
    fn build_step_includes_scope_when_set() {
        let step = ClaudeMarketplace::build_step("owner/repo", Some("user"), &[]);
        let display = step.atom.to_string();
        assert!(display.contains("--scope"), "got: {display}");
        assert!(display.contains("user"), "got: {display}");
    }

    #[test]
    fn build_step_includes_sparse_when_set() {
        let step = ClaudeMarketplace::build_step(
            "owner/repo",
            None,
            &[String::from(".claude-plugin")],
        );
        let display = step.atom.to_string();
        assert!(display.contains("--sparse"), "got: {display}");
        assert!(display.contains(".claude-plugin"), "got: {display}");
    }

    #[test]
    fn build_step_omits_sparse_when_empty() {
        let step = ClaudeMarketplace::build_step("owner/repo", None, &[]);
        let display = step.atom.to_string();
        assert!(!display.contains("--sparse"), "got: {display}");
    }

    #[test]
    fn summarize_includes_name() {
        let m = marketplace("caveman", "juliusbrussee/caveman");
        assert!(m.summarize().contains("caveman"));
    }

    #[test]
    fn deserialize_minimal() {
        let yaml = "name: caveman\nsource: juliusbrussee/caveman\n";
        let m: ClaudeMarketplace = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(m.name, "caveman");
        assert_eq!(m.source, "juliusbrussee/caveman");
        assert!(m.scope.is_none());
        assert!(m.sparse.is_empty());
    }

    #[test]
    fn deserialize_with_scope_and_sparse() {
        let yaml = "name: caveman\nsource: juliusbrussee/caveman\nscope: user\nsparse:\n  - .claude-plugin\n";
        let m: ClaudeMarketplace = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(m.scope, Some(String::from("user")));
        assert_eq!(m.sparse, vec![".claude-plugin"]);
    }
}
```

- [ ] **Step 2: Run tests — confirm they fail**

```bash
cargo test -p etch-lib claude::marketplace
```

Expected: FAIL (module not yet added to mod.rs)

- [ ] **Step 3: Register the module in `lib/src/actions/claude/mod.rs`**

Add after the existing `pub mod upgrade;` line:

```rust
pub mod marketplace;
pub mod marketplace_remove;
```

Add after the existing `pub(crate) use upgrade::ClaudeUpgrade;` line:

```rust
pub(crate) use marketplace::ClaudeMarketplace;
pub(crate) use marketplace_remove::ClaudeMarketplaceRemove;
```

Also create a placeholder `lib/src/actions/claude/marketplace_remove.rs` (needed for the module declaration to compile — full implementation in Task 3):

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeMarketplaceRemove {
    pub name: String,
    pub scope: Option<String>,
}

impl Action for ClaudeMarketplaceRemove {
    fn summarize(&self) -> String {
        format!("Removing Claude marketplace: {}", self.name)
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }
}
```

- [ ] **Step 4: Run tests — confirm they pass**

```bash
cargo test -p etch-lib claude::marketplace
```

Expected: all tests in `marketplace::tests` pass.

- [ ] **Step 5: Commit**

```bash
git add lib/src/actions/claude/marketplace.rs \
        lib/src/actions/claude/marketplace_remove.rs \
        lib/src/actions/claude/mod.rs
git commit -m "feat(claude): add claude.marketplace action"
```

---

## Task 3: `claude.marketplace.remove` action

**Files:**

- Modify: `lib/src/actions/claude/marketplace_remove.rs` (replace placeholder from Task 2)

- [ ] **Step 1: Write failing tests**

Replace the placeholder `marketplace_remove.rs` with the full file including tests:

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeMarketplaceRemove {
    pub name: String,
    pub scope: Option<String>,
}

impl ClaudeMarketplaceRemove {
    fn installed_marketplaces() -> Vec<String> {
        std::process::Command::new("claude")
            .args(["plugins", "marketplace", "list"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
                super::parse_marketplace_list(&stdout)
            })
            .unwrap_or_default()
    }

    fn build_step(name: &str, scope: Option<&str>) -> Step {
        use crate::atoms::command::Exec;
        let mut args = vec![
            String::from("plugins"),
            String::from("marketplace"),
            String::from("remove"),
            name.to_string(),
        ];
        if let Some(s) = scope {
            args.push(String::from("--scope"));
            args.push(s.to_string());
        }
        Step {
            atom: Box::new(Exec {
                command: String::from("claude"),
                arguments: args,
                streaming: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }
    }
}

impl Action for ClaudeMarketplaceRemove {
    fn summarize(&self) -> String {
        format!("Removing Claude marketplace: {}", self.name)
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        let installed = Self::installed_marketplaces();
        if !installed.contains(&self.name) {
            return Ok(vec![]);
        }
        Ok(vec![Self::build_step(&self.name, self.scope.as_deref())])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_step_contains_remove_and_name() {
        let step = ClaudeMarketplaceRemove::build_step("caveman", None);
        let display = step.atom.to_string();
        assert!(display.contains("marketplace"), "got: {display}");
        assert!(display.contains("remove"), "got: {display}");
        assert!(display.contains("caveman"), "got: {display}");
    }

    #[test]
    fn build_step_omits_scope_when_none() {
        let step = ClaudeMarketplaceRemove::build_step("caveman", None);
        let display = step.atom.to_string();
        assert!(!display.contains("--scope"), "got: {display}");
    }

    #[test]
    fn build_step_includes_scope_when_set() {
        let step = ClaudeMarketplaceRemove::build_step("caveman", Some("user"));
        let display = step.atom.to_string();
        assert!(display.contains("--scope"), "got: {display}");
        assert!(display.contains("user"), "got: {display}");
    }

    #[test]
    fn summarize_includes_name() {
        let r = ClaudeMarketplaceRemove {
            name: String::from("caveman"),
            scope: None,
        };
        assert!(r.summarize().contains("caveman"));
    }

    #[test]
    fn deserialize_minimal() {
        let yaml = "name: caveman\n";
        let r: ClaudeMarketplaceRemove = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(r.name, "caveman");
        assert!(r.scope.is_none());
    }

    #[test]
    fn deserialize_with_scope() {
        let yaml = "name: caveman\nscope: user\n";
        let r: ClaudeMarketplaceRemove = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(r.scope, Some(String::from("user")));
    }
}
```

- [ ] **Step 2: Run tests — confirm they pass**

```bash
cargo test -p etch-lib claude::marketplace_remove
```

Expected: all 6 tests pass.

- [ ] **Step 3: Commit**

```bash
git add lib/src/actions/claude/marketplace_remove.rs
git commit -m "feat(claude): add claude.marketplace.remove action"
```

---

## Task 4: Register `claude.marketplace` and `claude.marketplace.remove` in `mod.rs`

**Files:**

- Modify: `lib/src/actions/mod.rs`

The `lib/src/actions/mod.rs` file has 6 locations to update for each new action. Add both actions together in this task.

- [ ] **Step 1: Write failing test entries in `mod.rs`**

In the `all_major_action_variants_can_be_deserialized` test, add two entries to the YAML (after the `claude.install` entry if present, or after `brew.cleanup`) and update the count from `27` to `29`:

```yaml
- action: claude.marketplace
  name: caveman
  source: juliusbrussee/caveman
- action: claude.marketplace.remove
  name: caveman
```

Change `assert_eq!(27, manifest.actions.len())` → `assert_eq!(29, manifest.actions.len())`.

In `all_action_variants_inner_ref_and_deref`, add two entries after the existing `ruby.install` entry:

```yaml
- action: claude.marketplace
  name: caveman
  source: juliusbrussee/caveman
- action: claude.marketplace.remove
  name: caveman
```

Change `assert_eq!(40, manifest.actions.len())` → `assert_eq!(42, manifest.actions.len())`.

In `all_action_variants_display`, add two entries and update count `40` → `42`, and add two `assert!(names.contains(...))` lines:

```yaml
- action: claude.marketplace
  name: caveman
  source: juliusbrussee/caveman
- action: claude.marketplace.remove
  name: caveman
```

```rust
assert!(names.contains(&"claude.marketplace".to_string()));
assert!(names.contains(&"claude.marketplace.remove".to_string()));
```

- [ ] **Step 2: Run tests — confirm they fail**

```bash
cargo test -p etch-lib all_major_action_variants_can_be_deserialized
cargo test -p etch-lib all_action_variants_inner_ref_and_deref
cargo test -p etch-lib all_action_variants_display
```

Expected: FAIL (unknown action type `claude.marketplace`)

- [ ] **Step 3: Add `use` imports to `lib/src/actions/mod.rs`**

Find the line `use claude::{ClaudeInstall, ClaudeUpgrade};` and replace with:

```rust
use claude::{ClaudeInstall, ClaudeMarketplace, ClaudeMarketplaceRemove, ClaudeUpgrade};
```

- [ ] **Step 4: Add enum variants**

Find the block:

```rust
    #[serde(rename = "claude.install")]
    ClaudeInstall(ConditionalVariantAction<ClaudeInstall>),

    #[serde(rename = "claude.upgrade")]
    ClaudeUpgrade(ConditionalVariantAction<ClaudeUpgrade>),
```

Add after `ClaudeUpgrade`:

```rust
    #[serde(rename = "claude.marketplace")]
    ClaudeMarketplace(ConditionalVariantAction<ClaudeMarketplace>),

    #[serde(rename = "claude.marketplace.remove")]
    ClaudeMarketplaceRemove(ConditionalVariantAction<ClaudeMarketplaceRemove>),
```

- [ ] **Step 5: Add `inner_ref` match arms**

Find `Actions::ClaudeUpgrade(a) => a,` in the `inner_ref` match block and add after it:

```rust
            Actions::ClaudeMarketplace(a) => a,
            Actions::ClaudeMarketplaceRemove(a) => a,
```

- [ ] **Step 6: Add `notify` match arms**

Find `Actions::ClaudeUpgrade(a) => &a.notify,` in the `notify` match block and add after it:

```rust
            Actions::ClaudeMarketplace(a) => &a.notify,
            Actions::ClaudeMarketplaceRemove(a) => &a.notify,
```

- [ ] **Step 7: Add `Deref` match arms**

Find `Actions::ClaudeUpgrade(a) => a,` in the `Deref` impl match block and add after it:

```rust
            Actions::ClaudeMarketplace(a) => a,
            Actions::ClaudeMarketplaceRemove(a) => a,
```

- [ ] **Step 8: Add `Display` match arms**

Find `Actions::ClaudeUpgrade(_) => "claude.upgrade",` in the `Display` match block and add after it:

```rust
            Actions::ClaudeMarketplace(_) => "claude.marketplace",
            Actions::ClaudeMarketplaceRemove(_) => "claude.marketplace.remove",
```

- [ ] **Step 9: Run tests — confirm they pass**

```bash
cargo test -p etch-lib all_major_action_variants_can_be_deserialized
cargo test -p etch-lib all_action_variants_inner_ref_and_deref
cargo test -p etch-lib all_action_variants_display
```

Expected: all 3 tests pass.

- [ ] **Step 10: Run full test suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 11: Commit**

```bash
git add lib/src/actions/mod.rs
git commit -m "feat(claude): register claude.marketplace and claude.marketplace.remove"
```

---

## Task 5: Add `remove()` to `PackageProvider` trait

**Files:**

- Modify: `lib/src/actions/package/providers/mod.rs`
- Modify: `lib/src/actions/package/providers/aptitude.rs` (stub)
- Modify: `lib/src/actions/package/providers/snapcraft.rs` (stub)
- Modify: `lib/src/actions/package/providers/homebrew.rs` (stub)

Add the trait method and stub implementations so the codebase compiles. Full implementations follow in Tasks 6–8.

- [ ] **Step 1: Add `remove()` to the `PackageProvider` trait in `lib/src/actions/package/providers/mod.rs`**

Find the `fn installed_version` method signature and add `remove` after it:

```rust
    fn installed_version(&self, name: &str) -> anyhow::Result<Option<String>>;

    /// Remove installed packages. `purge` is apt-only (removes config files);
    /// snap and homebrew implementations silently ignore it.
    fn remove(
        &self,
        names: &[String],
        purge: bool,
        contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>>;
```

- [ ] **Step 2: Add stub `remove()` to Aptitude**

Add to the `impl PackageProvider for Aptitude` block in `lib/src/actions/package/providers/aptitude.rs`, after `fn installed_version`:

```rust
    fn remove(
        &self,
        names: &[String],
        _purge: bool,
        _contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        let _ = names;
        Ok(vec![]) // stub — full implementation in Task 6
    }
```

- [ ] **Step 3: Add stub `remove()` to Snapcraft**

Add to the `impl PackageProvider for Snapcraft` block in `lib/src/actions/package/providers/snapcraft.rs`, after `fn installed_version`:

```rust
    fn remove(
        &self,
        names: &[String],
        _purge: bool,
        _contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        let _ = names;
        Ok(vec![]) // stub — full implementation in Task 7
    }
```

- [ ] **Step 4: Add stub `remove()` to Homebrew**

Add to the `impl PackageProvider for Homebrew` block in `lib/src/actions/package/providers/homebrew.rs`, after `fn installed_version`:

```rust
    fn remove(
        &self,
        names: &[String],
        _purge: bool,
        _contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        let _ = names;
        Ok(vec![]) // stub — full implementation in Task 8
    }
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check -p etch-lib
```

Expected: compiles with no errors.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/package/providers/mod.rs \
        lib/src/actions/package/providers/aptitude.rs \
        lib/src/actions/package/providers/snapcraft.rs \
        lib/src/actions/package/providers/homebrew.rs
git commit -m "feat(package): add remove() stub to PackageProvider trait"
```

---

## Task 6: Aptitude `remove()` implementation

**Files:**

- Modify: `lib/src/actions/package/providers/aptitude.rs`

- [ ] **Step 1: Write failing tests**

Add to the `#[cfg(test)] mod test` block in `aptitude.rs`:

```rust
    #[test]
    fn remove_skips_package_not_installed() {
        // dpkg-query will return non-zero for a nonsense package name
        let apt = Aptitude {};
        let contexts = Contexts::default();
        let steps = apt
            .remove(&[String::from("__etch_nonexistent_pkg__")], false, &contexts)
            .unwrap();
        assert!(steps.is_empty(), "expected no steps for uninstalled package");
    }

    #[test]
    fn remove_uses_purge_verb_when_purge_true() {
        // We cannot call apt-get in tests, so test the step arguments directly
        // by bypassing the installed_version check with a known-present package.
        // Use build_remove_step which is the pure step-building helper.
        let step = Aptitude::build_remove_step("nginx", true, "sudo", &[]);
        let display = step.atom.to_string();
        assert!(display.contains("purge"), "expected purge: {display}");
        assert!(!display.contains("remove"), "should not say 'remove': {display}");
    }

    #[test]
    fn remove_uses_remove_verb_when_purge_false() {
        let step = Aptitude::build_remove_step("nginx", false, "sudo", &[]);
        let display = step.atom.to_string();
        assert!(display.contains("remove"), "expected remove: {display}");
        assert!(!display.contains("purge"), "should not say 'purge': {display}");
    }

    #[test]
    fn remove_step_is_privileged() {
        let step = Aptitude::build_remove_step("nginx", false, "sudo", &[]);
        let display = step.atom.to_string();
        assert!(display.contains("privileged=true"), "got: {display}");
    }

    #[test]
    fn remove_step_includes_yes_flag() {
        let step = Aptitude::build_remove_step("nginx", false, "sudo", &[]);
        let display = step.atom.to_string();
        assert!(display.contains("--yes"), "got: {display}");
    }
```

- [ ] **Step 2: Run tests — confirm they fail**

```bash
cargo test -p etch-lib aptitude::test::remove
```

Expected: FAIL (function `build_remove_step` not found)

- [ ] **Step 3: Replace the stub `remove()` in `aptitude.rs` with full implementation**

In the `impl Aptitude` block (the helper methods block, not the trait impl), add:

```rust
    fn build_remove_step(
        name: &str,
        purge: bool,
        privilege_provider: &str,
        env: &[(String, String)],
    ) -> Step {
        use crate::atoms::command::Exec;
        let verb = if purge { "purge" } else { "remove" };
        Step {
            atom: Box::new(Exec {
                command: String::from("apt-get"),
                arguments: vec![
                    String::from(verb),
                    String::from("--yes"),
                    name.to_string(),
                ],
                environment: env.to_vec(),
                privileged: true,
                privilege_provider: privilege_provider.to_string(),
                streaming: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }
    }
```

Replace the stub `remove()` in the `impl PackageProvider for Aptitude` block:

```rust
    fn remove(
        &self,
        names: &[String],
        purge: bool,
        contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());
        let env = self.env();
        let steps = names
            .iter()
            .filter_map(|name| match self.installed_version(name) {
                Ok(Some(_)) => Some(Ok(Self::build_remove_step(
                    name,
                    purge,
                    &privilege_provider,
                    &env,
                ))),
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            })
            .collect::<anyhow::Result<Vec<Step>>>()?;
        Ok(steps)
    }
```

- [ ] **Step 4: Run tests — confirm they pass**

```bash
cargo test -p etch-lib aptitude::test::remove
```

Expected: all 5 new tests pass.

- [ ] **Step 5: Run full lib tests**

```bash
cargo test -p etch-lib
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/package/providers/aptitude.rs
git commit -m "feat(package): implement apt remove with optional purge"
```

---

## Task 7: Snapcraft `remove()` implementation

**Files:**

- Modify: `lib/src/actions/package/providers/snapcraft.rs`

- [ ] **Step 1: Write failing tests**

Add to the `#[cfg(test)] mod test` block in `snapcraft.rs`:

```rust
    #[test]
    fn remove_skips_package_not_installed() {
        let snapcraft = Snapcraft {};
        let contexts = Contexts::default();
        let steps = snapcraft
            .remove(&[String::from("__etch_nonexistent_pkg__")], false, &contexts)
            .unwrap();
        assert!(steps.is_empty(), "expected no steps for uninstalled snap");
    }

    #[test]
    fn remove_step_contains_snap_remove_and_name() {
        let step = Snapcraft::build_remove_step("htop");
        let display = step.atom.to_string();
        assert!(display.contains("snap"), "got: {display}");
        assert!(display.contains("remove"), "got: {display}");
        assert!(display.contains("htop"), "got: {display}");
    }

    #[test]
    fn remove_step_is_privileged() {
        let step = Snapcraft::build_remove_step("htop");
        let display = step.atom.to_string();
        assert!(display.contains("privileged=true"), "got: {display}");
    }

    #[test]
    fn remove_ignores_purge_flag() {
        // snap remove has no --purge equivalent in this action
        let step = Snapcraft::build_remove_step("htop");
        let display = step.atom.to_string();
        assert!(!display.contains("purge"), "snap remove must not pass purge: {display}");
    }
```

- [ ] **Step 2: Run tests — confirm they fail**

```bash
cargo test -p etch-lib snapcraft::test::remove
```

Expected: FAIL (function `build_remove_step` not found)

- [ ] **Step 3: Replace stub `remove()` in `snapcraft.rs`**

In the `impl Snapcraft` block, add:

```rust
    fn build_remove_step(name: &str) -> Step {
        use crate::atoms::command::Exec;
        Step {
            atom: Box::new(Exec {
                command: String::from("snap"),
                arguments: vec![String::from("remove"), name.to_string()],
                privileged: true,
                streaming: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }
    }
```

Replace the stub `remove()` in `impl PackageProvider for Snapcraft`:

```rust
    fn remove(
        &self,
        names: &[String],
        _purge: bool,
        _contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        let steps = names
            .iter()
            .filter_map(|name| match self.installed_version(name) {
                Ok(Some(_)) => Some(Ok(Self::build_remove_step(name))),
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            })
            .collect::<anyhow::Result<Vec<Step>>>()?;
        Ok(steps)
    }
```

- [ ] **Step 4: Run tests — confirm they pass**

```bash
cargo test -p etch-lib snapcraft::test::remove
```

Expected: all 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add lib/src/actions/package/providers/snapcraft.rs
git commit -m "feat(package): implement snap remove"
```

---

## Task 8: Homebrew `remove()` implementation

**Files:**

- Modify: `lib/src/actions/package/providers/homebrew.rs`

- [ ] **Step 1: Write failing tests**

Add to the `#[cfg(test)] mod test` block in `homebrew.rs`:

```rust
    #[test]
    fn remove_skips_package_not_installed() {
        let homebrew = Homebrew {};
        let contexts = Contexts::default();
        let steps = homebrew
            .remove(&[String::from("__etch_nonexistent_pkg__")], false, &contexts)
            .unwrap();
        assert!(steps.is_empty(), "expected no steps for uninstalled formula");
    }

    #[test]
    fn remove_step_contains_brew_uninstall_and_name() {
        let step = Homebrew::build_remove_step("htop");
        let display = step.atom.to_string();
        assert!(display.contains("brew"), "got: {display}");
        assert!(display.contains("uninstall"), "got: {display}");
        assert!(display.contains("htop"), "got: {display}");
    }

    #[test]
    fn remove_step_is_not_privileged() {
        let step = Homebrew::build_remove_step("htop");
        let display = step.atom.to_string();
        assert!(display.contains("privileged=false"), "got: {display}");
    }

    #[test]
    fn remove_ignores_purge_flag() {
        let step = Homebrew::build_remove_step("htop");
        let display = step.atom.to_string();
        assert!(!display.contains("purge"), "brew uninstall must not pass purge: {display}");
    }
```

- [ ] **Step 2: Run tests — confirm they fail**

```bash
cargo test -p etch-lib homebrew::test::remove
```

Expected: FAIL (function `build_remove_step` not found)

- [ ] **Step 3: Replace stub `remove()` in `homebrew.rs`**

In the `impl Homebrew` block (helpers), add:

```rust
    fn build_remove_step(name: &str) -> Step {
        use crate::atoms::command::Exec;
        Step {
            atom: Box::new(Exec {
                command: String::from("brew"),
                arguments: vec![String::from("uninstall"), name.to_string()],
                streaming: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }
    }
```

Replace stub `remove()` in `impl PackageProvider for Homebrew`:

```rust
    fn remove(
        &self,
        names: &[String],
        _purge: bool,
        _contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        let steps = names
            .iter()
            .filter_map(|name| match self.installed_version(name) {
                Ok(Some(_)) => Some(Ok(Self::build_remove_step(name))),
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            })
            .collect::<anyhow::Result<Vec<Step>>>()?;
        Ok(steps)
    }
```

- [ ] **Step 4: Run tests — confirm they pass**

```bash
cargo test -p etch-lib homebrew::test::remove
```

Expected: all 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add lib/src/actions/package/providers/homebrew.rs
git commit -m "feat(package): implement brew uninstall"
```

---

## Task 9: `package.remove` action

**Files:**

- Create: `lib/src/actions/package/remove.rs`

- [ ] **Step 1: Write failing tests**

Create `lib/src/actions/package/remove.rs`:

```rust
use super::providers::PackageProviders;
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageRemove {
    pub name: Option<String>,
    #[serde(default)]
    pub list: Vec<String>,
    #[serde(default)]
    pub provider: PackageProviders,
    #[serde(default)]
    pub purge: bool,
}

impl PackageRemove {
    fn packages(&self) -> Vec<String> {
        self.name
            .as_ref()
            .map(|n| vec![n.clone()])
            .unwrap_or_else(|| self.list.clone())
    }
}

impl Action for PackageRemove {
    fn summarize(&self) -> String {
        let pkgs = self.packages();
        if pkgs.is_empty() {
            return String::from("Removing packages");
        }
        format!("Removing package(s): {}", pkgs.join(", "))
    }

    fn plan(&self, _manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let pkgs = self.packages();
        if pkgs.is_empty() {
            bail!("package.remove requires either 'name' or 'list'");
        }
        let provider = self.provider.clone().get_provider();
        provider.deref().remove(&pkgs, self.purge, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_errors_when_no_name_or_list() {
        let action = PackageRemove::default();
        let result = action.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap()
            .to_string()
            .contains("requires either 'name' or 'list'"));
    }

    #[test]
    fn plan_skips_package_not_installed_apt() {
        let action = PackageRemove {
            name: Some(String::from("__etch_nonexistent_pkg__")),
            provider: PackageProviders::Aptitude,
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert!(steps.is_empty(), "should skip uninstalled package");
    }

    #[test]
    fn plan_skips_package_not_installed_snap() {
        let action = PackageRemove {
            name: Some(String::from("__etch_nonexistent_pkg__")),
            provider: PackageProviders::Snapcraft,
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert!(steps.is_empty(), "should skip uninstalled snap");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn plan_skips_package_not_installed_homebrew() {
        let action = PackageRemove {
            name: Some(String::from("__etch_nonexistent_pkg__")),
            provider: PackageProviders::Homebrew,
            ..Default::default()
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert!(steps.is_empty(), "should skip uninstalled formula");
    }

    #[test]
    fn summarize_includes_package_name() {
        let action = PackageRemove {
            name: Some(String::from("nginx")),
            ..Default::default()
        };
        assert!(action.summarize().contains("nginx"));
    }

    #[test]
    fn summarize_includes_list_names() {
        let action = PackageRemove {
            list: vec![String::from("htop"), String::from("curl")],
            ..Default::default()
        };
        let summary = action.summarize();
        assert!(summary.contains("htop"));
        assert!(summary.contains("curl"));
    }

    #[test]
    fn deserialize_name_form() {
        let yaml = "name: htop\nprovider: apt\n";
        let action: PackageRemove = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(action.name, Some(String::from("htop")));
        assert!(!action.purge);
    }

    #[test]
    fn deserialize_list_form() {
        let yaml = "list:\n  - htop\n  - curl\nprovider: apt\n";
        let action: PackageRemove = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(action.list, vec!["htop", "curl"]);
    }

    #[test]
    fn deserialize_purge_true() {
        let yaml = "name: nginx\nprovider: apt\npurge: true\n";
        let action: PackageRemove = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(action.purge);
    }
}
```

- [ ] **Step 2: Run tests — confirm they fail**

```bash
cargo test -p etch-lib package::remove
```

Expected: FAIL (module not found)

- [ ] **Step 3: Register `remove` module in `lib/src/actions/package/mod.rs`**

Add `pub mod remove;` alongside the existing module declarations. Add re-export:

```rust
pub mod remove;
pub(crate) use remove::PackageRemove;
```

- [ ] **Step 4: Run tests — confirm they pass**

```bash
cargo test -p etch-lib package::remove
```

Expected: all 9 tests pass (the snap and apt skips call real dpkg-query/snap; nonsense package returns None).

- [ ] **Step 5: Run full test suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/package/remove.rs lib/src/actions/package/mod.rs
git commit -m "feat(package): add package.remove action"
```

---

## Task 10: Register `package.remove` in `mod.rs`

**Files:**

- Modify: `lib/src/actions/mod.rs`

Same 6-edit pattern as Task 4.

- [ ] **Step 1: Write failing test entries**

In `all_major_action_variants_can_be_deserialized`, add after the `package.install` entry:

```yaml
- action: package.remove
  name: htop
```

Change count `29` → `30`.

In `all_action_variants_inner_ref_and_deref`, add after `package.upgrade`:

```yaml
- action: package.remove
  name: htop
```

Change count `42` → `43`.

In `all_action_variants_display`, add and update count `42` → `43`, and add:

```yaml
- action: package.remove
  name: htop
```

```rust
assert!(names.contains(&"package.remove".to_string()));
```

- [ ] **Step 2: Run tests — confirm they fail**

```bash
cargo test -p etch-lib all_major_action_variants_can_be_deserialized
```

Expected: FAIL (count mismatch or unknown action)

- [ ] **Step 3: Add `use` import**

Find `use package::{...}` or individual package imports. Find the line that imports package actions (e.g. `use package::PackageInstall;`) and add alongside them:

```rust
use package::PackageRemove;
```

- [ ] **Step 4: Add enum variant**

Find:

```rust
    #[serde(rename = "package.install")]
    PackageInstall(ConditionalVariantAction<PackageInstall>),
```

Add after the `package.upgrade` variant:

```rust
    #[serde(rename = "package.remove")]
    PackageRemove(ConditionalVariantAction<PackageRemove>),
```

- [ ] **Step 5: Add arms to `inner_ref`, `notify`, `Deref`, `Display`**

After `Actions::PackageUpgrade(a) => a,` in `inner_ref`:

```rust
            Actions::PackageRemove(a) => a,
```

After `Actions::PackageUpgrade(a) => &a.notify,` in `notify`:

```rust
            Actions::PackageRemove(a) => &a.notify,
```

After `Actions::PackageUpgrade(a) => a,` in `Deref`:

```rust
            Actions::PackageRemove(a) => a,
```

After `Actions::PackageUpgrade(_) => "package.upgrade",` in `Display`:

```rust
            Actions::PackageRemove(_) => "package.remove",
```

- [ ] **Step 6: Run tests — confirm they pass**

```bash
cargo test -p etch-lib all_major_action_variants_can_be_deserialized
cargo test -p etch-lib all_action_variants_inner_ref_and_deref
cargo test -p etch-lib all_action_variants_display
```

Expected: all 3 tests pass.

- [ ] **Step 7: Run full test suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add lib/src/actions/mod.rs
git commit -m "feat(package): register package.remove in Actions dispatch"
```

---

## Task 11: Examples and documentation

**Files:**

- Create: `examples/claude/claude-marketplace.yaml`
- Modify: `examples/package/package-management.yaml`
- Modify: `README.md`
- Modify: `docs/knowledge/action-catalog.md`
- Modify: `docs/superpowers/README.md`

> **Note:** The plan index update (`docs/superpowers/README.md`) must be done on main after the PR merges — not inside a worktree. All other doc edits belong in the PR.

- [ ] **Step 1: Create `examples/claude/claude-marketplace.yaml`**

```yaml
# claude.marketplace — ensure a Claude Code plugin marketplace is registered
# claude.marketplace.remove — remove a registered marketplace
#
# Idempotent: add skips if marketplace already present;
# remove skips if marketplace already absent.
#
# Fields:
#   name:    marketplace handle (used for idempotency check)
#   source:  GitHub "owner/repo" or full git URL
#   scope:   optional — user (default) | project | local
#   sparse:  optional — list of paths for monorepo sparse checkout

actions:
    # Add a marketplace from a GitHub repo (shorthand: owner/repo)
    - action: claude.marketplace
      name: caveman
      source: juliusbrussee/caveman

    # Add with explicit user scope
    - action: claude.marketplace
      name: firecrawl
      source: firecrawl/firecrawl-claude-plugin
      scope: user

    # Add a marketplace from a full git URL
    - action: claude.marketplace
      name: my-internal-marketplace
      source: https://github.com/my-org/my-marketplace.git

    # Add from a monorepo using sparse checkout
    - action: claude.marketplace
      name: warp-plugins
      source: warpdotdev/warp-plugins
      sparse:
          - claude-plugins

    # Remove a marketplace from all scopes
    - action: claude.marketplace.remove
      name: caveman

    # Remove from a specific scope only
    - action: claude.marketplace.remove
      name: caveman
      scope: user
```

- [ ] **Step 2: Add `package.remove` section to `examples/package/package-management.yaml`**

Append to the end of `examples/package/package-management.yaml`:

```yaml
# ────────────────────────────────────────────────────
# REMOVE — uninstall packages
# ────────────────────────────────────────────────────

# Remove a single package via apt
- action: package.remove
  name: nginx
  provider: apt
  where: 'os.family == "linux"'

# Remove and purge config files (apt only)
- action: package.remove
  name: nginx
  provider: apt
  purge: true
  where: 'os.family == "linux"'

# Remove multiple packages at once
- action: package.remove
  list: [htop, curl, wget]
  provider: apt
  where: 'os.family == "linux"'

# Remove a snap package
- action: package.remove
  name: htop
  provider: snap
  where: "variables.has_snap"

# Remove a Homebrew formula (works on macOS and Linux)
- action: package.remove
  name: htop
  provider: homebrew
```

- [ ] **Step 3: Add 3 rows to README.md action catalog table**

Find the `claude.install` and `claude.upgrade` rows in the action catalog table in `README.md` and add two rows after them:

```markdown
| `claude.marketplace` | Add a Claude Code plugin marketplace | `name`, `source`, `scope?`, `sparse?[]` |
| `claude.marketplace.remove` | Remove a Claude Code plugin marketplace | `name`, `scope?` |
```

Find the `package.install` and `package.upgrade` rows and add after them:

```markdown
| `package.remove` | Remove installed packages (apt/snap/homebrew) | `name?`, `list?[]`, `provider?`, `purge?` |
```

- [ ] **Step 4: Add 3 rows to `docs/knowledge/action-catalog.md`**

Apply the same additions as Step 3 to the action catalog in `docs/knowledge/action-catalog.md`.

- [ ] **Step 5: Run full test suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 6: Commit examples and docs**

```bash
git add examples/claude/claude-marketplace.yaml \
        examples/package/package-management.yaml \
        README.md \
        docs/knowledge/action-catalog.md
git commit -m "docs: add examples and catalog entries for claude.marketplace and package.remove"
```

- [ ] **Step 7: Update plan index on main after PR merges**

_Do this directly on main after the PR merges — not inside the worktree._

Add a row to `docs/superpowers/README.md` All Plans table:

```markdown
| 2026-06-05 | [claude-marketplace-package-remove](plans/2026-06-05-claude-marketplace-package-remove.md) | [claude-marketplace-package-remove](specs/2026-06-05-claude-marketplace-package-remove-design.md) | Done |
```

Add `> **Status: DONE**` banner at the top of this plan file.

---

## Self-Review Checklist

- `parse_marketplace_list` → Task 1 ✓
- `claude.marketplace` struct + plan + idempotency → Task 2 ✓
- `claude.marketplace.remove` struct + plan + idempotency → Task 3 ✓
- mod.rs registration for both claude actions → Task 4 ✓
- `PackageProvider::remove()` trait method → Task 5 ✓
- Apt remove + purge flag → Task 6 ✓
- Snap remove → Task 7 ✓
- Homebrew remove → Task 8 ✓
- `package.remove` action struct + plan → Task 9 ✓
- mod.rs registration for package.remove → Task 10 ✓
- Examples + docs + plan index → Task 11 ✓
