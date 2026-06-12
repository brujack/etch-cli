> **Status: DONE** — Merged in etch-cli#104

# user.default_shell Action — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `user.default_shell` action that sets the current (or named) user's login shell via `chsh`, idempotent via `uzers` at plan time.

**Architecture:** Single new file `lib/src/actions/user/default_shell.rs` holds the struct and `impl Action`. `plan()` reads the target user's current shell from `/etc/passwd` using `uzers::get_current_user()` / `uzers::get_user_by_name()` and emits a `chsh` step only when a change is needed. `username: Some(name)` sets `privileged: true` on the `Exec` atom. Tasks 1 and 2 are committed together — `UserDefaultShell` is dead code until it is registered in `actions/mod.rs`.

**Tech Stack:** Rust, `uzers` crate (already a dep), `crate::atoms::command::Exec`, `serde`, `schemars`.

---

### Task 1: Action struct + plan() — TDD (commit together with Task 2)

**Files:**

- Create: `lib/src/actions/user/default_shell.rs`

- [ ] **Step 1: Write failing tests**

Create `lib/src/actions/user/default_shell.rs` with this content (tests first, struct and impl stubbed to compile):

```rust
use crate::actions::Action;
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserDefaultShell {
    pub shell: String,
    pub username: Option<String>,
}

impl Action for UserDefaultShell {
    fn summarize(&self) -> String {
        match &self.username {
            Some(u) => format!("Setting default shell for {} to {}", u, self.shell),
            None => format!("Setting default shell to {}", self.shell),
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    #[test]
    fn plan_errors_when_shell_empty() {
        let action = UserDefaultShell {
            shell: String::new(),
            username: None,
        };
        assert!(action.plan(&Manifest::default(), &Contexts::default()).is_err());
    }

    #[test]
    fn plan_skips_when_shell_matches_current_user() {
        let current_shell = uzers::get_current_user()
            .map(|u| u.shell().to_string_lossy().into_owned())
            .unwrap_or_else(|| "/bin/sh".to_string());
        let action = UserDefaultShell {
            shell: current_shell,
            username: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert!(steps.is_empty(), "expected skip when shell already matches");
    }

    #[test]
    fn plan_emits_chsh_when_shell_differs_current_user() {
        let action = UserDefaultShell {
            shell: String::from("/bin/definitely-not-a-shell-xyzzy"),
            username: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let step_display = format!("{}", steps[0].atom);
        assert!(
            step_display.contains("chsh") || step_display.contains("definitely-not"),
            "unexpected step: {step_display}"
        );
    }

    #[test]
    fn plan_emits_privileged_chsh_with_username() {
        let action = UserDefaultShell {
            shell: String::from("/bin/zsh"),
            username: Some(String::from("testuser-xyzzy-nonexistent")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        // User doesn't exist → shell won't match → step emitted
        assert_eq!(1, steps.len());
    }

}
```

Note: deserialization tests (`deserialization_with_shell_only`, `deserialization_with_username`) are NOT included here — they reference `Actions::UserDefaultShell` which won't exist until Task 2. They are added in Task 2 Step 11 after registration is complete.

- [ ] **Step 2: Run tests (RED — plan() is todo!())**

```bash
cargo test -p etch-lib 'actions::user::default_shell::tests' 2>&1 | tail -15
```

Expected: 4 tests compile; 3 that call `plan()` panic with `not yet implemented`; `plan_errors_when_shell_empty` may also panic. Compile succeeds. This is correct RED state.

- [ ] **Step 3: Implement plan()**

Replace the `todo!()` body in `plan()`:

```rust
fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
    if self.shell.is_empty() {
        anyhow::bail!("user.default_shell requires 'shell' to be specified");
    }

    let current_shell: Option<String> = match &self.username {
        Some(name) => uzers::get_user_by_name(name.as_str())
            .map(|u| u.shell().to_string_lossy().into_owned()),
        None => uzers::get_current_user()
            .map(|u| u.shell().to_string_lossy().into_owned()),
    };

    if current_shell.as_deref() == Some(self.shell.as_str()) {
        return Ok(vec![]);
    }

    let mut args = vec![String::from("-s"), self.shell.clone()];
    let privileged = self.username.is_some();
    if let Some(name) = &self.username {
        args.push(name.clone());
    }

    Ok(vec![Step {
        atom: Box::new(Exec {
            command: String::from("chsh"),
            arguments: args,
            privileged,
            ..Default::default()
        }),
        initializers: vec![],
        finalizers: vec![],
    }])
}
```

- [ ] **Step 4: Run plan() tests (GREEN)**

```bash
cargo test -p etch-lib 'actions::user::default_shell::tests' 2>&1 | tail -15
```

Expected: `plan_errors_when_shell_empty`, `plan_skips_when_shell_matches_current_user`, `plan_emits_chsh_when_shell_differs_current_user`, `plan_emits_privileged_chsh_with_username` all pass. `deserialization_*` tests still fail (struct not yet registered in `actions/mod.rs` — expected RED).

