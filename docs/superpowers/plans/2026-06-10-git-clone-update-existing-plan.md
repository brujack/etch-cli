> **Status: DONE** — Merged in etch-cli#103

# git.clone update_existing field — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `update_existing: bool` (default false) to `git.clone` so the action clones if the directory is missing, or runs `git pull` if it already exists as a valid git repo.

**Architecture:** Two files change. `lib/src/atoms/git/clone.rs` gains the field on the struct, new `plan()` branches (validate `.git` presence when `update_existing: true`), and a new early-return in `execute()` that runs `git pull` via subprocess when the directory already exists. `lib/src/actions/git/clone.rs` gets the same field wired through to the atom. No new files. No changes to `lib/src/actions/mod.rs`.

**Tech Stack:** Rust, `std::process::Command`, `tempfile`, `serial_test` (already in dev-dependencies).

**Batching note:** Tasks 1 and 2 modify `atoms/git/clone.rs` and are committed together. Running `plan()` with `update_existing: true` and a pre-existing directory returns `should_run: true`, but `execute()` wouldn't handle it until Task 2 is done — the intermediate state is logically incomplete.

---

### Task 1: Atom `plan()` changes (TDD — commit with Task 2)

**Files:**

- Modify: `lib/src/atoms/git/clone.rs`

- [ ] **Step 1: Add `update_existing` field to `Clone` struct**

Replace the struct definition in `lib/src/atoms/git/clone.rs`:

```rust
#[derive(Default)]
pub struct Clone {
    pub repository: Url,
    pub directory: PathBuf,
    pub update_existing: bool,
}
```

- [ ] **Step 2: Write failing tests for the new `plan()` branches**

Add to the `#[cfg(test)]` module (after the existing tests):

```rust
#[test]
fn plan_skips_when_dir_exists_update_existing_false() {
    let tmp = tempfile::tempdir().unwrap();
    let atom = Clone {
        repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
        directory: tmp.path().to_path_buf(),
        update_existing: false,
    };
    let outcome = atom.plan().unwrap();
    assert!(!outcome.should_run);
}

#[test]
fn plan_runs_when_dir_exists_with_git_update_existing_true() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let atom = Clone {
        repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
        directory: tmp.path().to_path_buf(),
        update_existing: true,
    };
    let outcome = atom.plan().unwrap();
    assert!(outcome.should_run);
}

#[test]
fn plan_errors_when_dir_exists_no_git_update_existing_true() {
    let tmp = tempfile::tempdir().unwrap();
    // No .git directory — not a git repo
    let atom = Clone {
        repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
        directory: tmp.path().to_path_buf(),
        update_existing: true,
    };
    assert!(atom.plan().is_err());
}

#[test]
fn plan_runs_when_dir_missing_update_existing_true() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("not_yet");
    let atom = Clone {
        repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
        directory: target,
        update_existing: true,
    };
    let outcome = atom.plan().unwrap();
    assert!(outcome.should_run);
}
```

- [ ] **Step 3: Fix existing test struct literals**

The three existing tests in the `#[cfg(test)]` module construct `Clone` without `update_existing` — they will not compile after Step 1 adds the field. Add `update_existing: false` to each:

`display_format`:

```rust
let atom = Clone {
    repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
    directory: std::path::PathBuf::from("/tmp/repo"),
    update_existing: false,
};
```

`plan_should_run_when_directory_does_not_exist`:

```rust
let atom = Clone {
    repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
    directory: target,
    update_existing: false,
};
```

`plan_should_not_run_when_directory_exists`:

```rust
let atom = Clone {
    repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
    directory: tmp.path().to_path_buf(),
    update_existing: false,
};
```

- [ ] **Step 4: Run tests (RED — new tests fail because plan() has old logic)**

```bash
cargo test -p etch-lib 'atoms::git::clone::tests' 2>&1 | tail -15
```

Expected: tests compile, original 3 pass, 4 new tests fail (wrong `should_run` values or missing error). Confirms RED state.

- [ ] **Step 5: Replace `plan()` method body**

