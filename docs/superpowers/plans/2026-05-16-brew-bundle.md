# brew.bundle Action Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `brew.bundle` action that runs `brew bundle install --file=<path>` with optional `--no-upgrade` and `--cleanup` flags.

**Architecture:** New `lib/src/actions/brew/` module following the `git/` pattern. `BrewBundle` struct implements `Action` by returning a single `Exec` atom. Registered in the `Actions` enum in `lib/src/actions/mod.rs`. The `it_can_be_deserialized` test requires the enum variant and is deferred to Task 2 (same compile-order pattern used for previous actions).

**Tech Stack:** Rust, serde, schemars, existing `Exec` atom, `anyhow`

---

## Files

| File                             | Change                                                                                                                |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `lib/src/actions/brew/mod.rs`    | **Create** — re-exports `BrewBundle`                                                                                  |
| `lib/src/actions/brew/bundle.rs` | **Create** — `BrewBundle` struct + `Action` impl + 4 tests                                                            |
| `lib/src/actions/mod.rs`         | **Modify** — add `mod brew;`, `BrewBundle` import, enum variant, `inner_ref`/`Deref`/`Display` arms, round-trip tests |

---

### Task 1: Create the brew module and BrewBundle action (3 non-deser tests)

**Files:**

- Create: `lib/src/actions/brew/mod.rs`
- Create: `lib/src/actions/brew/bundle.rs`
- Modify: `lib/src/actions/mod.rs` (module declaration only — not the enum variant yet)

- [ ] **Step 1: Create `lib/src/actions/brew/mod.rs`**

```rust
mod bundle;
pub use bundle::BrewBundle;
```

- [ ] **Step 2: Create `lib/src/actions/brew/bundle.rs`**

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrewBundle {
    pub file: String,

    #[serde(default = "get_false")]
    pub no_upgrade: bool,

    #[serde(default = "get_false")]
    pub cleanup: bool,
}

fn get_false() -> bool {
    false
}

