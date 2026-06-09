# macos.softwareupdate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a zero-field `macos.softwareupdate` action that runs `softwareupdate --install --all` as a privileged, self-idempotent step.

**Architecture:** Single Rust file following the `macos.rosetta` pattern — empty struct, one privileged `Exec` step, no `SkipIf` initializer (softwareupdate self-no-ops when nothing to install). Registered in `actions/mod.rs` with the standard 5-arm pattern.

**Tech Stack:** Rust, serde/serde_yaml_ng, schemars, existing `crate::atoms::command::Exec`, `crate::steps::Step`, `crate::utilities::get_privilege_provider`

---

### Task 1: Create `softwareupdate.rs` (TDD — failing tests first)

**Files:**

- Create: `lib/src/actions/macos/softwareupdate.rs`

- [ ] **Step 1: Write the failing tests**

Create `lib/src/actions/macos/softwareupdate.rs` with tests only — no impl yet:

```rust
#[cfg(test)]
mod tests {
    use crate::actions::Actions;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    use super::*;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: macos.softwareupdate
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSSoftwareUpdate(_)) => {}
            _ => panic!("MacOSSoftwareUpdate didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn summarize_returns_non_empty_string() {
        let action = MacOSSoftwareUpdate {};
        assert!(!action.summarize().is_empty());
    }

    #[test]
    fn plan_returns_one_step() {
        let action = MacOSSoftwareUpdate {};
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn plan_step_runs_softwareupdate_install_all() {
        let action = MacOSSoftwareUpdate {};
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("softwareupdate"),
            "expected 'softwareupdate' in: {display}"
        );
        assert!(
            display.contains("--install"),
            "expected '--install' in: {display}"
        );
        assert!(
            display.contains("--all"),
            "expected '--all' in: {display}"
        );
    }

    #[test]
    fn plan_step_is_privileged() {
        let action = MacOSSoftwareUpdate {};
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("privileged=true"),
            "expected privileged=true in: {display}"
        );
    }

    #[test]
    fn plan_step_has_no_initializers() {
        let action = MacOSSoftwareUpdate {};
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(
            steps[0].initializers.len(),
            0,
            "expected no initializers — softwareupdate is self-idempotent"
        );
    }
}
```

- [ ] **Step 2: Run to confirm compile error (RED)**

```bash
cargo test -p etch-lib 2>&1 | head -20
```

Expected: compile error — `MacOSSoftwareUpdate` not defined, `Actions::MacOSSoftwareUpdate` variant not found.

- [ ] **Step 3: Add the struct and impl above the tests**

Add to the top of `lib/src/actions/macos/softwareupdate.rs` (before the `#[cfg(test)]` block):

```rust
use crate::actions::Action;
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::utilities;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "macos.softwareupdate")]
pub struct MacOSSoftwareUpdate {}

impl Action for MacOSSoftwareUpdate {
    fn summarize(&self) -> String {
        String::from("Installing macOS software updates")
    }

    fn plan(&self, _manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let privilege_provider =
            utilities::get_privilege_provider(context).unwrap_or_else(|| "sudo".to_string());

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("softwareupdate"),
                arguments: vec![String::from("--install"), String::from("--all")],
                privileged: true,
                privilege_provider,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}
```

- [ ] **Step 4: Confirm tests still don't pass (Actions::MacOSSoftwareUpdate not registered)**

```bash
cargo test -p etch-lib macos::softwareupdate 2>&1 | head -20
```

Expected: compile error — `Actions::MacOSSoftwareUpdate` variant doesn't exist yet. That's correct — registration comes in Task 2.

---

### Task 2: Register `MacOSSoftwareUpdate` in mod files

**Files:**

- Modify: `lib/src/actions/macos/mod.rs`
- Modify: `lib/src/actions/mod.rs`

- [ ] **Step 1: Export from `macos/mod.rs`**

In `lib/src/actions/macos/mod.rs`, add `softwareupdate` alongside the existing modules:

```rust
mod default;
mod rosetta;
mod service;
mod softwareupdate;
pub use default::MacOSDefault;
pub use rosetta::MacOSRosetta;
pub use service::MacOSService;
pub use softwareupdate::MacOSSoftwareUpdate;
```

- [ ] **Step 2: Add `MacOSSoftwareUpdate` to the use import in `actions/mod.rs`**

Find line 24 in `lib/src/actions/mod.rs`:

```rust
use crate::actions::macos::{MacOSDefault, MacOSRosetta, MacOSService};
```

Replace with:

