# command.run skip_if_exists Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `skip_if_exists: Option<String>` to `command.run` so a step is skipped when the specified path already exists, eliminating inline `[ -d path ] ||` shell guards.

**Architecture:** Single field added to `RunCommand`. When set, `plan()` appends a `FlowControl::SkipIf(Box::new(FileExists(path)))` initializer after the existing `SetEnvVars` initializer. `FileExists` and `SkipIf` already exist in the step infrastructure — this is pure wiring.

**Tech Stack:** Rust, existing `FileExists` initializer (`lib/src/steps/initializers/file_exists.rs`), existing `FlowControl::SkipIf`

---

## Files

| File                                                          | Change                                                      |
| ------------------------------------------------------------- | ----------------------------------------------------------- |
| `lib/src/actions/command/run.rs`                              | Add `skip_if_exists` field, update imports, update `plan()` |
| `~/git-repos/personal/dotfiles/manifests/dotfiles/tools.yaml` | Replace oh-my-zsh inline guard with `skip_if_exists:`       |

---

### Task 1: Add field, implement, and test

**Files:**

- Modify: `lib/src/actions/command/run.rs`

- [ ] **Step 1: Write failing tests**

Add two new tests inside the existing `#[cfg(test)] mod tests { ... }` block in `lib/src/actions/command/run.rs`, after the last existing test:

```rust
    #[test]
    fn it_can_be_deserialized_with_skip_if_exists() {
        use crate::actions::Actions;
        let yaml = r#"
- action: command.run
  command: bash
  args:
    - "-c"
    - echo hello
  skip_if_exists: /tmp/test-path
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::CommandRun(action)) => {
                assert_eq!(
                    Some(String::from("/tmp/test-path")),
                    action.action.skip_if_exists
                );
            }
            _ => panic!("CommandRun didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn plan_includes_skip_if_initializer_when_set() {
        use crate::actions::Action;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = super::RunCommand {
            command: String::from("echo"),
            skip_if_exists: Some(String::from("/tmp/test-path")),
            ..Default::default()
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
        // SetEnvVars (Ensure) + FileExists (SkipIf) = 2 initializers
        assert_eq!(2, steps[0].initializers.len());
    }
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test -p etch-lib it_can_be_deserialized_with_skip_if_exists plan_includes_skip_if_initializer_when_set 2>&1 | tail -5
```

Expected: compile error — `RunCommand` has no field `skip_if_exists`.

- [ ] **Step 3: Add import and field to `RunCommand`**

Change the import at the top of `lib/src/actions/command/run.rs` from:

```rust
use crate::steps::finalizers::RemoveEnvVars;
use crate::steps::initializers::SetEnvVars;
```

to:

```rust
use crate::steps::finalizers::RemoveEnvVars;
use crate::steps::initializers::{FileExists, SetEnvVars};
use std::path::PathBuf;
```

Add `skip_if_exists` field to `RunCommand` after the `env` field:

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunCommand {
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default = "get_false", alias = "sudo")]
    pub privileged: bool,

    #[serde(default = "get_cwd")]
    pub dir: String,

    #[serde(default)]
    pub env: HashMap<String, String>,

    pub skip_if_exists: Option<String>,
}
```

- [ ] **Step 4: Update `plan()` to wire the initializer**

Replace the current `plan()` return from:

```rust
        Ok(vec![Step {
            atom: Box::new(Exec {
                command: self.command.clone(),
                arguments: self.args.clone(),
                privileged: self.privileged,
                working_dir: Some(self.dir.clone()),
                privilege_provider: privilege_provider.clone(),
                ..Default::default()
            }),
            initializers: vec![steps::initializers::FlowControl::Ensure(Box::new(
                SetEnvVars(self.env.clone()),
            ))],
            finalizers: vec![steps::finalizers::FlowControl::Ensure(Box::new(
                RemoveEnvVars(self.env.clone()),
            ))],
        }])
