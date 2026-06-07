# ruby.chruby Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `ruby.chruby` action that installs chruby via Homebrew and optionally sets `~/.ruby-version`, and extend `ruby.install` so `version_manager: chruby` writes `~/.ruby-version` after installing the ruby.

**Architecture:** New `lib/src/actions/ruby/chruby.rs` contains `RubyChruby` struct and `impl Action`. Registered in `lib/src/actions/mod.rs` like all other actions. `lib/src/actions/ruby/install.rs` gains a new branch in its `plan()` for `version_manager: Chruby` that appends a `SetContents` atom writing `~/.ruby-version`.

**Tech Stack:** Rust, clap (not needed — lib-only action), serde, schemars, shellexpand (already in lib/Cargo.toml), `SetContents` atom (already in `lib/src/atoms/file/contents.rs`)

---

### Task 1: Scaffold `ruby.chruby` and register it

**Files:**

- Create: `lib/src/actions/ruby/chruby.rs`
- Modify: `lib/src/actions/ruby/mod.rs`
- Modify: `lib/src/actions/mod.rs`

- [ ] **Step 1: Create `lib/src/actions/ruby/chruby.rs` with stub plan**

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RubyChruby {
    /// Ruby version to set as default in ~/.ruby-version.
    /// Verbatim string written to the file (e.g. "ruby-3.3.0").
    /// If omitted, ~/.ruby-version is not written.
    pub default_version: Option<String>,
}

impl Action for RubyChruby {
    fn summarize(&self) -> String {
        match &self.default_version {
            Some(v) => format!("Installing chruby and setting default ruby to {v}"),
            None => String::from("Installing chruby"),
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }
}
```

- [ ] **Step 2: Export `chruby` from `lib/src/actions/ruby/mod.rs`**

Replace the entire file with:

```rust
mod chruby;
mod install;
pub(crate) use chruby::RubyChruby;
pub(crate) use install::RubyInstall;
```

- [ ] **Step 3: Add `use ruby::RubyChruby;` import in `lib/src/actions/mod.rs`**

Find:

```rust
use ruby::RubyInstall;
```

Add immediately after:

```rust
use ruby::RubyChruby;
```

- [ ] **Step 4: Add enum variant in `lib/src/actions/mod.rs`**

Find:

```rust
    #[serde(rename = "ruby.install")]
    RubyInstall(ConditionalVariantAction<RubyInstall>),
```

Add immediately after:

```rust
    #[serde(rename = "ruby.chruby")]
    RubyChruby(ConditionalVariantAction<RubyChruby>),
```

- [ ] **Step 5: Add `inner_ref()` match arm in `lib/src/actions/mod.rs`**

Find (in the `inner_ref()` impl):

```rust
            Actions::RubyInstall(a) => a,
            Actions::GemInstall(a) => a,
```

Replace with:

```rust
            Actions::RubyInstall(a) => a,
            Actions::RubyChruby(a) => a,
            Actions::GemInstall(a) => a,
```

- [ ] **Step 6: Add `notify()` match arm in `lib/src/actions/mod.rs`**

Find (in the `notify()` impl):

```rust
            Actions::RubyInstall(a) => &a.notify,
            Actions::GemInstall(a) => &a.notify,
```

Replace with:

```rust
            Actions::RubyInstall(a) => &a.notify,
            Actions::RubyChruby(a) => &a.notify,
            Actions::GemInstall(a) => &a.notify,
```

- [ ] **Step 7: Add `Deref` match arm in `lib/src/actions/mod.rs`**

Find (in the `Deref` impl):

```rust
            Actions::RubyInstall(a) => a,
            Actions::GemInstall(a) => a,
```

Replace with:

```rust
            Actions::RubyInstall(a) => a,
            Actions::RubyChruby(a) => a,
            Actions::GemInstall(a) => a,
```

- [ ] **Step 8: Add `Display` match arm in `lib/src/actions/mod.rs`**

Find (in the `Display` impl):

```rust
            Actions::RubyInstall(_) => "ruby.install",
            Actions::GemInstall(_) => "gem.install",
```

Replace with:

```rust
            Actions::RubyInstall(_) => "ruby.install",
            Actions::RubyChruby(_) => "ruby.chruby",
            Actions::GemInstall(_) => "gem.install",
```

- [ ] **Step 9: Update `all_major_action_variants_can_be_deserialized` YAML list**

Find in `mod.rs` tests (around line 685):

```
  - action: package.remove
    name: htop
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(31, manifest.actions.len());
```

Replace with:

```
  - action: package.remove
    name: htop
  - action: ruby.chruby
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(32, manifest.actions.len());
```

- [ ] **Step 10: Update `all_action_variants_inner_ref_and_deref` YAML list**

Find in `mod.rs` tests (near line 1050):

```
  - action: ruby.install
    version: "3.3.0"
```

Add immediately after:

```
  - action: ruby.chruby
```

Then find:

```
        assert_eq!(44, manifest.actions.len());
```

in the `all_action_variants_inner_ref_and_deref` test and change to:

```
        assert_eq!(45, manifest.actions.len());
```

- [ ] **Step 11: Update `all_action_variants_display` YAML list and assertion**

Find in `mod.rs` tests (near line 1342):

```
  - action: ruby.install
    version: "3.3.0"
```

Add immediately after:

```
  - action: ruby.chruby
```

Then find:

```
        assert_eq!(44, manifest.actions.len());
```

in the `all_action_variants_display` test and change to:

```
        assert_eq!(45, manifest.actions.len());
```

Then add after `assert!(names.contains(&"ruby.install".to_string()));`:

```rust
        assert!(names.contains(&"ruby.chruby".to_string()));
```

- [ ] **Step 12: Compile check**

```bash
cargo check -p etch-lib 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 13: Run tests**

```bash
cargo test -p etch-lib 2>&1 | tail -15
```

Expected: all tests pass.

- [ ] **Step 14: Commit**

```bash
git add lib/src/actions/ruby/chruby.rs lib/src/actions/ruby/mod.rs lib/src/actions/mod.rs
git commit -m "feat(ruby): scaffold ruby.chruby action and register it"
```

---

### Task 2: TDD — implement `ruby.chruby` plan() and deserialization tests

**Files:**

- Modify: `lib/src/actions/ruby/chruby.rs`

- [ ] **Step 1: Write failing plan tests**

In `lib/src/actions/ruby/chruby.rs`, add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    #[test]
    fn it_can_be_deserialized_without_default_version() {
        let yaml = r#"
actions:
  - action: ruby.chruby
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let action = match &manifest.actions[0] {
            Actions::RubyChruby(a) => &a.action,
            _ => panic!("wrong variant"),
        };
        assert_eq!(None, action.default_version);
    }