impl Action for BrewBundle {
    fn summarize(&self) -> String {
        format!("Installing Homebrew bundle from {}", self.file)
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let mut args = vec![
            String::from("bundle"),
            String::from("install"),
            format!("--file={}", self.file),
        ];
        if self.no_upgrade {
            args.push(String::from("--no-upgrade"));
        }
        if self.cleanup {
            args.push(String::from("--cleanup"));
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
    // NOTE: it_can_be_deserialized requires Actions::BrewBundle from the enum.
    // Added in Task 2 after the enum variant is registered.

    #[test]
    fn plan_returns_exec_step() {
        use super::BrewBundle;
        use crate::actions::Action;
        let action = BrewBundle {
            file: String::from("/tmp/Brewfile"),
            no_upgrade: false,
            cleanup: false,
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("brew"), "expected 'brew' in: {display}");
    }

    #[test]
    fn plan_includes_no_upgrade_flag() {
        use super::BrewBundle;
        use crate::actions::Action;
        let action = BrewBundle {
            file: String::from("/tmp/Brewfile"),
            no_upgrade: true,
            cleanup: false,
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        // Exec Display format: "CommandExec with: privileged=false: brew bundle install --file=... --no-upgrade"
        let display = steps[0].atom.to_string();
        assert!(display.contains("--no-upgrade"), "expected '--no-upgrade' in: {display}");
    }

    #[test]
    fn plan_includes_cleanup_flag() {
        use super::BrewBundle;
        use crate::actions::Action;
        let action = BrewBundle {
            file: String::from("/tmp/Brewfile"),
            no_upgrade: false,
            cleanup: true,
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("--cleanup"), "expected '--cleanup' in: {display}");
    }
}
```

- [ ] **Step 3: Add `mod brew;` to `lib/src/actions/mod.rs`**

Find the module declarations block at the top (lines 1-10). Add `mod brew;` in alphabetical order (between `mod binary;` and `mod command;`):

```rust
mod binary;
mod brew;
mod command;
mod directory;
mod file;
mod git;
mod group;
mod macos;
mod package;
mod plugin;
mod user;
```

- [ ] **Step 4: Run the 3 non-deser tests**

```bash
cargo test -p etch-lib actions::brew::bundle::tests 2>&1 | tail -10
```

Expected: 3 tests pass (`plan_returns_exec_step`, `plan_includes_no_upgrade_flag`, `plan_includes_cleanup_flag`).

- [ ] **Step 5: Run full suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/brew/mod.rs \
        lib/src/actions/brew/bundle.rs \
        lib/src/actions/mod.rs
git commit -m "feat: add BrewBundle action (module + impl; enum registration in next commit)

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Register BrewBundle in the Actions enum + add deser test + round-trip tests

**Files:**

- Modify: `lib/src/actions/mod.rs`
- Modify: `lib/src/actions/brew/bundle.rs`

- [ ] **Step 1: Add import to `lib/src/actions/mod.rs`**

After `use binary::BinaryGitHub;`, add:

```rust
use brew::BrewBundle;
```

- [ ] **Step 2: Add enum variant to `lib/src/actions/mod.rs`**

In the `Actions` enum, add after `Actions::BinaryGitHub` (alphabetical by prefix):

```rust
    #[serde(rename = "brew.bundle")]
    BrewBundle(ConditionalVariantAction<BrewBundle>),
```

- [ ] **Step 3: Add `Actions::BrewBundle(a) => a` to `inner_ref()` match**

```rust
            Actions::BrewBundle(a) => a,
```

- [ ] **Step 4: Add `Actions::BrewBundle(a) => a` to `Deref::deref()` match**

```rust
            Actions::BrewBundle(a) => a,
```

- [ ] **Step 5: Add `Actions::BrewBundle(_) => "brew.bundle"` to `Display::fmt()` match**

```rust
            Actions::BrewBundle(_) => "brew.bundle",
```

- [ ] **Step 6: Update `all_major_action_variants_can_be_deserialized` in `lib/src/actions/mod.rs`**

Find the test YAML (around line 334). Add after the `git.clone` entry:

```yaml
- action: brew.bundle
  file: /tmp/Brewfile
```

Change the assertion count from 15 to 16:

```rust
        assert_eq!(16, manifest.actions.len());
```

- [ ] **Step 7: Update `actions_display_names` test in `lib/src/actions/mod.rs`**

Add `brew.bundle` entry and `"brew.bundle"` to expected names array (after `"binary.github"` alphabetically or in insertion order — follow the existing order in the test):

```yaml
- action: brew.bundle
  file: /tmp/Brewfile
```

Add `"brew.bundle"` to `expected_names`.

- [ ] **Step 8: Add `it_can_be_deserialized` test to `lib/src/actions/brew/bundle.rs`**

Add inside the existing `#[cfg(test)] mod tests { ... }` block, before `plan_returns_exec_step`:

```rust
    #[test]
    fn it_can_be_deserialized() {
        use crate::actions::Actions;
        let yaml = r#"
- action: brew.bundle
  file: /tmp/Brewfile
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::BrewBundle(action)) => {
                assert_eq!("/tmp/Brewfile", action.action.file);
                assert!(!action.action.no_upgrade);
                assert!(!action.action.cleanup);
            }
            _ => panic!("BrewBundle didn't deserialize to the correct type"),
        }
    }
```

- [ ] **Step 9: Run full suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass including the new `it_can_be_deserialized` test.

- [ ] **Step 10: Commit**

```bash
git add lib/src/actions/mod.rs lib/src/actions/brew/bundle.rs
git commit -m "feat: register BrewBundle in Actions enum; add deserialization test

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 3: Update docs

**Files:**

- Modify: `docs/superpowers/README.md` (post-merge on main — NOT in worktree)
- Modify: `CLAUDE.md` action catalog

Per the worktree-docs-conflict pattern: do docs status updates directly on main after the PR merges, not inside the worktree.

- [ ] **Step 1: After PR merges, update README status to Done**

Change `brew-bundle` row from `Pending` to `Done`.

- [ ] **Step 2: Update CLAUDE.md action catalog**

Add `brew.bundle` row to the Action Catalog table in `CLAUDE.md`:

```markdown
| `brew.bundle` | Install packages from a Brewfile | `file` (path), `no_upgrade` (bool), `cleanup` (bool) |
```

- [ ] **Step 3: Commit and push on main**

```bash
git add docs/superpowers/README.md CLAUDE.md
git commit -m "docs: mark brew-bundle Done; add to action catalog"
git push origin main
```