```

to:

```rust
        let mut initializers = vec![steps::initializers::FlowControl::Ensure(Box::new(
            SetEnvVars(self.env.clone()),
        ))];

        if let Some(path) = &self.skip_if_exists {
            initializers.push(steps::initializers::FlowControl::SkipIf(Box::new(
                FileExists(PathBuf::from(path)),
            )));
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: self.command.clone(),
                arguments: self.args.clone(),
                privileged: self.privileged,
                working_dir: Some(self.dir.clone()),
                privilege_provider: privilege_provider.clone(),
                ..Default::default()
            }),
            initializers,
            finalizers: vec![steps::finalizers::FlowControl::Ensure(Box::new(
                RemoveEnvVars(self.env.clone()),
            ))],
        }])
```

- [ ] **Step 5: Run tests to confirm both new tests pass**

```bash
cargo test -p etch-lib it_can_be_deserialized_with_skip_if_exists plan_includes_skip_if_initializer_when_set 2>&1 | tail -10
```

Expected: 2 tests PASS.

- [ ] **Step 6: Run full suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass. The existing `plan_returns_one_step_with_initializer_and_finalizer` test is unaffected — it uses `..Default::default()` which sets `skip_if_exists: None`, so initializers count remains 1.

- [ ] **Step 7: Commit**

```bash
git add lib/src/actions/command/run.rs
git commit -m "feat: add skip_if_exists to command.run

When set, the step is skipped if the path already exists.
Replaces inline [ -d path ] || shell guards.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Update dotfiles tools.yaml to use skip_if_exists

**Files:**

- Modify: `~/git-repos/personal/dotfiles/manifests/dotfiles/tools.yaml`

- [ ] **Step 1: Read current tools.yaml**

```bash
cat ~/git-repos/personal/dotfiles/manifests/dotfiles/tools.yaml
```

Note the oh-my-zsh install step with the inline guard.

- [ ] **Step 2: Replace the inline guard**

Change the oh-my-zsh install action from:

```yaml
- action: command.run
  command: bash
  args:
      - "-c"
      - '[ -d {{ user.home_dir }}/.oh-my-zsh ] || RUNZSH=no KEEP_ZSHRC=yes sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"'
```

to:

```yaml
- action: command.run
  command: bash
  args:
      - "-c"
      - 'RUNZSH=no KEEP_ZSHRC=yes sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"'
  skip_if_exists: "{{ user.home_dir }}/.oh-my-zsh"
```

- [ ] **Step 3: Verify with dry-run (using the new binary)**

Build etch first if needed:

```bash
cargo build --bin etch 2>&1 | tail -3
```

Then verify:

```bash
etch --config ~/git-repos/personal/dotfiles/etch.yaml apply --dry-run -v -m dotfiles.tools 2>&1 | head -10
```

Expected: dry-run output shows the tools manifest; oh-my-zsh step shows as "nothing to be done" (since `.oh-my-zsh` exists on this machine) rather than producing a command.run step.

- [ ] **Step 4: Commit to dotfiles**

```bash
cd ~/git-repos/personal/dotfiles
git add manifests/dotfiles/tools.yaml
git commit -m "feat(etch): use skip_if_exists for oh-my-zsh install

Replaces inline shell guard with declarative skip_if_exists field.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
git push
```

---

### Task 3: Docs update (post-merge on main)

**Do this directly on main after the PR merges — not inside the worktree.**

- [ ] **Step 1: Mark spec Done in README**

Change `command-run-skip-if-exists` row from `Pending` to `Done`.

- [ ] **Step 2: Update CLAUDE.md action catalog**

Update the `command.run` row to mention the new field:

```markdown
| `command.run` | Run shell commands | `command`, `args`, `privileged` (bool), `skip_if_exists` (path — skip step if path exists) |
```

- [ ] **Step 3: Commit and push on main**

```bash
git add docs/superpowers/README.md CLAUDE.md
git commit -m "docs: mark command-run-skip-if-exists Done; update action catalog"
git push origin main
```
