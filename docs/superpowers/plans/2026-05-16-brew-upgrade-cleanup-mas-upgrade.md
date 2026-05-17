# brew.upgrade, brew.cleanup, mas.upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `brew.upgrade`, `brew.cleanup`, and `mas.upgrade` actions following the exact `brew.bundle` / `mas.install` patterns already in the codebase.

**Architecture:** Three new files in existing `lib/src/actions/brew/` and `lib/src/actions/mas/` modules. Each wraps a single `Exec` atom. Module exports updated in `brew/mod.rs` and `mas/mod.rs`. All three registered in the `Actions` enum in `lib/src/actions/mod.rs`. The `it_can_be_deserialized` tests are deferred to Task 2 (same compile-order pattern as previous actions).

**Tech Stack:** Rust, serde, schemars, existing `Exec` atom, `anyhow`

---

## Files

| File                              | Change                                                            |
| --------------------------------- | ----------------------------------------------------------------- |
| `lib/src/actions/brew/upgrade.rs` | **Create** — `BrewUpgrade { greedy: bool }`                       |
| `lib/src/actions/brew/cleanup.rs` | **Create** — `BrewCleanup { prune: Option<u32> }`                 |
| `lib/src/actions/mas/upgrade.rs`  | **Create** — `MasUpgrade { id: Option<u64> }`                     |
| `lib/src/actions/brew/mod.rs`     | **Modify** — re-export all three brew actions                     |
| `lib/src/actions/mas/mod.rs`      | **Modify** — re-export MasUpgrade                                 |
| `lib/src/actions/mod.rs`          | **Modify** — imports, enum variants, match arms, round-trip tests |

---

### Task 1: Create action files + update module exports + non-deser tests

**Files:**

- Create: `lib/src/actions/brew/upgrade.rs`
- Create: `lib/src/actions/brew/cleanup.rs`
- Create: `lib/src/actions/mas/upgrade.rs`
- Modify: `lib/src/actions/brew/mod.rs`
- Modify: `lib/src/actions/mas/mod.rs`

- [ ] **Step 1: Create `lib/src/actions/brew/upgrade.rs`**

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrewUpgrade {
    #[serde(default = "get_false")]
    pub greedy: bool,
}

fn get_false() -> bool {
    false
}

