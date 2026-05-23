> **Status: DONE**

# Error-Path Integration Tests Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 4 integration tests in `app/tests/error_paths.rs` verifying that `etch apply` exits non-zero on missing source files, malformed YAML, unknown action type, and permission-denied targets.

**Architecture:** Tests assert existing CLI behavior — no implementation code is written. Each test creates an isolated tempdir, writes a broken manifest, runs `etch apply`, and asserts `.failure()`. Pattern matches `integration.rs` (local `apply()` helper + `tempfile::tempdir()`).

**Tech Stack:** Rust, `assert_cmd`, `tempfile`, `cargo nextest`

---

## Files

- Create: `app/tests/error_paths.rs`

No other files modified.

---

### Task 1: Worktree setup

- [ ] **Create worktree on a feature branch**

```bash
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"
git -C ~/git-repos/personal/etch-cli worktree add .worktrees/feat/error-path-tests -b feat/error-path-tests
```

All subsequent work happens in `/Users/bruce/git-repos/personal/etch-cli/.worktrees/feat/error-path-tests/`.

---

### Task 2: Test 1 — missing source file exits non-zero

**Files:** Create `app/tests/error_paths.rs`

- [ ] **Create the file with preamble and first test**

Write `/Users/bruce/git-repos/personal/etch-cli/.worktrees/feat/error-path-tests/app/tests/error_paths.rs`:

```rust
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

fn apply(dir: &std::path::Path) -> assert_cmd::assert::Assert {
    Command::new(assert_cmd::cargo::cargo_bin!("etch"))
        .current_dir(dir)
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
}

#[test]
fn apply_with_missing_source_file_exits_nonzero() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("should_not_exist.txt");

    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: file.link\n    source: nonexistent.txt\n    target: {}\n",
            target.display()
        ),
    )
    .unwrap();

    apply(dir.path()).failure();
}
```

- [ ] **Run test to verify it passes (existing behavior)**

```bash
cd /Users/bruce/git-repos/personal/etch-cli/.worktrees/feat/error-path-tests
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"
cargo nextest run -E 'test(apply_with_missing_source_file_exits_nonzero)' -p etch-cli 2>&1
```

Expected: `PASS`. If `FAIL` (etch returns exit 0 on missing source), investigate — that is a bug in the implementation, not in the test.

- [ ] **Commit**

```bash
cd /Users/bruce/git-repos/personal/etch-cli/.worktrees/feat/error-path-tests
git add app/tests/error_paths.rs
git commit -m "test(integration): missing source file exits nonzero"
```

---

### Task 3: Test 2 — malformed YAML exits non-zero

**Files:** Modify `app/tests/error_paths.rs`

- [ ] **Add the malformed YAML test**

Append to `app/tests/error_paths.rs`:

```rust
#[test]
fn apply_with_malformed_yaml_exits_nonzero() {
    let dir = tempdir().unwrap();

    fs::write(dir.path().join("test.yaml"), "actions: [unclosed").unwrap();

    apply(dir.path()).failure();
}
```

- [ ] **Run test to verify it passes**

```bash
cd /Users/bruce/git-repos/personal/etch-cli/.worktrees/feat/error-path-tests
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"
cargo nextest run -E 'test(apply_with_malformed_yaml_exits_nonzero)' -p etch-cli 2>&1
```

Expected: `PASS`.

- [ ] **Commit**

```bash
git add app/tests/error_paths.rs
git commit -m "test(integration): malformed YAML exits nonzero"
```

---

### Task 4: Test 3 — unknown action type exits non-zero

**Files:** Modify `app/tests/error_paths.rs`

- [ ] **Add the unknown action test**

Append to `app/tests/error_paths.rs`:

```rust
#[test]
fn apply_with_unknown_action_exits_nonzero() {
    let dir = tempdir().unwrap();

    fs::write(
        dir.path().join("test.yaml"),
        "actions:\n  - action: does.not.exist\n    some: value\n",
    )
    .unwrap();

    apply(dir.path()).failure();
}
```

- [ ] **Run test to verify it passes**

```bash
cd /Users/bruce/git-repos/personal/etch-cli/.worktrees/feat/error-path-tests
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"
cargo nextest run -E 'test(apply_with_unknown_action_exits_nonzero)' -p etch-cli 2>&1
```

Expected: `PASS`.

- [ ] **Commit**

```bash
git add app/tests/error_paths.rs
git commit -m "test(integration): unknown action type exits nonzero"
```

---

### Task 5: Test 4 — permission-denied target exits non-zero

**Files:** Modify `app/tests/error_paths.rs`

- [ ] **Add the permission-denied test**

Append to `app/tests/error_paths.rs`:

