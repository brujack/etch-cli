# Integration Tests — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 8 integration tests in `app/tests/integration.rs` that run the real `etch` binary against inline YAML manifests in tempdirs, covering file.link, file.copy, command.run, and directory.create (happy path + idempotency each).

**Architecture:** Each test creates a `tempfile::tempdir()`, writes a YAML manifest and any required source files into it, spawns the `etch` binary with `current_dir` set to the tempdir and `-d . apply`, then asserts filesystem state. Follows the pattern established in `app/tests/basic_usage.rs` using `Command::new(assert_cmd::cargo::cargo_bin!("etch"))`.

**Tech Stack:** `assert_cmd`, `tempfile`, `std::fs` — all already in `app/Cargo.toml` dev-dependencies.

---

## Files

- **Create:** `app/tests/integration.rs`
- **Modify:** `docs/superpowers/README.md` — **post-merge on main only**

---

## Task 1: file.link tests

**Files:**

- Create: `app/tests/integration.rs`

These are the first 2 tests. The file doesn't exist yet — `cargo nextest run` will fail to compile until the file is created.

- [ ] **Step 1: Create `app/tests/integration.rs` with file.link tests**

```rust
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

/// Spawn `etch --no-color -d . apply` from `dir`.
fn apply(dir: &std::path::Path) -> assert_cmd::assert::Assert {
    Command::new(assert_cmd::cargo::cargo_bin!("etch"))
        .current_dir(dir)
        .args(["--no-color", "-d", ".", "apply"])
        .assert()
}

// ─── file.link ────────────────────────────────────────────────────────────────

#[test]
fn file_link_creates_symlink() {
    let dir = tempdir().unwrap();

    // Source file that will be linked
    fs::write(dir.path().join("dotfile.txt"), "hello from etch").unwrap();

    // Manifest: link dotfile.txt → linked.txt (relative paths, cwd = dir)
    fs::write(
        dir.path().join("test.yaml"),
        "actions:\n  - action: file.link\n    source: dotfile.txt\n    target: linked.txt\n",
    )
    .unwrap();

    apply(dir.path()).success();

    let target = dir.path().join("linked.txt");
    assert!(target.is_symlink(), "linked.txt should be a symlink");
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "hello from etch",
        "symlink should resolve to dotfile.txt content"
    );
}

#[test]
fn file_link_is_idempotent() {
    let dir = tempdir().unwrap();

    fs::write(dir.path().join("dotfile.txt"), "hello").unwrap();
    fs::write(
        dir.path().join("test.yaml"),
        "actions:\n  - action: file.link\n    source: dotfile.txt\n    target: linked.txt\n",
    )
    .unwrap();

    apply(dir.path()).success();
    apply(dir.path()).success(); // second apply must also succeed

    let target = dir.path().join("linked.txt");
    assert!(target.is_symlink(), "symlink should still exist after second apply");
    assert_eq!(fs::read_to_string(&target).unwrap(), "hello");
}
```

- [ ] **Step 2: Confirm tests compile and pass**

```bash
cargo nextest run --manifest-path app/Cargo.toml --test integration 2>&1 | tail -10
```

Expected:

```
test file_link_creates_symlink ... ok
test file_link_is_idempotent ... ok
Summary: 2 tests run, 2 passed
```

If either test fails, inspect the error. Common cause: etch may use `source`/`target` field names for `file.link` — the spec confirmed these are correct but verify against `lib/src/actions/file/link.rs` if needed.

- [ ] **Step 3: Commit**

```bash
git add app/tests/integration.rs
git commit -m "test(integration): add file.link happy path and idempotency tests

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 2: file.copy tests

**Files:**

- Modify: `app/tests/integration.rs`

- [ ] **Step 1: Append file.copy tests to `app/tests/integration.rs`**

Add after the file.link tests:

```rust
// ─── file.copy ────────────────────────────────────────────────────────────────

#[test]
fn file_copy_copies_content() {
    let dir = tempdir().unwrap();

    fs::write(dir.path().join("source.txt"), "copy me").unwrap();
    fs::write(
        dir.path().join("test.yaml"),
        "actions:\n  - action: file.copy\n    from: source.txt\n    to: dest.txt\n",
    )
    .unwrap();

    apply(dir.path()).success();

    let dest = dir.path().join("dest.txt");
    assert!(dest.exists(), "dest.txt should exist after copy");
    assert_eq!(
        fs::read_to_string(&dest).unwrap(),
        "copy me",
        "dest.txt content should match source"
    );
}

