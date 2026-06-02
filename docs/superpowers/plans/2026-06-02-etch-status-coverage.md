> **Status: DONE**

# etch status integration tests Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 7 integration tests for `etch status` that bring `app/src/commands/status.rs` from 0% to ~85% coverage.

**Architecture:** New file `app/tests/status.rs`. Each test spawns the `etch` binary via `assert_cmd::Command::cargo_bin("etch")`, creates a temp directory with a manifest YAML and the necessary filesystem state (symlinks, files), and asserts on exit code and stdout. No production code changes — tests only.

**Tech Stack:** Rust, `assert_cmd 2.1`, `predicates 3.1`, `tempfile 3.26`, `serde_json 1.0`, `std::os::unix::fs::symlink`

---

## Files

- Create: `app/tests/status.rs` — 7 integration tests for `etch status`

No `Cargo.toml` changes needed — all deps already present.
No `[[test]]` registration needed — `app/tests/` files are auto-discovered.

---

## Background: how `file.link` resolves paths

The `file.link` action resolves `source: source.txt` to `<manifest_root_dir>/files/source.txt` — that is, the `files/` subdirectory inside the manifest directory. The `target:` is used as-is (must be an absolute path).

The `file.link` atom's `status()` calls `std::fs::read_link(target)` and compares the result to `source` (absolute path). So:

- **Ok**: symlink at `target` points to `<manifest_dir>/files/source.txt`
- **Missing**: no symlink exists at `target`
- **Drifted**: symlink at `target` points to any other path

---

## Directory structure used in every file.link test

```
<root_tmpdir>/
  directory/
    mymanifest/
      main.yaml           ← written with format!() to embed target path
      files/
        source.txt        ← the link source file
<target_tmpdir>/
  target_link             ← where the symlink lives (created by test, not etch)
```

CLI invocation:

```
etch --no-color -d <root_tmpdir>/directory status -m mymanifest
```

---

### Task 1: Create `app/tests/status.rs` — ok and missing tests

**Files:**

- Create: `app/tests/status.rs`

- [ ] **Step 1: Write the ok and missing tests**

Create `app/tests/status.rs` with this full content:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use std::path::PathBuf;
use tempfile::TempDir;

fn etch() -> Command {
    Command::cargo_bin("etch").unwrap()
}

/// Creates the directory structure for a file.link manifest test.
/// Returns (root_tmpdir, manifest_dir, source_file_path).
/// root_tmpdir must stay in scope for the duration of the test.
fn setup_link_manifest(root: &PathBuf, manifest_name: &str, target_path: &PathBuf) -> PathBuf {
    let manifest_dir = root.join("directory").join(manifest_name);
    let files_dir = manifest_dir.join("files");
    std::fs::create_dir_all(&files_dir).unwrap();

    let source_file = files_dir.join("source.txt");
    std::fs::write(&source_file, "link source content").unwrap();

    let yaml = format!(
        "actions:\n  - action: file.link\n    source: source.txt\n    target: {}\n",
        target_path.display()
    );
    std::fs::write(manifest_dir.join("main.yaml"), yaml).unwrap();

    source_file
}

#[test]
fn status_exits_zero_when_all_atoms_ok() {
    let root_tmp = TempDir::new().unwrap();
    let target_tmp = TempDir::new().unwrap();
    let root = root_tmp.path().to_path_buf();
    let target_link = target_tmp.path().join("target_link");

    let source_file = setup_link_manifest(&root, "mymanifest", &target_link);

    // Create the correct symlink — status should be Ok
    std::os::unix::fs::symlink(&source_file, &target_link).unwrap();

    etch()
        .current_dir(&root)
        .args(["--no-color", "-d", "./directory", "status", "-m", "mymanifest"])
        .assert()
        .success()
        .stdout(contains("ok"));
}