```rust
use crate::actions::macos::{MacOSDefault, MacOSRosetta, MacOSService, MacOSSoftwareUpdate};
```

- [ ] **Step 3: Add enum variant**

In `lib/src/actions/mod.rs`, find the `macos.service` variant:

```rust
    #[serde(rename = "macos.service")]
    MacOSService(ConditionalVariantAction<MacOSService>),
```

Add `macos.softwareupdate` immediately after it (before `systemd.service`):

```rust
    #[serde(rename = "macos.service")]
    MacOSService(ConditionalVariantAction<MacOSService>),

    #[serde(rename = "macos.softwareupdate")]
    MacOSSoftwareUpdate(ConditionalVariantAction<MacOSSoftwareUpdate>),
```

- [ ] **Step 4: Add `inner_ref()` match arm**

In the `inner_ref()` impl block, find:

```rust
            Actions::MacOSRosetta(a) => a,
            Actions::MacOSService(a) => a,
```

Add after `MacOSService`:

```rust
            Actions::MacOSRosetta(a) => a,
            Actions::MacOSService(a) => a,
            Actions::MacOSSoftwareUpdate(a) => a,
```

- [ ] **Step 5: Add `notify` match arm**

In the `notify()` impl block, find:

```rust
            Actions::MacOSRosetta(a) => &a.notify,
            Actions::MacOSService(a) => &a.notify,
```

Add after `MacOSService`:

```rust
            Actions::MacOSRosetta(a) => &a.notify,
            Actions::MacOSService(a) => &a.notify,
            Actions::MacOSSoftwareUpdate(a) => &a.notify,
```

- [ ] **Step 6: Add `Deref` match arm**

In the `Deref` impl block, find:

```rust
            Actions::MacOSRosetta(a) => a,
            Actions::MacOSService(a) => a,
```

Add after `MacOSService`:

```rust
            Actions::MacOSRosetta(a) => a,
            Actions::MacOSService(a) => a,
            Actions::MacOSSoftwareUpdate(a) => a,
```

- [ ] **Step 7: Add `Display` match arm**

In the `Display` impl block, find:

```rust
            Actions::MacOSRosetta(_) => "macos.rosetta",
            Actions::MacOSService(_) => "macos.service",
```

Add after `MacOSService`:

```rust
            Actions::MacOSRosetta(_) => "macos.rosetta",
            Actions::MacOSService(_) => "macos.service",
            Actions::MacOSSoftwareUpdate(_) => "macos.softwareupdate",
```

- [ ] **Step 8: Run softwareupdate unit tests (GREEN)**

```bash
cargo test -p etch-lib macos::softwareupdate 2>&1
```

Expected: all 6 tests pass.

- [ ] **Step 9: Commit**

```bash
git add lib/src/actions/macos/softwareupdate.rs lib/src/actions/macos/mod.rs lib/src/actions/mod.rs
git commit -m "feat(macos): add macos.softwareupdate action"
```

---

### Task 3: Update the three dispatch tests in `actions/mod.rs`

These tests verify every action variant appears in the enum. They fail until updated.

**Files:**

- Modify: `lib/src/actions/mod.rs`

- [ ] **Step 1: Confirm the three tests currently fail**

```bash
cargo test -p etch-lib all_major_action_variants 2>&1
cargo test -p etch-lib all_remaining_action_variants_notify 2>&1
cargo test -p etch-lib all_action_variants_display 2>&1
```

Expected: all three compile fine but `all_major_action_variants_can_be_deserialized` and `all_action_variants_display` fail on `assert_eq!(46, ...)` because `macos.softwareupdate` is now a valid variant that raises the count, and the YAML lists don't include it yet. (The notify test may pass — it's not exhaustive.)

- [ ] **Step 2: Update `all_major_action_variants_can_be_deserialized`**

In the YAML for this test, find:

```yaml
- action: macos.rosetta
- action: macos.service
  plist: /Library/LaunchDaemons/com.example.plist
  state: loaded
```

Add `macos.softwareupdate` between them:

```yaml
- action: macos.rosetta
- action: macos.softwareupdate
- action: macos.service
  plist: /Library/LaunchDaemons/com.example.plist
  state: loaded
```

Then update the count assertion:

```rust
assert_eq!(47, manifest.actions.len());
```

- [ ] **Step 3: Update `all_remaining_action_variants_notify_returns_slice`**

In the YAML for this test, find:

```yaml
- action: macos.rosetta
- action: macos.service
```

Add `macos.softwareupdate` between them:

```yaml
- action: macos.rosetta
- action: macos.softwareupdate
- action: macos.service
```