Replace the existing `plan()` in the `impl Atom for Clone` block:

```rust
#[instrument(name = "git.clone.plan", level = "info", skip(self))]
fn plan(&self) -> anyhow::Result<Outcome> {
    if self.directory.exists() {
        if self.update_existing {
            if !self.directory.join(".git").exists() {
                anyhow::bail!(
                    "directory {} exists but is not a git repository",
                    self.directory.display()
                );
            }
            return Ok(Outcome {
                side_effects: vec![],
                should_run: true,
            });
        }
        return Ok(Outcome {
            side_effects: vec![],
            should_run: false,
        });
    }
    Ok(Outcome {
        side_effects: vec![],
        should_run: true,
    })
}
```

- [ ] **Step 6: Run plan() tests (GREEN)**

```bash
cargo test -p etch-lib 'atoms::git::clone::tests' 2>&1 | tail -15
```

Expected: all existing tests pass + 4 new tests pass.

**Do NOT commit yet** — `execute()` hasn't been updated. Proceed to Task 2.

---

### Task 2: Atom `execute()` changes (TDD — commit with Task 1)

**Files:**

- Modify: `lib/src/atoms/git/clone.rs`

- [ ] **Step 1: Add imports for serial_test to the test module**

At the top of the `#[cfg(test)]` module, add:

```rust
use serial_test::serial;
```

The full test module open becomes:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
```

- [ ] **Step 2: Add mock git helper to the test module**

Add this function inside the `#[cfg(test)]` module (after the `use` lines, before the tests):

```rust
fn write_mock_git(mock_dir: &std::path::Path, calls_file: &std::path::Path, exit_code: i32) {
    use std::os::unix::fs::PermissionsExt;
    let script = mock_dir.join("git");
    let content = format!(
        "#!/usr/bin/env bash\nprintf 'git %s\\n' \"$*\" >> '{}'\nexit {}\n",
        calls_file.display(),
        exit_code
    );
    std::fs::write(&script, &content).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
}
```

- [ ] **Step 3: Write failing execute() tests**

Add to the `#[cfg(test)]` module:

```rust
#[test]
#[serial]
fn execute_pulls_when_dir_exists() {
    let tmp = tempfile::tempdir().unwrap();
    let mock_dir = tempfile::tempdir().unwrap();
    let calls_file = tmp.path().join("calls.log");
    write_mock_git(mock_dir.path(), &calls_file, 0);

    let target = tmp.path().join("existing_repo");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::create_dir_all(target.join(".git")).unwrap();

    let original_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var(
        "PATH",
        format!("{}:{}", mock_dir.path().display(), original_path),
    );

    let mut atom = Clone {
        repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
        directory: target.clone(),
        update_existing: true,
    };
    let result = atom.execute();

    std::env::set_var("PATH", &original_path);

    assert!(result.is_ok(), "execute failed: {:?}", result);
    let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
    assert!(log.contains("pull"), "expected 'pull' in log, got: {log}");
    assert!(log.contains("-C"), "expected '-C' flag in log, got: {log}");
}

#[test]
#[serial]
fn execute_propagates_pull_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let mock_dir = tempfile::tempdir().unwrap();
    let calls_file = tmp.path().join("calls.log");
    write_mock_git(mock_dir.path(), &calls_file, 1);

    let target = tmp.path().join("existing_repo");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::create_dir_all(target.join(".git")).unwrap();

    let original_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var(
        "PATH",
        format!("{}:{}", mock_dir.path().display(), original_path),
    );

    let mut atom = Clone {
        repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
        directory: target,
        update_existing: true,
    };
    let result = atom.execute();

    std::env::set_var("PATH", &original_path);

    assert!(result.is_err(), "expected Err from failed pull");
}
```

- [ ] **Step 4: Run execute() tests (RED)**

```bash
cargo test -p etch-lib 'atoms::git::clone::tests::execute_pull' 2>&1 | tail -15
```

Expected: `execute_pulls_when_dir_exists` fails (git CLI not called, gix clone attempted on existing dir) and/or `execute_propagates_pull_failure` fails.