#[test]
fn status_exits_nonzero_when_atom_missing() {
    let root_tmp = TempDir::new().unwrap();
    let target_tmp = TempDir::new().unwrap();
    let root = root_tmp.path().to_path_buf();
    let target_link = target_tmp.path().join("target_link");

    setup_link_manifest(&root, "mymanifest", &target_link);
    // No symlink created — status should be Missing

    etch()
        .current_dir(&root)
        .args(["--no-color", "-d", "./directory", "status", "-m", "mymanifest"])
        .assert()
        .failure()
        .stdout(contains("missing"));
}
```

- [ ] **Step 2: Run the tests**

```bash
cargo nextest run -p etch-cli --test status 2>&1 | tail -20
```

Expected: `2 tests run, 2 passed`.

- [ ] **Step 3: Commit**

```bash
git add app/tests/status.rs
git commit -m "test(status): add ok and missing integration tests"
```

---

### Task 2: Add the drifted test

**Files:**

- Modify: `app/tests/status.rs`

- [ ] **Step 1: Add drifted test**

Append to `app/tests/status.rs`:

```rust
#[test]
fn status_exits_nonzero_when_atom_drifted() {
    let root_tmp = TempDir::new().unwrap();
    let target_tmp = TempDir::new().unwrap();
    let root = root_tmp.path().to_path_buf();
    let target_link = target_tmp.path().join("target_link");

    setup_link_manifest(&root, "mymanifest", &target_link);

    // Create a symlink pointing to a different file — status should be Drifted
    let other_file = target_tmp.path().join("other.txt");
    std::fs::write(&other_file, "other content").unwrap();
    std::os::unix::fs::symlink(&other_file, &target_link).unwrap();

    etch()
        .current_dir(&root)
        .args(["--no-color", "-d", "./directory", "status", "-m", "mymanifest"])
        .assert()
        .failure()
        .stdout(contains("drifted"));
}
```

- [ ] **Step 2: Run the tests**

```bash
cargo nextest run -p etch-cli --test status 2>&1 | tail -20
```

Expected: `3 tests run, 3 passed`.

- [ ] **Step 3: Commit**

```bash
git add app/tests/status.rs
git commit -m "test(status): add drifted integration test"
```

---

### Task 3: Add unchecked and where-false tests

**Files:**

- Modify: `app/tests/status.rs`

- [ ] **Step 1: Add unchecked and where-false tests**

Append to `app/tests/status.rs`:

```rust
#[test]
fn status_unchecked_atom_exits_zero() {
    let root_tmp = TempDir::new().unwrap();
    let root = root_tmp.path().to_path_buf();
    let manifest_dir = root.join("directory/mymanifest");
    std::fs::create_dir_all(&manifest_dir).unwrap();

    // command.run atoms return Unchecked — always exit 0
    std::fs::write(
        manifest_dir.join("main.yaml"),
        "actions:\n  - action: command.run\n    command: echo\n    args:\n      - hello\n",
    )
    .unwrap();

    etch()
        .current_dir(&root)
        .args(["--no-color", "-d", "./directory", "status", "-m", "mymanifest"])
        .assert()
        .success()
        .stdout(contains("unchecked"));
}

#[test]
fn status_where_false_skips_manifest() {
    let root_tmp = TempDir::new().unwrap();
    let target_tmp = TempDir::new().unwrap();
    let root = root_tmp.path().to_path_buf();
    let target_link = target_tmp.path().join("target_link");

    let manifest_dir = root.join("directory/mymanifest");
    let files_dir = manifest_dir.join("files");
    std::fs::create_dir_all(&files_dir).unwrap();
    std::fs::write(files_dir.join("source.txt"), "content").unwrap();

    // where: 'false' — manifest is skipped entirely, even though symlink is missing
    let yaml = format!(
        "where: 'false'\nactions:\n  - action: file.link\n    source: source.txt\n    target: {}\n",
        target_link.display()
    );
    std::fs::write(manifest_dir.join("main.yaml"), yaml).unwrap();

    etch()
        .current_dir(&root)
        .args(["--no-color", "-d", "./directory", "status", "-m", "mymanifest"])
        .assert()
        .success()
        .stdout(predicates::str::contains("mymanifest").not());
}
```

- [ ] **Step 2: Add `not()` import at the top of the file**

The `where_false` test uses `.not()` on a predicate. Add this import alongside the existing ones at the top of `app/tests/status.rs`:

```rust
use predicates::prelude::PredicateBooleanExt;
```

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p etch-cli --test status 2>&1 | tail -20
```