Update the count assertion:

```rust
assert_eq!(8, m.actions.len());
```

- [ ] **Step 4: Update `all_action_variants_display`**

In the YAML for this test, find:

```yaml
- action: macos.rosetta
- action: macos.service
  plist: /Library/LaunchAgents/com.example.plist
  state: loaded
```

Add `macos.softwareupdate` between them:

```yaml
- action: macos.rosetta
- action: macos.softwareupdate
- action: macos.service
  plist: /Library/LaunchAgents/com.example.plist
  state: loaded
```

Update the count assertion:

```rust
assert_eq!(47, manifest.actions.len());
```

Add the `names.contains` assertion after `macos.rosetta`:

```rust
        assert!(names.contains(&"macos.rosetta".to_string()));
        assert!(names.contains(&"macos.softwareupdate".to_string()));
        assert!(names.contains(&"macos.service".to_string()));
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
make test
```

Expected: all tests pass, lint clean.

- [ ] **Step 7: Commit**

```bash
git add lib/src/actions/mod.rs
git commit -m "test: update dispatch tests for macos.softwareupdate (47 variants)"
```

---

### Task 4: Add example manifest and update docs

**Files:**

- Create: `examples/macos-softwareupdate/macos-softwareupdate.yaml`
- Modify: `README.md`

- [ ] **Step 1: Create example manifest**

Create `examples/macos-softwareupdate/macos-softwareupdate.yaml`:

```yaml
# Install all available macOS software updates.
# Requires admin privileges — uses sudo by default (configure via etch.yaml privilege: field).
# softwareupdate is self-idempotent: no-ops when nothing to install.
# Always add where: guard — this action is macOS-only.

- action: macos.softwareupdate
  where: 'os.name == "macos"'
```

- [ ] **Step 2: Update README.md action catalog**

In `README.md`, find the action catalog table entry for `macos.rosetta` and add `macos.softwareupdate` after `macos.service` (keep alphabetical order within the `macos.*` group):

```markdown
| `macos.softwareupdate` | Install all available macOS software updates via `softwareupdate --install --all`. Privileged, self-idempotent. |
```

- [ ] **Step 3: Run tests to confirm nothing broke**

```bash
make test
```

Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add examples/macos-softwareupdate/macos-softwareupdate.yaml README.md
git commit -m "docs: add macos.softwareupdate example and README catalog entry"
```

---

### Task 5: Open PR and monitor CI

- [ ] **Step 1: Push branch**

```bash
git push -u origin <branch-name>
```

- [ ] **Step 2: Open PR**

```bash
gh pr create --repo brujack/etch-cli --title "feat(macos): add macos.softwareupdate action" --body "$(cat <<'EOF'
## Summary
- Adds `macos.softwareupdate` action: runs `softwareupdate --install --all` as a privileged, self-idempotent step
- Zero-field struct — no configuration surface needed
- No SkipIf initializer — softwareupdate self-no-ops when nothing to install
- 47 registered action variants (was 46)

## Test plan
- [ ] All 6 unit tests in `macos/softwareupdate.rs` pass
- [ ] Three dispatch tests pass with updated counts (47) and YAML lists
- [ ] `make test` green locally

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 3: Monitor CI**

```bash
gh pr checks <number> --repo brujack/etch-cli --watch
```

Expected: `test`, `secret-scan`, `cargo-audit`, `snyk-scan`, `docs-lint`, `docs-build` all green; `semver-check` advisory failure (expected — enum_variant_added).

- [ ] **Step 4: After PR auto-merges, clean up**

```bash
git fetch --prune
git reset --hard origin/main
git branch -D <branch-name>
```

---

### Task 6: Post-merge docs update (on main, not in worktree)

> **Do this directly on main after the PR merges — not inside the worktree.**

- [ ] **Step 1: Update `docs/superpowers/README.md`**

Add row to the All Plans table:

```markdown
| 2026-06-09 | [macos-softwareupdate](plans/2026-06-09-macos-softwareupdate-plan.md) | [macos-softwareupdate](specs/2026-06-09-macos-softwareupdate-design.md) | Done |
```

Remove the `macos.softwareupdate` backlog entry from the Backlog table.

Add `> **Status: DONE**` banner at top of `docs/superpowers/plans/2026-06-09-macos-softwareupdate-plan.md`.

Also remove the stale "chruby action" backlog entry (already implemented as `ruby-chruby`).

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-06-09-macos-softwareupdate-plan.md
git commit -m "docs(superpowers): mark macos.softwareupdate Done, remove stale chruby backlog entry"
git push
```