- [ ] **Step 5: Replace `execute()` method body**

Replace the existing `execute()` in the `impl Atom for Clone` block:

```rust
#[instrument(name = "git.clone.execute", level = "info", skip(self))]
fn execute(&mut self) -> anyhow::Result<()> {
    if self.directory.exists() {
        // update_existing=true; plan() already validated .git exists
        let status = std::process::Command::new("git")
            .args(["-C", &self.directory.to_string_lossy(), "pull"])
            .status()?;
        if !status.success() {
            anyhow::bail!(
                "git -C {} pull failed with {}",
                self.directory.display(),
                status
            );
        }
        return Ok(());
    }

    unsafe {
        interrupt::init_handler(1, || {})?;
    };

    std::fs::create_dir_all(&self.directory)?;

    let mut prepare_clone = gix::prepare_clone(self.repository.clone(), &self.directory)?;
    let (mut prepare_checkout, _) = prepare_clone
        .fetch_then_checkout(gix::progress::Discard, &interrupt::IS_INTERRUPTED)?;

    let (repo, _) = prepare_checkout.main_worktree(Discard, &interrupt::IS_INTERRUPTED)?;

    let _ = repo
        .find_default_remote(gix::remote::Direction::Fetch)
        .expect("always present after clone")?;

    Ok(())
}
```

- [ ] **Step 6: Run all atom tests (GREEN)**

```bash
cargo test -p etch-lib 'atoms::git::clone::tests' 2>&1 | tail -15
```

Expected: all tests pass (3 original + 4 plan() + 2 execute() = 9 total).

- [ ] **Step 7: Run full test suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass, lint clean.

- [ ] **Step 8: Commit (Tasks 1 + 2 together)**

```bash
git add lib/src/atoms/git/clone.rs
git commit -m "feat(git.clone): add update_existing field — pull when dir exists"
```

---

### Task 3: Action struct + plan() wiring (TDD)

**Files:**

- Modify: `lib/src/actions/git/clone.rs`

- [ ] **Step 1: Write failing deserialization tests**

Add to the `#[cfg(test)]` module in `lib/src/actions/git/clone.rs`:

```rust
#[test]
fn deserialization_with_update_existing_true() {
    let yaml = r#"
- action: git.clone
  repo_url: https://github.com/example/repo.git
  directory: /tmp/repo
  update_existing: true
"#;
    let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
    match actions.pop() {
        Some(Actions::GitClone(action)) => {
            assert!(action.action.update_existing);
        }
        _ => panic!("GitClone didn't deserialize"),
    }
}

#[test]
fn deserialization_defaults_update_existing_false() {
    let yaml = r#"
- action: git.clone
  repo_url: https://github.com/example/repo.git
  directory: /tmp/repo
"#;
    let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
    match actions.pop() {
        Some(Actions::GitClone(action)) => {
            assert!(!action.action.update_existing);
        }
        _ => panic!("GitClone didn't deserialize"),
    }
}
```

- [ ] **Step 2: Run deserialization tests (RED — field not on struct yet)**

```bash
cargo test -p etch-lib 'actions::git::clone::tests' 2>&1 | tail -15
```

Expected: compile error — `update_existing` is not a field on `GitClone`.

- [ ] **Step 3: Add `update_existing` field to `GitClone` struct**