    #[test]
    fn it_can_be_deserialized_with_default_version() {
        let yaml = r#"
actions:
  - action: ruby.chruby
    default_version: "ruby-3.3.0"
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let action = match &manifest.actions[0] {
            Actions::RubyChruby(a) => &a.action,
            _ => panic!("wrong variant"),
        };
        assert_eq!(
            Some(String::from("ruby-3.3.0")),
            action.default_version
        );
    }

    #[test]
    fn plan_without_default_version_emits_one_step() {
        let action = RubyChruby { default_version: None };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_with_default_version_emits_two_steps() {
        let action = RubyChruby {
            default_version: Some(String::from("ruby-3.3.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(2, steps.len());
    }
}
```

- [ ] **Step 2: Run tests, confirm they fail**

```bash
cargo test -p etch-lib ruby_chruby 2>&1 | tail -15
```

Expected: `plan_without_default_version_emits_one_step` and `plan_with_default_version_emits_two_steps` fail (plan returns 0 steps from the stub).

- [ ] **Step 3: Implement `plan()` in `chruby.rs`**

Replace the stub `plan()` with:

```rust
    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;
        use crate::atoms::file::SetContents;
        use std::path::PathBuf;

        let mut steps = vec![Step {
            atom: Box::new(Exec {
                command: String::from("brew"),
                arguments: vec![String::from("install"), String::from("chruby")],
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }];

        if let Some(version) = &self.default_version {
            let path = PathBuf::from(shellexpand::tilde("~/.ruby-version").into_owned());
            steps.push(Step {
                atom: Box::new(SetContents {
                    path,
                    contents: format!("{version}\n").into_bytes(),
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        Ok(steps)
    }
```

Add `use std::path::PathBuf;` to the top-level imports in `chruby.rs` (outside the function):

Full updated imports at top of file:

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
```

And update the plan function to remove the `use` statements inside (they're now at the top level):

```rust
    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;
        use crate::atoms::file::SetContents;

        let mut steps = vec![Step {
            atom: Box::new(Exec {
                command: String::from("brew"),
                arguments: vec![String::from("install"), String::from("chruby")],
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }];

        if let Some(version) = &self.default_version {
            let path = PathBuf::from(shellexpand::tilde("~/.ruby-version").into_owned());
            steps.push(Step {
                atom: Box::new(SetContents {
                    path,
                    contents: format!("{version}\n").into_bytes(),
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        Ok(steps)
    }
```

- [ ] **Step 4: Run tests, confirm they pass**

```bash
cargo test -p etch-lib ruby_chruby 2>&1 | tail -15
```

Expected: 4 tests pass (`it_can_be_deserialized_without_default_version`, `it_can_be_deserialized_with_default_version`, `plan_without_default_version_emits_one_step`, `plan_with_default_version_emits_two_steps`).

- [ ] **Step 5: Run full suite**

```bash
cargo test -p etch-lib 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/ruby/chruby.rs
git commit -m "feat(ruby): implement ruby.chruby plan with brew install and ~/.ruby-version write"
```

---

### Task 3: Extend `ruby.install` `version_manager: chruby` to write `~/.ruby-version`

**Files:**

- Modify: `lib/src/actions/ruby/install.rs`

- [ ] **Step 1: Update existing chruby test to expect 2 steps**

In `lib/src/actions/ruby/install.rs`, find and rename `plan_with_chruby_emits_one_step` to `plan_with_chruby_emits_two_steps` and update the assertion:

```rust
    #[test]
    fn plan_with_chruby_emits_two_steps() {
        let tmp = tempfile::tempdir().unwrap();
        let action = RubyInstall {
            version: String::from("3.3.0"),
            implementation: None,
            rubies_dir: Some(tmp.path().to_string_lossy().to_string()),
            version_manager: Some(VersionManager::Chruby),
            compile_flags: vec![],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(
            2,
            steps.len(),
            "chruby should emit ruby-install step + ~/.ruby-version SetContents step"
        );
    }
```

- [ ] **Step 2: Add jruby + chruby test**

Add this test after `plan_with_chruby_emits_two_steps`:

```rust
    #[test]
    fn plan_with_chruby_and_jruby_impl_emits_two_steps() {
        let tmp = tempfile::tempdir().unwrap();
        let action = RubyInstall {
            version: String::from("9.4.0.0"),
            implementation: Some(String::from("jruby")),
            rubies_dir: Some(tmp.path().to_string_lossy().to_string()),
            version_manager: Some(VersionManager::Chruby),
            compile_flags: vec![],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(2, steps.len());
        // Second step writes ~/.ruby-version; verify it mentions ruby-version path
        assert!(
            format!("{}", steps[1].atom).contains("ruby-version"),
            "second step should be the ~/.ruby-version write"
        );
    }
```

- [ ] **Step 3: Run tests, confirm they fail**

```bash
cargo test -p etch-lib plan_with_chruby 2>&1 | tail -15
```

Expected: `plan_with_chruby_emits_two_steps` fails (currently returns 1 step), `plan_with_chruby_and_jruby_impl_emits_two_steps` fails.

- [ ] **Step 4: Add `SetContents` import to `install.rs`**

In `lib/src/actions/ruby/install.rs`, add to the top-level imports:

```rust
use crate::atoms::file::SetContents;
```

The file already imports `use std::path::PathBuf;` and uses `shellexpand` — no new dependencies needed.

- [ ] **Step 5: Add the chruby branch in `plan()` in `install.rs`**

Find in `plan()` (after the rbenv block, before `Ok(steps)`):

```rust
        Ok(steps)
    }
}
```

Insert the chruby branch before `Ok(steps)`:

```rust
        if let Some(VersionManager::Chruby) = &self.version_manager {
            let version_file =
                PathBuf::from(shellexpand::tilde("~/.ruby-version").into_owned());
            steps.push(Step {
                atom: Box::new(SetContents {
                    path: version_file,
                    contents: format!("{}-{}\n", self.impl_name(), self.version).into_bytes(),
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        Ok(steps)
```

The full `plan()` end section now looks like:

```rust
        if let Some(VersionManager::Rbenv) = &self.version_manager {
            steps.push(Step {
                atom: Box::new(Exec {
                    command: String::from("rbenv"),
                    arguments: vec![String::from("global"), self.version.clone()],
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            });
            steps.push(Step {
                atom: Box::new(Exec {
                    command: String::from("rbenv"),
                    arguments: vec![String::from("rehash")],
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        if let Some(VersionManager::Chruby) = &self.version_manager {
            let version_file =
                PathBuf::from(shellexpand::tilde("~/.ruby-version").into_owned());
            steps.push(Step {
                atom: Box::new(SetContents {
                    path: version_file,
                    contents: format!("{}-{}\n", self.impl_name(), self.version).into_bytes(),
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        Ok(steps)
    }
```

- [ ] **Step 6: Run tests, confirm they pass**

```bash
cargo test -p etch-lib plan_with_chruby 2>&1 | tail -15
```

Expected: both chruby tests pass.

- [ ] **Step 7: Confirm rbenv regression test still passes**

```bash
cargo test -p etch-lib plan_with_rbenv 2>&1 | tail -10
```

Expected: rbenv test passes (3 steps unchanged).

- [ ] **Step 8: Run full suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 9: Commit**

```bash
git add lib/src/actions/ruby/install.rs
git commit -m "feat(ruby): write ~/.ruby-version when version_manager is chruby"
```

---

### Task 4: Add example, update catalogs, open PR

**Files:**

- Create: `examples/ruby/ruby-chruby.yaml`
- Modify: `docs/knowledge/action-catalog.md`
- Modify: `README.md`

- [ ] **Step 1: Create `examples/ruby/ruby-chruby.yaml`**

```yaml
# ruby.chruby — install chruby and optionally set a default Ruby version
#
# chruby is a lightweight Ruby version manager. This action installs it via
# Homebrew. Shell RC sourcing (source chruby.sh in .zshrc) is left to
# file.link or file.copy — see examples/file/.
#
# Pair with ruby.install to install a ruby and set it as the default.

# Minimal: install chruby only, do not touch ~/.ruby-version
- action: ruby.chruby
  where: 'os.name == "macos"'

---
# Install chruby and set a default ruby version in ~/.ruby-version.
# The value is written verbatim — use the chruby directory-name format.
- action: ruby.chruby
  default_version: "ruby-3.3.0"
  where: 'os.name == "macos"'

---
# Full workflow: install chruby, then install ruby and set as default.
# ruby.install with version_manager: chruby writes ~/.ruby-version automatically.
- action: ruby.chruby
  where: 'os.name == "macos"'

- action: ruby.install
  version: "3.3.0"
  version_manager: chruby
  where: 'os.name == "macos"'
```

- [ ] **Step 2: Add `ruby.chruby` row to `docs/knowledge/action-catalog.md`**

Find the `ruby.install` row and add `ruby.chruby` immediately after it. The row format follows the existing table. Add:

```
| `ruby.chruby` | Install chruby via Homebrew; optionally write a default ruby version to `~/.ruby-version`. `default_version` is a verbatim string (e.g. `ruby-3.3.0`). macOS only (brew). Shell RC sourcing left to the manifest. |
```

- [ ] **Step 3: Add `ruby.chruby` row to `README.md` action catalog table**

Find the `ruby.install` row in the README action catalog table and add `ruby.chruby` immediately after it. Match the existing table format.

- [ ] **Step 4: Commit examples and docs**

```bash
git add examples/ruby/ruby-chruby.yaml docs/knowledge/action-catalog.md README.md
git commit -m "docs: add ruby.chruby example and catalog entries"
```

- [ ] **Step 5: Push branch and open PR**

```bash
git push -u origin HEAD
gh pr create --repo brujack/etch-cli \
  --title "feat(ruby): add ruby.chruby action and extend version_manager: chruby" \
  --body "$(cat <<'EOF'
## Summary
- New \`ruby.chruby\` action: installs chruby via \`brew install chruby\` and optionally writes \`~/.ruby-version\`
- Extends \`ruby.install\` with \`version_manager: chruby\` to write \`~/.ruby-version\` after installing the ruby
- Shell RC sourcing (\`source chruby.sh\`) is left to the manifest via \`file.link\`/\`file.copy\`

## Test Plan
- [ ] \`make test\` passes
- [ ] \`ruby.chruby\` (no default_version) → 1 step (brew install)
- [ ] \`ruby.chruby\` (with default_version) → 2 steps (brew + SetContents)
- [ ] \`ruby.install\` + \`version_manager: chruby\` → 2 steps (ruby-install + SetContents writing \`ruby-3.3.0\n\`)
- [ ] \`ruby.install\` + \`version_manager: rbenv\` → still 3 steps (no regression)
- [ ] deserialization of both new YAML forms works
EOF
)"
```

- [ ] **Step 6: Monitor CI**

```bash
gh pr checks --repo brujack/etch-cli --watch
```

Fix any failures before merging.

- [ ] **Step 7: After merge — update plan index** _(do this on main after the PR merges, not inside the worktree)_

In `docs/superpowers/README.md`, update the `ruby-chruby` row status to `Done` and add the plan link.

Add `> **Status: DONE**` banner at the top of this plan file.
