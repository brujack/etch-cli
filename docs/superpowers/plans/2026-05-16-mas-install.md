# mas.install Action Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `mas.install` action that runs `mas install <id>` to install Mac App Store apps declaratively.

**Architecture:** New `lib/src/actions/mas/` module following the `brew/` pattern. `MasInstall { name: String, id: u64 }` implements `Action` by returning a single `Exec` atom running `mas install <id>`. Registered in the `Actions` enum as `mas.install`. The `it_can_be_deserialized` test requires the enum variant and is deferred to Task 2 (same compile-order pattern used for brew.bundle and file.chmod).

**Tech Stack:** Rust, serde, schemars, existing `Exec` atom, `anyhow`

---

## Files

| File                             | Change                                                                                                           |
| -------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `lib/src/actions/mas/mod.rs`     | **Create** — re-exports `MasInstall`                                                                             |
| `lib/src/actions/mas/install.rs` | **Create** — `MasInstall` struct + `Action` impl + 3+ tests                                                      |
| `lib/src/actions/mod.rs`         | **Modify** — `mod mas;`, `MasInstall` import, enum variant, `inner_ref`/`Deref`/`Display` arms, round-trip tests |

---

### Task 1: Create the mas module and MasInstall action (non-deser tests)

**Files:**

- Create: `lib/src/actions/mas/mod.rs`
- Create: `lib/src/actions/mas/install.rs`
- Modify: `lib/src/actions/mod.rs` (module declaration only — not the enum variant yet)

- [ ] **Step 1: Create `lib/src/actions/mas/mod.rs`**

```rust
mod install;
pub use install::MasInstall;
```

- [ ] **Step 2: Create `lib/src/actions/mas/install.rs`**

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasInstall {
    pub name: String,
    pub id: u64,
}

impl Action for MasInstall {
    fn summarize(&self) -> String {
        format!("Installing {} from the Mac App Store", self.name)
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("mas"),
                arguments: vec![String::from("install"), self.id.to_string()],
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

#[cfg(test)]
mod tests {
    // NOTE: it_can_be_deserialized requires Actions::MasInstall from the enum.
    // Added in Task 2 after the enum variant is registered.

    #[test]
    fn plan_returns_exec_step() {
        use super::MasInstall;
        use crate::actions::Action;
        let action = MasInstall {
            name: String::from("Better Rename 9"),
            id: 414209656,
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        // Exec Display: "CommandExec with: privileged=false: mas install 414209656"
        let display = steps[0].atom.to_string();
        assert!(display.contains("mas"), "expected 'mas' in: {display}");
        assert!(
            display.contains("414209656"),
            "expected app ID in: {display}"
        );
    }

    #[test]
    fn plan_includes_correct_id() {
        use super::MasInstall;
        use crate::actions::Action;
        let action = MasInstall {
            name: String::from("Flycut"),
            id: 442160987,
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("442160987"),
            "expected correct ID in: {display}"
        );
    }

    #[test]
    fn summarize_includes_name() {
        use super::MasInstall;
        use crate::actions::Action;
        let action = MasInstall {
            name: String::from("Better Rename 9"),
            id: 414209656,
        };
        let summary = action.summarize();
        assert!(
            summary.contains("Better Rename 9"),
            "expected app name in: {summary}"
        );
    }
}
```

- [ ] **Step 3: Add `mod mas;` to `lib/src/actions/mod.rs`**

Find the module declarations block at the top. Add `mod mas;` in alphabetical order (between `mod macos;` and `mod package;`):

```rust
mod macos;
mod mas;
mod package;
```

- [ ] **Step 4: Run the 3 non-deser tests**

```bash
cargo test -p etch-lib actions::mas::install::tests 2>&1 | tail -10
```

Expected: 3 tests pass (`plan_returns_exec_step`, `plan_includes_correct_id`, `summarize_includes_name`).

- [ ] **Step 5: Run full suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/mas/mod.rs \
        lib/src/actions/mas/install.rs \
        lib/src/actions/mod.rs
git commit -m "feat: add MasInstall action (module + impl; enum registration in next commit)

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Register MasInstall in the Actions enum + add deser test + round-trip tests

**Files:**

- Modify: `lib/src/actions/mod.rs`
- Modify: `lib/src/actions/mas/install.rs`

- [ ] **Step 1: Add import to `lib/src/actions/mod.rs`**

After `use macos::MacOSDefault;` (or similar), add:

```rust
use mas::MasInstall;
```

- [ ] **Step 2: Add enum variant**

In the `Actions` enum, add in alphabetical position (between `MacOSDefault` and `PackageInstall`):

```rust
    #[serde(rename = "mas.install")]
    MasInstall(ConditionalVariantAction<MasInstall>),
```

- [ ] **Step 3: Add `Actions::MasInstall(a) => a` to `inner_ref()` match**

- [ ] **Step 4: Add `Actions::MasInstall(a) => a` to `Deref::deref()` match**

- [ ] **Step 5: Add `Actions::MasInstall(_) => "mas.install"` to `Display::fmt()` match**

- [ ] **Step 6: Update `all_major_action_variants_can_be_deserialized`**

Add `mas.install` to the YAML (after `macos.default` entry):

```yaml
- action: mas.install
  name: "Better Rename 9"
  id: 414209656
```

Change the count assertion: `assert_eq!(16, ...)` → `assert_eq!(17, manifest.actions.len())`

- [ ] **Step 7: Update `actions_display_names`**

Add the same `mas.install` YAML entry and add `"mas.install"` to the `expected_names` array.

- [ ] **Step 8: Update `all_action_variants_inner_ref_and_deref`**

Add the `mas.install` YAML entry and change the count: `assert_eq!(20, ...)` → `assert_eq!(21, manifest.actions.len())`

- [ ] **Step 9: Add `it_can_be_deserialized` to `lib/src/actions/mas/install.rs`**

Add inside `#[cfg(test)] mod tests { ... }`, before `plan_returns_exec_step`:

```rust
    #[test]
    fn it_can_be_deserialized() {
        use crate::actions::Actions;
        let yaml = r#"
- action: mas.install
  name: "Better Rename 9"
  id: 414209656
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MasInstall(action)) => {
                assert_eq!("Better Rename 9", action.action.name);
                assert_eq!(414209656u64, action.action.id);
            }
            _ => panic!("MasInstall didn't deserialize to the correct type"),
        }
    }
```

- [ ] **Step 10: Run full suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass including `it_can_be_deserialized`.

- [ ] **Step 11: Commit**

```bash
git add lib/src/actions/mod.rs lib/src/actions/mas/install.rs
git commit -m "feat: register MasInstall in Actions enum; add deserialization test

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 3: Docs update (post-merge on main)

Per the worktree-docs-conflict pattern: do docs updates directly on main after the PR merges, not inside the worktree.

- [ ] **Step 1: After PR merges, mark spec Done in README**

Change the `mas-install` row from `Pending` to `Done`.

- [ ] **Step 2: Update CLAUDE.md action catalog**

Add `mas.install` row:

```markdown
| `mas.install` | Install a Mac App Store app | `name` (string, for readability), `id` (u64, App Store numeric ID) |
```

- [ ] **Step 3: Commit and push on main**

```bash
git add docs/superpowers/README.md CLAUDE.md
git commit -m "docs: mark mas-install Done; add to action catalog"
git push origin main
```