Replace the struct definition:

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitClone {
    pub repo_url: String,
    pub directory: String,
    #[serde(default)]
    pub update_existing: bool,
}
```

- [ ] **Step 4: Update `plan()` to pass field to atom**

Replace the `plan()` method body:

```rust
fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
    let url = gix::url::parse(self.repo_url.as_str().into())?;
    Ok(vec![Step {
        atom: Box::new(crate::atoms::git::Clone {
            repository: url.clone(),
            directory: PathBuf::from(self.directory.clone()),
            update_existing: self.update_existing,
        }),
        initializers: vec![],
        finalizers: vec![],
    }])
}
```

- [ ] **Step 5: Fix existing test struct literals**

The two existing tests construct `GitClone { repo_url: ..., directory: ... }` — these now need the new field. Update both:

In `plan_returns_one_step_for_valid_url`:

```rust
let action = GitClone {
    repo_url: String::from("https://github.com/example/repo.git"),
    directory: String::from("/tmp/repo"),
    update_existing: false,
};
```

In `plan_errors_on_invalid_url`:

```rust
let action = GitClone {
    repo_url: String::from("not a url ://"),
    directory: String::from("/tmp/repo"),
    update_existing: false,
};
```

- [ ] **Step 6: Run action tests (GREEN)**

```bash
cargo test -p etch-lib 'actions::git::clone::tests' 2>&1 | tail -15
```

Expected: all tests pass (3 original + 2 new = 5 total).

- [ ] **Step 7: Run full test suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass, lint clean.

- [ ] **Step 8: Commit**

```bash
git add lib/src/actions/git/clone.rs
git commit -m "feat(git.clone): wire update_existing field from action to atom"
```

---

### Task 4: Examples and docs

**Files:**

- Modify: `examples/git/clone.yaml`
- Modify: `README.md`

- [ ] **Step 1: Update `examples/git/clone.yaml`**

Replace the file contents with:

```yaml
actions:
    # Clone a git repository. If `directory` already exists, clone is skipped.
    - action: git.clone
      repo_url: https://github.com/brujack/etch-cli
      directory: "{{ user.home_dir }}/src/etch-cli"

    # Clone-or-pull: clones if directory is missing, pulls if it already exists.
    # Errors if directory exists but is not a git repository (bare repos not supported).
    - action: git.clone
      repo_url: https://github.com/brujack/dotfiles
      directory: "{{ user.home_dir }}/git-repos/personal/dotfiles"
      update_existing: true
```

- [ ] **Step 2: Update README.md action catalog entry for git.clone**

Find the `git.clone` row in the action catalog table in `README.md`. Update the Description column to note `update_existing`:

The description should read something like:

> Clone a git repository. Skips if directory exists. Set `update_existing: true` to pull instead of skip when directory exists.

- [ ] **Step 3: Commit**

```bash
git add examples/git/clone.yaml README.md
git commit -m "docs(git.clone): document update_existing field in example and catalog"
```

---

### Task 5: Open PR and monitor CI

- [ ] **Step 1: Push branch**

```bash
git push -u origin <branch-name>
```

- [ ] **Step 2: Open PR**

```bash
gh pr create --repo brujack/etch-cli \
  --title "feat(git.clone): add update_existing field for clone-or-pull" \
  --body "$(cat <<'EOF'
## Summary
- Adds `update_existing: bool` (default `false`) to `git.clone`
- When `update_existing: true`: clones if directory is missing, runs `git pull` if it exists as a git repo
- Errors at plan time if directory exists but has no `.git` entry (not a git repo)
- Eliminates `command.run` workarounds for clone-or-pull in dotfiles manifests
- No change to default behavior (`update_existing: false`)

## Test plan
- [x] Atom `plan()` tests (4): skip-false, run-true-with-git, error-no-git, run-missing-dir
- [x] Atom `execute()` tests (2): pulls when dir exists, propagates pull failure
- [x] Action deserialization tests (2): explicit true, default false
- [x] All existing tests pass
- [x] `make test` green

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 3: Monitor CI**

```bash
gh pr checks <number> --repo brujack/etch-cli --watch
```

Expected: all required jobs green; auto-merge fires.

- [ ] **Step 4: Post-merge cleanup**

```bash
git fetch --prune && git reset --hard origin/main
git branch -D <branch-name>
```

> **Do this directly on main after the PR merges — not inside the worktree.**

Update `docs/superpowers/README.md` — change the `git-clone-update-existing` row status from `Pending` to `Done` and add the plan link:

```markdown
| 2026-06-10 | [git-clone-update-existing](plans/2026-06-10-git-clone-update-existing-plan.md) | [git-clone-update-existing](specs/2026-06-10-git-clone-update-existing-design.md) | Done |
```

Add `> **Status: DONE**` banner at the top of this plan file.

Commit and push.