#[test]
fn file_copy_is_idempotent() {
    let dir = tempdir().unwrap();

    fs::write(dir.path().join("source.txt"), "content").unwrap();
    fs::write(
        dir.path().join("test.yaml"),
        "actions:\n  - action: file.copy\n    from: source.txt\n    to: dest.txt\n",
    )
    .unwrap();

    apply(dir.path()).success();
    apply(dir.path()).success();

    let dest = dir.path().join("dest.txt");
    assert!(dest.exists());
    assert_eq!(fs::read_to_string(&dest).unwrap(), "content");
}
```

- [ ] **Step 2: Run integration tests — confirm 4 pass**

```bash
cargo nextest run --manifest-path app/Cargo.toml --test integration 2>&1 | tail -10
```

Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add app/tests/integration.rs
git commit -m "test(integration): add file.copy happy path and idempotency tests

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 3: command.run tests

**Files:**

- Modify: `app/tests/integration.rs`

- [ ] **Step 1: Append command.run tests**

```rust
// ─── command.run ──────────────────────────────────────────────────────────────

#[test]
fn command_run_creates_file() {
    let dir = tempdir().unwrap();

    let output = dir.path().join("created.txt");
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: command.run\n    command: touch\n    args:\n      - {}\n",
            output.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();

    assert!(output.exists(), "touch should have created the file");
}

#[test]
fn command_run_is_idempotent() {
    let dir = tempdir().unwrap();

    let output = dir.path().join("created.txt");
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: command.run\n    command: touch\n    args:\n      - {}\n",
            output.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();
    apply(dir.path()).success(); // touch on existing file is a no-op

    assert!(output.exists());
}
```

- [ ] **Step 2: Run — confirm 6 pass**

```bash
cargo nextest run --manifest-path app/Cargo.toml --test integration 2>&1 | tail -10
```

Expected: 6 tests pass.

- [ ] **Step 3: Commit**

```bash
git add app/tests/integration.rs
git commit -m "test(integration): add command.run happy path and idempotency tests

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 4: directory.create tests

**Files:**

- Modify: `app/tests/integration.rs`

- [ ] **Step 1: Append directory.create tests**

```rust
// ─── directory.create ─────────────────────────────────────────────────────────

#[test]
fn directory_create_makes_dir() {
    let dir = tempdir().unwrap();

    let new_dir = dir.path().join("myconfig");
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: directory.create\n    path: {}\n",
            new_dir.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();

    assert!(new_dir.is_dir(), "myconfig directory should have been created");
}

#[test]
fn directory_create_is_idempotent() {
    let dir = tempdir().unwrap();

    let new_dir = dir.path().join("myconfig");
    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: directory.create\n    path: {}\n",
            new_dir.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();
    apply(dir.path()).success();

    assert!(new_dir.is_dir(), "directory should still exist after second apply");
}
```

- [ ] **Step 2: Run — confirm all 8 pass**

```bash
cargo nextest run --manifest-path app/Cargo.toml --test integration 2>&1 | tail -12
```

Expected:

```
test command_run_creates_file ... ok
test command_run_is_idempotent ... ok
test directory_create_is_idempotent ... ok
test directory_create_makes_dir ... ok
test file_copy_copies_content ... ok
test file_copy_is_idempotent ... ok
test file_link_creates_symlink ... ok
test file_link_is_idempotent ... ok
Summary: 8 tests run, 8 passed
```

- [ ] **Step 3: Run full suite to confirm no regressions**

```bash
cargo nextest run --manifest-path app/Cargo.toml 2>&1 | tail -5
```

Expected: all existing tests still pass alongside the 8 new ones.

- [ ] **Step 4: Commit**

```bash
git add app/tests/integration.rs
git commit -m "test(integration): add directory.create happy path and idempotency tests

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 5: Post-merge docs update

> **Do this directly on main after the PR merges — not inside the worktree.**

- [ ] **Step 1: Update plan index**

In `docs/superpowers/README.md`, update the integration-tests row: add plan link, set status to Done.

- [ ] **Step 2: Add Done banner**

Add `> **Status: DONE**` at the top of `docs/superpowers/plans/2026-05-20-integration-tests.md`.

- [ ] **Step 3: Commit on main**

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-05-20-integration-tests.md
git commit -m "docs: mark integration-tests plan done

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
git push
```