```rust
#[test]
#[cfg(unix)]
fn apply_to_unwritable_target_dir_exits_nonzero() {
    use std::os::unix::fs::PermissionsExt;

    // Skip if running as root — root bypasses permission enforcement
    let uid_out = std::process::Command::new("id")
        .arg("-u")
        .output()
        .unwrap();
    if String::from_utf8_lossy(&uid_out.stdout).trim() == "0" {
        return;
    }

    let dir = tempdir().unwrap();
    let files_dir = dir.path().join("files");
    fs::create_dir_all(&files_dir).unwrap();
    fs::write(files_dir.join("source.txt"), "content").unwrap();

    let target_dir = dir.path().join("locked");
    fs::create_dir_all(&target_dir).unwrap();
    fs::set_permissions(&target_dir, fs::Permissions::from_mode(0o000)).unwrap();

    let target_path = target_dir.join("dest.txt");
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: file.copy\n    from: source.txt\n    to: {}\n",
            target_path.display()
        ),
    )
    .unwrap();

    let result = apply(dir.path());

    // Restore perms so tempdir cleanup succeeds
    fs::set_permissions(&target_dir, fs::Permissions::from_mode(0o755)).unwrap();

    result.failure();
}
```

- [ ] **Run test to verify it passes**

```bash
cd /Users/bruce/git-repos/personal/etch-cli/.worktrees/feat/error-path-tests
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"
cargo nextest run -E 'test(apply_to_unwritable_target_dir_exits_nonzero)' -p etch-cli 2>&1
```

Expected: `PASS`. If the test is skipped (running as root on CI), that is correct behavior.

- [ ] **Commit**

```bash
git add app/tests/error_paths.rs
git commit -m "test(integration): permission-denied target exits nonzero"
```

---

### Task 6: Full test suite, push, PR, CI

- [ ] **Run full test suite to confirm no regressions**

```bash
cd /Users/bruce/git-repos/personal/etch-cli/.worktrees/feat/error-path-tests
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"
make test 2>&1 | tail -20
```

Expected: all tests pass including the 4 new ones.

- [ ] **Push from main repo (avoids GIT_DIR leakage)**

```bash
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"
git -C ~/git-repos/personal/etch-cli push origin feat/error-path-tests
```

- [ ] **Open PR**

```bash
cd /Users/bruce/git-repos/personal/etch-cli/.worktrees/feat/error-path-tests
gh pr create -R brujack/etch-cli --head feat/error-path-tests --base main \
  --title "test(integration): add error-path tests for missing source, invalid YAML, permission denied" \
  --body "Adds 4 integration tests in \`app/tests/error_paths.rs\` covering error paths that were deferred from the initial integration test scope:

- Missing source file → non-zero exit
- Malformed YAML → non-zero exit
- Unknown action type → non-zero exit
- Permission-denied target → non-zero exit (skipped when running as root)"
```

- [ ] **Watch CI to completion**

```bash
gh pr checks <PR_NUMBER> --watch -R brujack/etch-cli
```

---

### Task 7: Post-merge cleanup and docs update

**Do this directly on main after the PR merges — not inside the worktree.**

- [ ] **Remove worktree and clean up branches**

```bash
git -C ~/git-repos/personal/etch-cli worktree remove .worktrees/feat/error-path-tests
git -C ~/git-repos/personal/etch-cli branch -D feat/error-path-tests
git -C ~/git-repos/personal/etch-cli push origin --delete feat/error-path-tests
git -C ~/git-repos/personal/etch-cli fetch --prune && git -C ~/git-repos/personal/etch-cli pull
```

- [ ] **Update etch-cli superpowers README on main**

In `~/git-repos/personal/etch-cli/docs/superpowers/README.md`, add a row to the All Plans table:

```markdown
| 2026-05-23 | [error-path-integration-tests](plans/2026-05-23-error-path-integration-tests.md) | [spec](specs/2026-05-23-error-path-integration-tests-design.md) | Done |
```

Add `> **Status: DONE**` banner at the top of `docs/superpowers/plans/2026-05-23-error-path-integration-tests.md`.

- [ ] **Update ai-config backlog**

In `~/git-repos/personal/ai-config/docs/superpowers/README.md`, remove the `etch-cli error-path integration tests` row from the Backlog table.

- [ ] **Commit and push both repos**

```bash
cd ~/git-repos/personal/etch-cli
git add docs/superpowers/README.md docs/superpowers/plans/2026-05-23-error-path-integration-tests.md
git commit -m "chore(docs): mark error-path integration tests plan done"
git push

cd ~/git-repos/personal/ai-config
git add docs/superpowers/README.md
git commit -m "chore(docs): remove etch-cli error-path tests from backlog — done"
git push
```