**Do NOT commit yet** — `UserDefaultShell` is unreachable dead code until registered. Proceed to Task 2.

---

### Task 2: Register action in mod files (commit together with Task 1)

**Files:**

- Modify: `lib/src/actions/user/mod.rs`
- Modify: `lib/src/actions/mod.rs`

- [ ] **Step 1: Add module declaration to `user/mod.rs`**

In `lib/src/actions/user/mod.rs`, add `pub mod default_shell;` after the existing module declarations:

```rust
pub mod add;
pub mod add_group;
pub mod default_shell;
pub mod providers;
```

- [ ] **Step 2: Add `use` import to `actions/mod.rs`**

In `lib/src/actions/mod.rs`, add adjacent to the existing user imports (around line 63–66):

```rust
use user::add::UserAdd;
use user::add_group::UserAddGroup;
use user::default_shell::UserDefaultShell;
```

- [ ] **Step 3: Add enum variant**

In `lib/src/actions/mod.rs`, in the `Actions` enum, add adjacent to the existing user variants (after `UserAddGroup`):

```rust
    #[serde(rename = "user.group")]
    UserAddGroup(ConditionalVariantAction<UserAddGroup>),

    #[serde(rename = "user.default_shell")]
    UserDefaultShell(ConditionalVariantAction<UserDefaultShell>),
```

- [ ] **Step 4: Add match arm to `inner_ref()`**

In the `inner_ref()` impl block, add adjacent to `UserAdd`/`UserAddGroup`:

```rust
            Actions::UserAdd(a) => a,
            Actions::UserAddGroup(a) => a,
            Actions::UserDefaultShell(a) => a,
```

- [ ] **Step 5: Add match arm to `notify` accessor**

In the `notify` accessor impl block, add adjacent to `UserAdd`/`UserAddGroup`:

```rust
            Actions::UserAdd(a) => &a.notify,
            Actions::UserAddGroup(a) => &a.notify,
            Actions::UserDefaultShell(a) => &a.notify,
```

- [ ] **Step 6: Add match arm to `Deref` impl**

In the `Deref` impl block, add adjacent to `UserAdd`/`UserAddGroup`:

```rust
            Actions::UserAdd(a) => a,
            Actions::UserAddGroup(a) => a,
            Actions::UserDefaultShell(a) => a,
```

- [ ] **Step 7: Add match arm to `Display` impl**

In the `Display` impl block, add adjacent to `UserAdd`/`UserAddGroup`:

```rust
            Actions::UserAdd(_) => "user.add",
            Actions::UserAddGroup(_) => "user.group",
            Actions::UserDefaultShell(_) => "user.default_shell",
```

- [ ] **Step 8: Update `all_major_action_variants_can_be_deserialized` test**

This test (around line 610–721) has a YAML with 34 actions. Add one entry adjacent to the existing `user.add` entry and update the count:

```yaml
- action: user.add
  username: alice
- action: user.default_shell
  shell: /bin/zsh
```

Change `assert_eq!(34, manifest.actions.len());` → `assert_eq!(35, manifest.actions.len());`

- [ ] **Step 9: Update `all_action_variants_inner_ref_and_deref` test**

This test (around line 990–1116) has a YAML with 48 actions. Add one entry adjacent to the existing user entries:

```yaml
- action: user.add
  username: alice
- action: user.group
  username: alice
  group_name: staff
- action: user.default_shell
  shell: /bin/zsh
```

Change `assert_eq!(48, manifest.actions.len());` → `assert_eq!(49, manifest.actions.len());`

- [ ] **Step 10: Update `all_action_variants_display` test**

This test (around line 1300–1465) has a YAML with 48 actions. Add one entry adjacent to the existing user entries:

```yaml
- action: user.add
  username: alice
- action: user.group
  username: alice
  group_name: staff
- action: user.default_shell
  shell: /bin/zsh
```

Change `assert_eq!(48, manifest.actions.len());` → `assert_eq!(49, manifest.actions.len());`

Add assertion in the `names.contains` block adjacent to existing user assertions:

```rust
        assert!(names.contains(&"user.add".to_string()));
        assert!(names.contains(&"user.group".to_string()));
        assert!(names.contains(&"user.default_shell".to_string()));
```

- [ ] **Step 11: Add deserialization tests to `default_shell.rs`**

Now that `Actions::UserDefaultShell` exists, append these two tests to the `#[cfg(test)] mod tests` block in `lib/src/actions/user/default_shell.rs`:

