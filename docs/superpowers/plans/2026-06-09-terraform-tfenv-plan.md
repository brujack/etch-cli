# terraform.tfenv Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `terraform.tfenv` action that installs tfenv via git clone and optionally installs and activates a specific Terraform version.

**Architecture:** Single Rust file following the `ruby.chruby` pattern — optional `version` field, step count varies based on presence of version. New `terraform` action group (first action in this module). Registration follows the standard 6-edit pattern in `lib/src/actions/mod.rs`.

**Tech Stack:** Rust, serde/serde_yaml_ng, schemars, `crate::atoms::command::Exec`, `crate::steps::initializers::{FileExists, FlowControl}`, `shellexpand`

---

### Task 1: Create `terraform/tfenv.rs` (TDD)

**Files:**

- Create: `lib/src/actions/terraform/tfenv.rs`
- Create: `lib/src/actions/terraform/mod.rs`

- [ ] **Step 1: Write the full file with impl and tests**

Create `lib/src/actions/terraform/tfenv.rs`:

```rust
use crate::actions::Action;
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::initializers::{FileExists, FlowControl};
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "terraform.tfenv")]
pub struct TerraformTfenv {
    /// Terraform version to install and set as default (e.g. "1.9.0").
    /// If omitted, only tfenv itself is installed.
    pub version: Option<String>,
}

impl Action for TerraformTfenv {
    fn summarize(&self) -> String {
        match &self.version {
            Some(v) => format!("Installing tfenv and Terraform {v}"),
            None => String::from("Installing tfenv"),
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        let tfenv_dir = shellexpand::tilde("~/.tfenv").into_owned();
        let tfenv_bin = shellexpand::tilde("~/.tfenv/bin/tfenv").into_owned();

        let mut steps = vec![Step {
            atom: Box::new(Exec {
                command: String::from("git"),
                arguments: vec![
                    String::from("clone"),
                    String::from("https://github.com/tfutils/tfenv.git"),
                    tfenv_dir.clone(),
                ],
                ..Default::default()
            }),
            initializers: vec![FlowControl::SkipIf(Box::new(FileExists(PathBuf::from(
                &tfenv_dir,
            ))))],
            finalizers: vec![],
        }];

        if let Some(version) = &self.version {
            let versions_dir =
                shellexpand::tilde(&format!("~/.tfenv/versions/{version}")).into_owned();

            steps.push(Step {
                atom: Box::new(Exec {
                    command: tfenv_bin.clone(),
                    arguments: vec![String::from("install"), version.clone()],
                    ..Default::default()
                }),
                initializers: vec![FlowControl::SkipIf(Box::new(FileExists(PathBuf::from(
                    versions_dir,
                ))))],
                finalizers: vec![],
            });

            steps.push(Step {
                atom: Box::new(Exec {
                    command: tfenv_bin,
                    arguments: vec![String::from("use"), version.clone()],
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    #[test]
    fn it_can_be_deserialized_without_version() {
        let yaml = r#"
- action: terraform.tfenv
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::TerraformTfenv(a)) => assert_eq!(None, a.action.version),
            _ => panic!("TerraformTfenv didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_version() {
        let yaml = r#"
- action: terraform.tfenv
  version: "1.9.0"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::TerraformTfenv(a)) => {
                assert_eq!(Some(String::from("1.9.0")), a.action.version)
            }
            _ => panic!("TerraformTfenv didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn summarize_without_version() {
        let action = TerraformTfenv { version: None };
        assert_eq!("Installing tfenv", action.summarize());
    }

    #[test]
    fn summarize_with_version() {
        let action = TerraformTfenv {
            version: Some(String::from("1.9.0")),
        };
        assert_eq!("Installing tfenv and Terraform 1.9.0", action.summarize());
    }

    #[test]
    fn plan_without_version_emits_one_step() {
        let action = TerraformTfenv { version: None };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_with_version_emits_three_steps() {
        let action = TerraformTfenv {
            version: Some(String::from("1.9.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(3, steps.len());
    }

    #[test]
    fn plan_step1_clones_tfenv() {
        let action = TerraformTfenv { version: None };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[0].atom.to_string();
        assert!(display.contains("git"), "expected 'git' in: {display}");
        assert!(display.contains("clone"), "expected 'clone' in: {display}");
        assert!(display.contains("tfenv"), "expected 'tfenv' in: {display}");
    }

    #[test]
    fn plan_step1_has_one_initializer() {
        let action = TerraformTfenv { version: None };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(
            1,
            steps[0].initializers.len(),
            "expected 1 SkipIf initializer for idempotency"
        );
    }

    #[test]
    fn plan_step2_runs_tfenv_install() {
        let action = TerraformTfenv {
            version: Some(String::from("1.9.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[1].atom.to_string();
        assert!(display.contains("tfenv"), "expected 'tfenv' in: {display}");
        assert!(
            display.contains("install"),
            "expected 'install' in: {display}"
        );
        assert!(display.contains("1.9.0"), "expected '1.9.0' in: {display}");
    }

    #[test]
    fn plan_step2_has_one_initializer() {
        let action = TerraformTfenv {
            version: Some(String::from("1.9.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(
            1,
            steps[1].initializers.len(),
            "expected 1 SkipIf initializer on install step"
        );
    }

    #[test]
    fn plan_step3_runs_tfenv_use() {
        let action = TerraformTfenv {
            version: Some(String::from("1.9.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[2].atom.to_string();
        assert!(display.contains("tfenv"), "expected 'tfenv' in: {display}");
        assert!(display.contains("use"), "expected 'use' in: {display}");
        assert!(display.contains("1.9.0"), "expected '1.9.0' in: {display}");
    }

    #[test]
    fn plan_step3_has_no_initializers() {
        let action = TerraformTfenv {
            version: Some(String::from("1.9.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(
            0,
            steps[2].initializers.len(),
            "expected no initializers — tfenv use is idempotent"
        );
    }
}
```

