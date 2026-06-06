> **Status: DONE**

# claude.plugin.update Action — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `claude.plugin.update` action that runs `claude plugins update <name>` for one or more named plugins.

**Architecture:** One new file `lib/src/actions/claude/plugin_update.rs` holds the struct and tests; `claude/mod.rs` re-exports it; six edits to `lib/src/actions/mod.rs` register the enum variant and match arms. No idempotency pre-check — `claude plugins update` is safe to re-run.

**Tech Stack:** Rust, serde/serde_yaml_ng, schemars, anyhow, serial_test, tempfile

---

## File Map

| File                                                      | Change                                                  |
| --------------------------------------------------------- | ------------------------------------------------------- |
| `lib/src/actions/claude/plugin_update.rs`                 | Create — struct + Action impl + tests                   |
| `lib/src/actions/claude/mod.rs`                           | Modify — add `pub mod plugin_update` + re-export        |
| `lib/src/actions/mod.rs`                                  | Modify — 6 registration edits + 3 dispatch test updates |
| `examples/claude.plugin.update/claude.plugin.update.yaml` | Create — example manifest                               |
| `docs/superpowers/README.md`                              | Modify — add plan row (post-merge on main only)         |

---

## Task 1: Create `plugin_update.rs` with failing tests

**Files:**

- Create: `lib/src/actions/claude/plugin_update.rs`

- [ ] **Step 1: Write the failing tests**

Create `lib/src/actions/claude/plugin_update.rs` with this content (struct stub + full test suite):

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudePluginUpdate {
    pub name: Option<String>,
    #[serde(default)]
    pub list: Vec<String>,
}

impl Action for ClaudePluginUpdate {
    fn summarize(&self) -> String {
        todo!()
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized_name() {
        let yaml = "- action: claude.plugin.update\n  name: superpowers\n";
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::ClaudePluginUpdate(a)) => {
                assert_eq!(a.action.name.as_deref(), Some("superpowers"));
            }
            _ => panic!("expected ClaudePluginUpdate"),
        }
    }

    #[test]
    fn it_can_be_deserialized_list() {
        let yaml = concat!(
            "- action: claude.plugin.update\n",
            "  list:\n",
            "    - superpowers\n",
            "    - context7\n",
        );
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::ClaudePluginUpdate(a)) => {
                assert_eq!(a.action.list, vec!["superpowers", "context7"]);
            }
            _ => panic!("expected ClaudePluginUpdate"),
        }
    }

    #[test]
    fn summarize_includes_plugin_name() {
        let action = ClaudePluginUpdate {
            name: Some(String::from("superpowers")),
            list: vec![],
        };
        let s = action.summarize();
        assert!(s.contains("superpowers"), "got: {s}");
    }

    #[test]
    fn summarize_includes_all_list_plugins() {
        let action = ClaudePluginUpdate {
            name: None,
            list: vec![String::from("superpowers"), String::from("context7")],
        };
        let s = action.summarize();
        assert!(s.contains("superpowers"), "got: {s}");
        assert!(s.contains("context7"), "got: {s}");
    }

    #[test]
    fn summarize_with_no_plugins_returns_generic() {
        let s = ClaudePluginUpdate::default().summarize();
        assert!(!s.is_empty(), "expected non-empty summarize");
    }

    #[test]
    fn plan_errors_without_name_or_list() {
        let result =
            ClaudePluginUpdate::default().plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(msg.contains("name") || msg.contains("list"), "got: {msg}");
    }

    #[test]
    fn plan_returns_exec_for_name() {
        let action = ClaudePluginUpdate {
            name: Some(String::from("superpowers")),
            list: vec![],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("claude"), "got: {display}");
        assert!(display.contains("update"), "got: {display}");
        assert!(display.contains("superpowers"), "got: {display}");
    }

    #[test]
    fn plan_returns_exec_for_list() {
        let action = ClaudePluginUpdate {
            name: None,
            list: vec![String::from("superpowers"), String::from("context7")],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(2, steps.len());
        let d0 = steps[0].atom.to_string();
        let d1 = steps[1].atom.to_string();
        assert!(d0.contains("superpowers"), "got: {d0}");
        assert!(d1.contains("context7"), "got: {d1}");
        assert!(d0.contains("update"), "got: {d0}");
        assert!(d1.contains("update"), "got: {d1}");
    }
}
```

> **Note:** `it_can_be_deserialized_name` and `it_can_be_deserialized_list` reference `Actions::ClaudePluginUpdate` which does not exist yet — the file will not compile until Task 3 registers the variant. That is expected. Run `cargo test -p etch-lib` after Task 3.

- [ ] **Step 2: Verify file exists and is syntactically valid (ignoring missing variant)**

```bash
cargo check -p etch-lib 2>&1 | grep -v "ClaudePluginUpdate" | head -20
```

Expected: errors only about `ClaudePluginUpdate` not found — no other syntax errors.

---

## Task 2: Register in `claude/mod.rs`

**Files:**

- Modify: `lib/src/actions/claude/mod.rs`

- [ ] **Step 1: Add `pub mod plugin_update` and re-export**

In `lib/src/actions/claude/mod.rs`, add after the `pub mod upgrade;` line:

```rust
pub mod plugin_update;
```

And add `ClaudePluginUpdate` to the `pub(crate) use` block. Change:

```rust
pub(crate) use install::ClaudeInstall;
pub(crate) use marketplace::ClaudeMarketplace;
pub(crate) use marketplace_remove::ClaudeMarketplaceRemove;
pub(crate) use upgrade::ClaudeUpgrade;
```

to:

```rust
pub(crate) use install::ClaudeInstall;
pub(crate) use marketplace::ClaudeMarketplace;
pub(crate) use marketplace_remove::ClaudeMarketplaceRemove;
pub(crate) use plugin_update::ClaudePluginUpdate;
pub(crate) use upgrade::ClaudeUpgrade;
```

- [ ] **Step 2: Quick compile check**

```bash
cargo check -p etch-lib 2>&1 | head -20
```

Expected: errors about `ClaudePluginUpdate` not in `Actions` enum — not syntax errors.

---

## Task 3: Register variant and match arms in `lib/src/actions/mod.rs`

**Files:**

- Modify: `lib/src/actions/mod.rs`

This file has six locations that need edits plus three dispatch tests to update.

- [ ] **Step 1: Add `ClaudePluginUpdate` to the `use claude::` import**

Find:

```rust
use claude::{ClaudeInstall, ClaudeMarketplace, ClaudeMarketplaceRemove, ClaudeUpgrade};
```

Replace with:

```rust
use claude::{
    ClaudeInstall, ClaudeMarketplace, ClaudeMarketplaceRemove, ClaudePluginUpdate, ClaudeUpgrade,
};
```

- [ ] **Step 2: Add enum variant**

Find (the ClaudeUpgrade variant block):

```rust
    #[serde(rename = "claude.upgrade")]
    ClaudeUpgrade(ConditionalVariantAction<ClaudeUpgrade>),