```rust
    #[test]
    fn deserialization_with_shell_only() {
        use crate::actions::Actions;
        let yaml = r#"
- action: user.default_shell
  shell: /bin/zsh
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::UserDefaultShell(action)) => {
                assert_eq!("/bin/zsh", action.action.shell);
                assert!(action.action.username.is_none());
            }
            _ => panic!("expected UserDefaultShell"),
        }
    }

    #[test]
    fn deserialization_with_username() {
        use crate::actions::Actions;
        let yaml = r#"
- action: user.default_shell
  shell: /bin/zsh
  username: alice
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::UserDefaultShell(action)) => {
                assert_eq!("/bin/zsh", action.action.shell);
                assert_eq!(Some("alice".to_string()), action.action.username);
            }
            _ => panic!("expected UserDefaultShell"),
        }
    }
```

- [ ] **Step 12: Run all tests (GREEN)**

```bash
cargo test -p etch-lib 2>&1 | tail -10
```

Expected: all tests pass including the 2 deserialization tests and all 3 dispatch tests.

- [ ] **Step 13: Run full test suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass, lint clean.

- [ ] **Step 14: Commit (Tasks 1 + 2 together)**

```bash
git add lib/src/actions/user/default_shell.rs lib/src/actions/user/mod.rs lib/src/actions/mod.rs
git commit -m "$(cat <<'EOF'
feat(user): add user.default_shell action via chsh

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Examples and docs

**Files:**

- Modify: `examples/user/user.yaml`
- Modify: `README.md`
- Modify: `docs/knowledge/action-catalog.md`
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update `examples/user/user.yaml`**

Append to the existing file:

```yaml
# Change the current user's default login shell. Idempotent: no-op if already correct.
# Shell must be listed in /etc/shells — chsh errors otherwise.
- action: user.default_shell
  shell: /bin/zsh

# Change another user's default shell (requires privilege escalation).
- action: user.default_shell
  shell: /bin/zsh
  username: bruce
  where: 'os.name == "linux"'
```

- [ ] **Step 2: Update README.md action catalog**

Find the `user.add` / `user.group` row in the action catalog table in `README.md`. Add a new row for `user.default_shell`. The description should read:

> Change login shell via `chsh`. Idempotent: reads current shell from `/etc/passwd` at plan time; skips if already correct. `username:` targets another user (requires privilege escalation). Fields: `shell` (required), `username` (optional).

- [ ] **Step 3: Update `docs/knowledge/action-catalog.md`**

Find the `user.add` row. Add a row for `user.default_shell`:

| `user.default_shell` | Change login shell via `chsh`. Idempotent: reads current shell from `/etc/passwd` via `uzers` at plan time; skips if already correct. `username:` targets another user (`privileged: true`). Shell must be in `/etc/shells` — `chsh` validates this at execute time. | `shell` (required), `username` (optional — defaults to current user) |

- [ ] **Step 4: Update CLAUDE.md action count**

Change `48 actions` → `49 actions` in two places:

- Line starting with `│       ├── actions/          # 48 action types`
- Line starting with `48 actions — full field reference`

- [ ] **Step 5: Commit**

```bash
git add examples/user/user.yaml README.md docs/knowledge/action-catalog.md CLAUDE.md
git commit -m "$(cat <<'EOF'
docs(user.default_shell): add example, catalog entries, bump action count

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Open PR and monitor CI

- [ ] **Step 1: Push branch**

```bash
git push -u origin <branch-name>
```

- [ ] **Step 2: Open PR**

```bash
gh pr create --repo brujack/etch-cli \
  --title "feat(user): add user.default_shell action" \
  --body "$(cat <<'EOF'
## Summary
- Adds `user.default_shell` action with fields `shell:` (required) and `username:` (optional)
- Idempotent: reads current shell from `/etc/passwd` via `uzers` at plan time; no-op if already correct
- `username:` targets another user and sets `privileged: true` on the `chsh` step
- Eliminates `command.run` workaround in dotfiles for `chsh -s /bin/zsh`

## Test plan
- [x] `plan_errors_when_shell_empty`
- [x] `plan_skips_when_shell_matches_current_user`
- [x] `plan_emits_chsh_when_shell_differs_current_user`
- [x] `plan_emits_privileged_chsh_with_username`
- [x] `deserialization_with_shell_only`
- [x] `deserialization_with_username`
- [x] All 3 dispatch tests updated (counts 48→49)
- [x] `make test` green

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 3: Monitor CI**

```bash
gh pr checks <number> --repo brujack/etch-cli --watch
```

Expected: all required jobs green; auto-merge fires. `semver-check` advisory failure expected (new enum variant added).

- [ ] **Step 4: Post-merge cleanup**

> **Do this directly on main after the PR merges — not inside the worktree.**

```bash
git fetch --prune && git reset --hard origin/main
git branch -D <branch-name>
```

Update `docs/superpowers/README.md` — change `user-default-shell` row status from `Pending` to `Done`:

```markdown
| 2026-06-10 | [user-default-shell](plans/2026-06-10-user-default-shell-plan.md) | [user-default-shell](specs/2026-06-10-user-default-shell-design.md) | Done |
```

Add `> **Status: DONE**` banner at the top of this plan file.

Commit and push.