Expected: `5 tests run, 5 passed`.

- [ ] **Step 4: Commit**

```bash
git add app/tests/status.rs
git commit -m "test(status): add unchecked and where-false tests"
```

---

### Task 4: Add JSON output tests

**Files:**

- Modify: `app/tests/status.rs`

- [ ] **Step 1: Add JSON tests**

Append to `app/tests/status.rs`:

```rust
#[test]
fn status_json_flag_produces_valid_json() {
    let root_tmp = TempDir::new().unwrap();
    let target_tmp = TempDir::new().unwrap();
    let root = root_tmp.path().to_path_buf();
    let target_link = target_tmp.path().join("target_link");

    let source_file = setup_link_manifest(&root, "mymanifest", &target_link);
    std::os::unix::fs::symlink(&source_file, &target_link).unwrap();

    let output = etch()
        .current_dir(&root)
        .args(["--no-color", "-d", "./directory", "status", "--json", "-m", "mymanifest"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text)
        .expect("--json output must be valid JSON");

    assert!(parsed["manifests"].is_array(), "must have manifests array");
    assert!(parsed["summary"].is_object(), "must have summary object");
    assert_eq!(parsed["summary"]["ok"], 1);
    assert_eq!(parsed["summary"]["missing"], 0);
    assert_eq!(parsed["summary"]["drifted"], 0);
}

#[test]
fn status_json_missing_has_nonzero_exit_and_summary() {
    let root_tmp = TempDir::new().unwrap();
    let target_tmp = TempDir::new().unwrap();
    let root = root_tmp.path().to_path_buf();
    let target_link = target_tmp.path().join("target_link");

    setup_link_manifest(&root, "mymanifest", &target_link);
    // No symlink — atom is Missing

    let output = etch()
        .current_dir(&root)
        .args(["--no-color", "-d", "./directory", "status", "--json", "-m", "mymanifest"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text)
        .expect("--json output must be valid JSON even on failure");

    assert_eq!(parsed["summary"]["missing"], 1);
    assert_eq!(parsed["summary"]["ok"], 0);
}
```

- [ ] **Step 2: Run the full test suite**

```bash
cargo nextest run -p etch-cli --test status 2>&1 | tail -20
```

Expected: `7 tests run, 7 passed`.

- [ ] **Step 3: Run the full make test to check for regressions**

```bash
make test 2>&1 | tail -30
```

Expected: all tests pass, no regressions.

- [ ] **Step 4: Commit**

```bash
git add app/tests/status.rs
git commit -m "test(status): add JSON output tests"
```

---

### Task 5: Update docs (post-merge on main — NOT inside the worktree)

**Do this directly on main after the PR merges — not inside the worktree.**

- [ ] **Step 1: Add row to `docs/superpowers/README.md`**

Add to the All Plans table:

```
| 2026-06-02 | [etch-status-coverage](plans/2026-06-02-etch-status-coverage.md) | [etch-status-coverage](specs/2026-06-02-etch-status-coverage-design.md) | Done |
```

- [ ] **Step 2: Add `> **Status: DONE**` banner to plan file**

At the top of `docs/superpowers/plans/2026-06-02-etch-status-coverage.md`, add:

```
> **Status: DONE**
```

- [ ] **Step 3: Commit directly to main**

```bash
git add docs/superpowers/README.md docs/superpowers/plans/2026-06-02-etch-status-coverage.md
git commit -m "docs(superpowers): mark etch-status-coverage Done"
```