- [ ] **Step 2: Create `lib/src/actions/terraform/mod.rs`**

```rust
mod tfenv;
pub use tfenv::TerraformTfenv;
```

- [ ] **Step 3: Attempt compile (expect error — not registered yet)**

```bash
cargo build -p etch-lib 2>&1 | head -20
```

Expected: compile error — `Actions::TerraformTfenv` variant not found (because registration hasn't happened yet). This is correct RED state.

---

### Task 2: Register `TerraformTfenv` in `actions/mod.rs`

**Files:**

- Modify: `lib/src/actions/mod.rs`

Read the file before editing.

- [ ] **Step 1: Add `mod terraform;` declaration**

In `lib/src/actions/mod.rs`, find:

```rust
mod systemd;
mod user;
```

Add `mod terraform;` between them (alphabetical order):

```rust
mod systemd;
mod terraform;
mod user;
```

- [ ] **Step 2: Add `use terraform::TerraformTfenv;` import**

In the `use` block (near `use ruby::RubyChruby;`), add after the systemd import. Find this line:

```rust
use crate::actions::systemd::SystemdService;
```

Add after it:

```rust
use terraform::TerraformTfenv;
```

- [ ] **Step 3: Add enum variant**

Find:

```rust
    #[serde(rename = "pyenv.virtualenv")]
    PyenvVirtualenv(ConditionalVariantAction<PyenvVirtualenv>),

    #[serde(rename = "zsh.oh-my-zsh")]
    ZshOhMyZsh(ConditionalVariantAction<ZshOhMyZsh>),
```

Add `TerraformTfenv` between them:

```rust
    #[serde(rename = "pyenv.virtualenv")]
    PyenvVirtualenv(ConditionalVariantAction<PyenvVirtualenv>),

    #[serde(rename = "terraform.tfenv")]
    TerraformTfenv(ConditionalVariantAction<TerraformTfenv>),

    #[serde(rename = "zsh.oh-my-zsh")]
    ZshOhMyZsh(ConditionalVariantAction<ZshOhMyZsh>),
```

- [ ] **Step 4: Add `inner_ref()` match arm**

Find:

```rust
            Actions::PyenvVirtualenv(a) => a,
            Actions::ZshOhMyZsh(a) => a,
```

Add between them:

```rust
            Actions::PyenvVirtualenv(a) => a,
            Actions::TerraformTfenv(a) => a,
            Actions::ZshOhMyZsh(a) => a,
```

- [ ] **Step 5: Add `notify()` match arm**

Find:

```rust
            Actions::PyenvVirtualenv(a) => &a.notify,
            Actions::ZshOhMyZsh(a) => &a.notify,
```

Add between them:

```rust
            Actions::PyenvVirtualenv(a) => &a.notify,
            Actions::TerraformTfenv(a) => &a.notify,
            Actions::ZshOhMyZsh(a) => &a.notify,
```

- [ ] **Step 6: Add `Deref` match arm**

Find:

```rust
            Actions::PyenvVirtualenv(a) => a,
            Actions::ZshOhMyZsh(a) => a,
```

(This is the second block containing `PyenvVirtualenv(a) => a,` — in the `Deref` impl, not `inner_ref`.) Add between them:

```rust
            Actions::PyenvVirtualenv(a) => a,
            Actions::TerraformTfenv(a) => a,
            Actions::ZshOhMyZsh(a) => a,
```

Note: `PyenvVirtualenv(a) => a,` appears in both `inner_ref()` and `Deref`. Use enough surrounding context lines to target the correct block. If Edit fails with "Found 2 matches", use `replace_all: true` — both blocks need the same addition.

- [ ] **Step 7: Add `Display` match arm**

Find:

```rust
            Actions::PyenvVirtualenv(_) => "pyenv.virtualenv",
            Actions::ZshOhMyZsh(_) => "zsh.oh-my-zsh",
```

Add between them:

```rust
            Actions::PyenvVirtualenv(_) => "pyenv.virtualenv",
            Actions::TerraformTfenv(_) => "terraform.tfenv",
            Actions::ZshOhMyZsh(_) => "zsh.oh-my-zsh",
```

- [ ] **Step 8: Run unit tests (GREEN)**

```bash
cargo test -p etch-lib terraform::tfenv 2>&1
```

Expected: all 12 tests pass.

- [ ] **Step 9: Commit**

```bash
git add lib/src/actions/terraform/tfenv.rs lib/src/actions/terraform/mod.rs lib/src/actions/mod.rs
git commit -m "feat(terraform): add terraform.tfenv action"
```

---

### Task 3: Update the three dispatch tests in `actions/mod.rs`

The three dispatch tests assert the total variant count. Adding `TerraformTfenv` raises the count from 47 to 48.

**Files:**

- Modify: `lib/src/actions/mod.rs`

- [ ] **Step 1: Confirm the dispatch count tests fail**

```bash
cargo test -p etch-lib all_major_action_variants 2>&1
cargo test -p etch-lib all_action_variants_display 2>&1
```

Expected: both fail on `assert_eq!(47, ...)`.

- [ ] **Step 2: Update `all_major_action_variants_can_be_deserialized`**

Find in the test YAML:

```yaml
- action: pyenv.virtualenv
  python_version: "3.12.0"
  name: myproject
- action: zsh.oh-my-zsh
```

Add `terraform.tfenv` between them:

```yaml
- action: pyenv.virtualenv
  python_version: "3.12.0"
  name: myproject
- action: terraform.tfenv
- action: zsh.oh-my-zsh
```

Update count assertion:

```rust
assert_eq!(48, manifest.actions.len());
```

- [ ] **Step 3: Update `all_remaining_action_variants_notify_returns_slice`**

Find in the test YAML:

```yaml
- action: zsh.oh-my-zsh
```

Note: if this YAML list doesn't include `pyenv.virtualenv` or `zsh.oh-my-zsh` already, search for the last action before the closing `"#`. Add `terraform.tfenv` before `zsh.oh-my-zsh`. Update the count assertion:

Current count is 8 — update to:

```rust
assert_eq!(9, m.actions.len());
```

And add `- action: terraform.tfenv` before `- action: zsh.oh-my-zsh` in that test's YAML.

- [ ] **Step 4: Update `all_action_variants_display`**

Find in the test YAML:

```yaml
- action: pyenv.virtualenv
  python_version: "3.12.0"
  name: myproject
- action: zsh.oh-my-zsh
```

Add `terraform.tfenv` between them:

```yaml
- action: pyenv.virtualenv
  python_version: "3.12.0"
  name: myproject
- action: terraform.tfenv
- action: zsh.oh-my-zsh
```

Update count assertion:

```rust
assert_eq!(48, manifest.actions.len());
```

Add `names.contains` assertion after `pyenv.virtualenv`:

```rust
        assert!(names.contains(&"pyenv.virtualenv".to_string()));
        assert!(names.contains(&"terraform.tfenv".to_string()));
        assert!(names.contains(&"zsh.oh-my-zsh".to_string()));
```

- [ ] **Step 5: Run all three dispatch tests (GREEN)**

```bash
cargo test -p etch-lib all_major_action_variants 2>&1
cargo test -p etch-lib all_remaining_action_variants_notify 2>&1
cargo test -p etch-lib all_action_variants_display 2>&1
```

Expected: all pass.

- [ ] **Step 6: Run full test suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass, lint clean.

- [ ] **Step 7: Commit**

```bash
git add lib/src/actions/mod.rs
git commit -m "test: update dispatch tests for terraform.tfenv (48 variants)"
```

---

### Task 4: Add example manifest and update docs

**Files:**

- Create: `examples/terraform/tfenv.yaml`
- Modify: `README.md`

- [ ] **Step 1: Create example manifest**

Create `examples/terraform/tfenv.yaml`:

```yaml
# Install tfenv and Terraform 1.9.0, set as global default.
#
# tfenv is cloned to ~/.tfenv. Add ~/.tfenv/bin to PATH separately,
# e.g. in .zshrc: export PATH="$HOME/.tfenv/bin:$PATH"
#
# Idempotent:
#   - skips clone if ~/.tfenv already exists
#   - skips install if ~/.tfenv/versions/1.9.0 already exists
#   - tfenv use is always run (idempotent — just writes ~/.terraform-version)

- action: terraform.tfenv
  version: "1.9.0"
  where: 'os.family == "linux" or os.name == "macos"'

---
# Install tfenv only (no specific Terraform version)
- action: terraform.tfenv
  where: 'os.family == "linux" or os.name == "macos"'
```

- [ ] **Step 2: Update README.md action catalog**

In `README.md`, find the action catalog table. Locate `macos.softwareupdate` or nearby entries. Add `terraform.tfenv` in the correct alphabetical section (after the `systemd` group, before `user` group or in the standalone entry area). Add:

```markdown
| `terraform.tfenv` | Install tfenv (Terraform version manager) via git clone, optionally install and activate a specific Terraform version. Idempotent. |
```

- [ ] **Step 3: Run tests to confirm nothing broke**

```bash
make test 2>&1 | tail -5
```

Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add examples/terraform/tfenv.yaml README.md
git commit -m "docs: add terraform.tfenv example and README catalog entry"
```

---

### Task 5: Open PR and monitor CI

- [ ] **Step 1: Push branch**

```bash
git push -u origin <branch-name>
```

- [ ] **Step 2: Open PR**

```bash
gh pr create --repo brujack/etch-cli --title "feat(terraform): add terraform.tfenv action" --body "$(cat <<'EOF'
## Summary
- Adds `terraform.tfenv` action: installs tfenv via git clone, optionally installs and activates a Terraform version
- Optional `version:` field — omit to install tfenv only, provide to install + use a specific version
- 3 steps when version set (clone, install, use); 1 step without
- Idempotent: SkipIf `~/.tfenv` exists (clone), SkipIf `~/.tfenv/versions/<version>` exists (install)
- 48 registered action variants (was 47)

## Test plan
- [x] All 12 unit tests in `terraform/tfenv.rs` pass
- [x] Three dispatch tests pass with updated counts (48) and YAML lists
- [x] `make test` green locally

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 3: Monitor CI**

```bash
gh pr checks <number> --repo brujack/etch-cli --watch
```

Expected: `test`, `secret-scan`, `cargo-audit`, `snyk-scan`, `docs-lint`, `docs-build` all green; `semver-check` advisory failure (expected — `enum_variant_added`).

- [ ] **Step 4: After PR auto-merges, clean up**

```bash
git fetch --prune
git reset --hard origin/main
git branch -D <branch-name>
git push origin --delete <branch-name>
```

---

### Task 6: Post-merge docs update (on main, not in worktree)

> **Do this directly on main after the PR merges — not inside the worktree.**

- [ ] **Step 1: Update `docs/superpowers/README.md`**

Add row to the All Plans table:

```markdown
| 2026-06-09 | [terraform-tfenv](plans/2026-06-09-terraform-tfenv-plan.md) | [terraform-tfenv](specs/2026-06-09-terraform-tfenv-design.md) | Done |
```

Remove the `tfenv action` backlog entry from the Backlog table.

Add `> **Status: DONE**` banner at the top of `docs/superpowers/plans/2026-06-09-terraform-tfenv-plan.md`.

Update `CLAUDE.md`: bump action count 47 → 48, add `terraform.tfenv` to the action catalog in `docs/knowledge/action-catalog.md`.

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-06-09-terraform-tfenv-plan.md CLAUDE.md docs/knowledge/action-catalog.md
git commit -m "docs(superpowers): mark terraform.tfenv Done, prune tfenv backlog entry"
git push
```