impl Action for BrewUpgrade {
    fn summarize(&self) -> String {
        if self.greedy {
            String::from("Upgrading Homebrew packages (greedy)")
        } else {
            String::from("Upgrading Homebrew packages")
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let mut args = vec![String::from("upgrade")];
        if self.greedy {
            args.push(String::from("--greedy"));
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("brew"),
                arguments: args,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

#[cfg(test)]
mod tests {
    // NOTE: it_can_be_deserialized requires Actions::BrewUpgrade from the enum.
    // Added in Task 2 after the enum variant is registered.

    #[test]
    fn plan_returns_exec_step() {
        use super::BrewUpgrade;
        use crate::actions::Action;
        let action = BrewUpgrade { greedy: false };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("brew"), "expected 'brew' in: {display}");
        assert!(display.contains("upgrade"), "expected 'upgrade' in: {display}");
    }

    #[test]
    fn plan_includes_greedy_flag() {
        use super::BrewUpgrade;
        use crate::actions::Action;
        let action = BrewUpgrade { greedy: true };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("--greedy"), "expected '--greedy' in: {display}");
    }
}
```

- [ ] **Step 2: Create `lib/src/actions/brew/cleanup.rs`**

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrewCleanup {
    pub prune: Option<u32>,
}

impl Action for BrewCleanup {
    fn summarize(&self) -> String {
        String::from("Cleaning up Homebrew cache")
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let mut args = vec![String::from("cleanup")];
        if let Some(days) = self.prune {
            args.push(format!("--prune={days}"));
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("brew"),
                arguments: args,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

#[cfg(test)]
mod tests {
    // NOTE: it_can_be_deserialized added in Task 2.

    #[test]
    fn plan_returns_exec_step() {
        use super::BrewCleanup;
        use crate::actions::Action;
        let action = BrewCleanup { prune: None };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("brew"), "expected 'brew' in: {display}");
        assert!(display.contains("cleanup"), "expected 'cleanup' in: {display}");
    }

    #[test]
    fn plan_includes_prune_flag() {
        use super::BrewCleanup;
        use crate::actions::Action;
        let action = BrewCleanup { prune: Some(30) };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("--prune=30"), "expected '--prune=30' in: {display}");
    }
}
```

- [ ] **Step 3: Create `lib/src/actions/mas/upgrade.rs`**

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasUpgrade {
    pub id: Option<u64>,
}

impl Action for MasUpgrade {
    fn summarize(&self) -> String {
        match self.id {
            Some(id) => format!("Upgrading App Store app {id}"),
            None => String::from("Upgrading all App Store apps"),
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let mut args = vec![String::from("upgrade")];
        if let Some(id) = self.id {
            args.push(id.to_string());
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("mas"),
                arguments: args,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

#[cfg(test)]
mod tests {
    // NOTE: it_can_be_deserialized added in Task 2.

    #[test]
    fn plan_returns_exec_step() {
        use super::MasUpgrade;
        use crate::actions::Action;
        let action = MasUpgrade { id: None };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("mas"), "expected 'mas' in: {display}");
        assert!(display.contains("upgrade"), "expected 'upgrade' in: {display}");
    }

    #[test]
    fn plan_includes_id_when_set() {
        use super::MasUpgrade;
        use crate::actions::Action;
        let action = MasUpgrade { id: Some(414209656) };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("414209656"), "expected app ID in: {display}");
    }
}
```

- [ ] **Step 4: Update `lib/src/actions/brew/mod.rs`**

Replace the current content with:

```rust
mod bundle;
mod cleanup;
mod upgrade;
pub use bundle::BrewBundle;
pub use cleanup::BrewCleanup;
pub use upgrade::BrewUpgrade;
```

- [ ] **Step 5: Update `lib/src/actions/mas/mod.rs`**

Replace the current content with:

```rust
mod install;
mod upgrade;
pub use install::MasInstall;
pub use upgrade::MasUpgrade;
```

- [ ] **Step 6: Run the 6 non-deser tests**

```bash
cargo test -p etch-lib actions::brew::upgrade::tests actions::brew::cleanup::tests actions::mas::upgrade::tests 2>&1 | tail -10
```

Expected: 6 tests pass (2 per action).

- [ ] **Step 7: Run full suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add lib/src/actions/brew/upgrade.rs \
        lib/src/actions/brew/cleanup.rs \
        lib/src/actions/mas/upgrade.rs \
        lib/src/actions/brew/mod.rs \
        lib/src/actions/mas/mod.rs
git commit -m "feat: add BrewUpgrade, BrewCleanup, MasUpgrade actions (enum registration in next commit)

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Register all three in the Actions enum + add deser tests + update round-trip tests

**Files:**

- Modify: `lib/src/actions/mod.rs`
- Modify: `lib/src/actions/brew/upgrade.rs`
- Modify: `lib/src/actions/brew/cleanup.rs`
- Modify: `lib/src/actions/mas/upgrade.rs`

- [ ] **Step 1: Update imports in `lib/src/actions/mod.rs`**

Change `use brew::BrewBundle;` to:

```rust
use brew::{BrewBundle, BrewCleanup, BrewUpgrade};
```

Change `use crate::actions::mas::MasInstall;` to:

```rust
use crate::actions::mas::{MasInstall, MasUpgrade};
```

- [ ] **Step 2: Add enum variants (alphabetically within brew/mas groups)**

In the `Actions` enum, after `BrewBundle`, add:

```rust
    #[serde(rename = "brew.cleanup")]
    BrewCleanup(ConditionalVariantAction<BrewCleanup>),

    #[serde(rename = "brew.upgrade")]
    BrewUpgrade(ConditionalVariantAction<BrewUpgrade>),
```

After `MasInstall`, add:

```rust
    #[serde(rename = "mas.upgrade")]
    MasUpgrade(ConditionalVariantAction<MasUpgrade>),
```

- [ ] **Step 3: Add match arms to `inner_ref()`, `Deref::deref()`, `Display::fmt()`**

Add to `inner_ref()` and `Deref::deref()`:

```rust
            Actions::BrewCleanup(a) => a,
            Actions::BrewUpgrade(a) => a,
            Actions::MasUpgrade(a) => a,
```

Add to `Display::fmt()`:

```rust
            Actions::BrewCleanup(_) => "brew.cleanup",
            Actions::BrewUpgrade(_) => "brew.upgrade",
            Actions::MasUpgrade(_) => "mas.upgrade",
```

- [ ] **Step 4: Update `all_major_action_variants_can_be_deserialized`**

Add three YAML entries after the existing `brew.bundle` entry:

```yaml
- action: brew.upgrade
- action: brew.cleanup
- action: mas.upgrade
```

Change count: `assert_eq!(17, ...)` → `assert_eq!(20, manifest.actions.len())`

- [ ] **Step 5: Update `actions_display_names`**

Add the same three YAML entries and add to `expected_names`:

```rust
            "brew.upgrade",
            "brew.cleanup",
            "mas.upgrade",
```

- [ ] **Step 6: Update `all_action_variants_inner_ref_and_deref`**

Add the same three YAML entries and change count: `assert_eq!(21, ...)` → `assert_eq!(24, manifest.actions.len())`

- [ ] **Step 7: Add `it_can_be_deserialized` to each action file**

Add to `lib/src/actions/brew/upgrade.rs` tests, before `plan_returns_exec_step`:

```rust
    #[test]
    fn it_can_be_deserialized() {
        use crate::actions::Actions;
        let yaml = r#"
- action: brew.upgrade
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::BrewUpgrade(action)) => {
                assert!(!action.action.greedy);
            }
            _ => panic!("BrewUpgrade didn't deserialize to the correct type"),
        }
    }
```

Add to `lib/src/actions/brew/cleanup.rs` tests:

```rust
    #[test]
    fn it_can_be_deserialized() {
        use crate::actions::Actions;
        let yaml = r#"
- action: brew.cleanup
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::BrewCleanup(action)) => {
                assert!(action.action.prune.is_none());
            }
            _ => panic!("BrewCleanup didn't deserialize to the correct type"),
        }
    }