```

Replace with:

```rust
    #[serde(rename = "claude.upgrade")]
    ClaudeUpgrade(ConditionalVariantAction<ClaudeUpgrade>),
    #[serde(rename = "claude.plugin.update")]
    ClaudePluginUpdate(ConditionalVariantAction<ClaudePluginUpdate>),
```

- [ ] **Step 3: Add arm to `inner_ref()` impl**

Find:

```rust
            Actions::ClaudeUpgrade(a) => a,
```

There are two such lines (one in `inner_ref`, one in `Deref`). Add after the one in `inner_ref` (the block around line 290):

```rust
            Actions::ClaudeUpgrade(a) => a,
            Actions::ClaudePluginUpdate(a) => a,
```

Use `replace_all: false` and include surrounding context to target the right block.

- [ ] **Step 4: Add arm to `notify` accessor**

Find:

```rust
            Actions::ClaudeUpgrade(a) => &a.notify,
```

Replace with:

```rust
            Actions::ClaudeUpgrade(a) => &a.notify,
            Actions::ClaudePluginUpdate(a) => &a.notify,
```

- [ ] **Step 5: Add arm to `Deref` impl**

Find the second `Actions::ClaudeUpgrade(a) => a,` (in the `Deref` impl, around line 396).

Replace with:

```rust
            Actions::ClaudeUpgrade(a) => a,
            Actions::ClaudePluginUpdate(a) => a,
```

- [ ] **Step 6: Add arm to `Display` impl**

Find:

```rust
            Actions::ClaudeUpgrade(_) => "claude.upgrade",
```

Replace with:

```rust
            Actions::ClaudeUpgrade(_) => "claude.upgrade",
            Actions::ClaudePluginUpdate(_) => "claude.plugin.update",
```

- [ ] **Step 7: Update the three dispatch tests**

Locate the three test functions with:

```bash
grep -n "fn all_major_action_variants_can_be_deserialized\|fn all_action_variants_inner_ref_and_deref\|fn all_action_variants_display" lib/src/actions/mod.rs
```

**Test 1 — `all_major_action_variants_can_be_deserialized`** (count currently 30):
Find `assert_eq!(30, manifest.actions.len())` and change to `assert_eq!(31, manifest.actions.len())`.
Find the YAML block containing `- action: claude.install` and add a new entry:

```yaml
- action: claude.plugin.update
  name: superpowers
```

**Test 2 — `all_action_variants_inner_ref_and_deref`** (count currently 43):
Find `assert_eq!(43, manifest.actions.len())` in this test and change to `assert_eq!(44, manifest.actions.len())`.
Find the YAML block containing `- action: claude.marketplace` and add:

```yaml
- action: claude.plugin.update
  name: superpowers