```

Add to `lib/src/actions/mas/upgrade.rs` tests:

```rust
    #[test]
    fn it_can_be_deserialized() {
        use crate::actions::Actions;
        let yaml = r#"
- action: mas.upgrade
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MasUpgrade(action)) => {
                assert!(action.action.id.is_none());
            }
            _ => panic!("MasUpgrade didn't deserialize to the correct type"),
        }
    }
```

- [ ] **Step 8: Run full suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass including the 9 new tests (3 deser + 2 per action × 3 = 9 total).

- [ ] **Step 9: Commit**

```bash
git add lib/src/actions/mod.rs \
        lib/src/actions/brew/upgrade.rs \
        lib/src/actions/brew/cleanup.rs \
        lib/src/actions/mas/upgrade.rs
git commit -m "feat: register BrewUpgrade, BrewCleanup, MasUpgrade in Actions enum

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 3: Docs update (post-merge on main)

**Do this directly on main after the PR merges — not inside the worktree.**

- [ ] **Step 1: Mark spec Done in README**

Change `brew-upgrade-cleanup-mas-upgrade` row from `Pending` to `Done`.

- [ ] **Step 2: Update CLAUDE.md action catalog**

Add three rows to the Action Catalog table:

```markdown
| `brew.upgrade` | Upgrade installed Homebrew formulae and casks (macOS) | `greedy` (bool, default false — also upgrades auto-update casks) |
| `brew.cleanup` | Remove old Homebrew versions and cache (macOS) | `prune` (u32 days, optional — omit to use brew's default of 120 days) |
| `mas.upgrade` | Upgrade Mac App Store apps (macOS only) | `id` (u64, optional — omit to upgrade all; requires `mas` CLI) |
```

- [ ] **Step 3: Commit and push on main**

```bash
git add docs/superpowers/README.md CLAUDE.md
git commit -m "docs: mark brew-upgrade-cleanup-mas-upgrade Done; update action catalog"
git push origin main
```