```

**Test 3 — `all_action_variants_display`** (count currently 43):
Find `assert_eq!(43, manifest.actions.len())` in this test and change to `assert_eq!(44, manifest.actions.len())`.
Find the YAML block containing `- action: claude.marketplace.remove` and add:

```yaml
- action: claude.plugin.update
  name: superpowers
```

Add the names assertion after the last `names.contains` line:

```rust
        assert!(names.contains(&"claude.plugin.update".to_string()));
```

- [ ] **Step 8: Run tests**

```bash
cargo test -p etch-lib 2>&1 | tail -30
```

Expected: all tests pass, zero failures.

- [ ] **Step 9: Commit**

```bash
git add lib/src/actions/claude/plugin_update.rs \
        lib/src/actions/claude/mod.rs \
        lib/src/actions/mod.rs
git commit -m "feat: add claude.plugin.update action

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 4: Implement `ClaudePluginUpdate::summarize` and `plan`

**Files:**

- Modify: `lib/src/actions/claude/plugin_update.rs`

- [ ] **Step 1: Replace `todo!()` stubs with real implementation**

Replace the entire `impl Action for ClaudePluginUpdate` block with:

```rust
impl ClaudePluginUpdate {
    fn plugin_names(&self) -> Vec<String> {
        if !self.list.is_empty() {
            self.list.clone()
        } else if let Some(name) = &self.name {
            vec![name.clone()]
        } else {
            vec![]
        }
    }
}

impl Action for ClaudePluginUpdate {
    fn summarize(&self) -> String {
        let names = self.plugin_names();
        if names.is_empty() {
            return String::from("Updating Claude plugins");
        }
        format!("Updating Claude plugin(s): {}", names.join(", "))
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let names = self.plugin_names();
        if names.is_empty() {
            bail!("claude.plugin.update requires either 'name' or 'list'");
        }

        let steps = names
            .into_iter()
            .map(|name| Step {
                atom: Box::new(Exec {
                    command: String::from("claude"),
                    arguments: vec![String::from("plugins"), String::from("update"), name],
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
```

- [ ] **Step 2: Run the full test suite**

```bash
make test
```

Expected: `test result: ok` for all crates, lint clean.

- [ ] **Step 3: Commit**

```bash
git add lib/src/actions/claude/plugin_update.rs
git commit -m "feat(claude.plugin.update): implement summarize and plan

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 5: Add example and open PR

**Files:**

- Create: `examples/claude.plugin.update/claude.plugin.update.yaml`

- [ ] **Step 1: Create example file**

```bash
mkdir -p examples/claude.plugin.update
```

Create `examples/claude.plugin.update/claude.plugin.update.yaml`:

```yaml
actions:
    # Update a single plugin by name
    - action: claude.plugin.update
      name: superpowers

    # Update a single plugin with explicit marketplace
    - action: claude.plugin.update
      name: superpowers@claude-plugins-official

    # Update multiple plugins at once
    - action: claude.plugin.update
      list:
          - superpowers
          - context7
          - context-mode
          - caveman

    # Update only on macOS
    - action: claude.plugin.update
      name: superpowers
      where: 'os.name == "macos"'
```

- [ ] **Step 2: Run make test one final time**

```bash
make test
```

Expected: all tests pass, lint clean.

- [ ] **Step 3: Commit and push**

```bash
git add examples/claude.plugin.update/
git commit -m "docs(examples): add claude.plugin.update example manifest

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"

git checkout -b feat/claude-plugin-update
git push -u origin feat/claude-plugin-update
gh pr create --repo brujack/etch-cli \
  --title "feat: add claude.plugin.update action" \
  --body "$(cat <<'EOF'
## Summary

- Adds `claude.plugin.update` action for updating already-installed Claude Code plugins
- Fields: `name` (single plugin) and `list` (multiple plugins), mirroring `claude.install`
- Always emits `claude plugins update <name>` steps — no idempotency pre-check
- Supports `name@marketplace` passthrough

## Test plan

- [ ] All unit tests in `plugin_update.rs` pass
- [ ] Three dispatch tests in `mod.rs` updated and passing
- [ ] `make test` clean
- [ ] CI passes

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 4: Monitor CI**

```bash
gh pr checks --repo brujack/etch-cli --watch
```

Wait for all checks to go green. If any fail, read the output:

```bash
gh run view --repo brujack/etch-cli --log-failed
```

Fix, commit to the branch, and push. CI re-runs automatically.

---

## Post-Merge (do on `main` after PR auto-merges — NOT in worktree)

- [ ] **Update plan index**

In `docs/superpowers/README.md`, add a row to the All Plans table:

```markdown
| 2026-06-06 | [claude-plugin-update](plans/2026-06-06-claude-plugin-update.md) | [claude-plugin-update](specs/2026-06-06-claude-plugin-update-design.md) | Done |
```

Add `> **Status: DONE**` banner at the top of this plan file.

Commit directly on `main`:

```bash
git commit -m "docs: mark claude-plugin-update Done in plan index"
```
