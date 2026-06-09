> **Status: DONE** — merged via PR #102 (2026-06-09)

# pyenv.virtualenv `recreate:` field — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `recreate: bool` to `PyenvVirtualenv` so `etch apply` deletes and recreates the virtualenv when the installed Python version differs from the manifest pin.

**Architecture:** All changes are in `lib/src/actions/pyenv/virtualenv.rs`. A new `installed_python_version(dir, name)` helper reads the pyenv symlink at `~/.pyenv/versions/{name}` to determine the current Python version without spawning a subprocess. When `recreate: true` and a version mismatch is detected, `plan()` emits `pyenv uninstall -f {name}` followed by `pyenv virtualenv {python_version} {name}`. Default `recreate: false` preserves existing behavior exactly.

**Tech Stack:** Rust, `std::fs::read_link`, `std::path::PathBuf`, `shellexpand::tilde` (already in lib Cargo.toml), `tempfile` + `serial_test` for tests.

**Rust `-D warnings` note:** `installed_python_version` is a private helper only used from `plan()`. Writing it in Task 1 without wiring it into `plan()` will produce a `dead_code` compiler error that blocks commits. Tasks 1 and 2 must be batched into a single commit after both are green.

---

### Task 1: `installed_python_version` helper (TDD — no standalone commit)

**Files:**

- Modify: `lib/src/actions/pyenv/virtualenv.rs`

- [ ] **Step 1: Add imports at top of file**

Add to the existing `use` block at the top of `lib/src/actions/pyenv/virtualenv.rs`:

```rust
use std::path::{Path, PathBuf};
```

- [ ] **Step 2: Write failing tests for `installed_python_version`**

Add to the `#[cfg(test)]` module at the bottom of `lib/src/actions/pyenv/virtualenv.rs`:

```rust
#[test]
fn installed_python_version_returns_none_when_not_a_symlink() {
    let tmp = tempfile::tempdir().unwrap();
    let versions_dir = tmp.path().join("versions");
    std::fs::create_dir_all(versions_dir.join("ansible")).unwrap(); // regular dir, not symlink
    let result = installed_python_version(&versions_dir, "ansible");
    assert!(result.is_none(), "expected None for non-symlink path");
}

#[test]
fn installed_python_version_returns_none_when_venv_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let versions_dir = tmp.path().join("versions");
    std::fs::create_dir_all(&versions_dir).unwrap();
    let result = installed_python_version(&versions_dir, "ansible");
    assert!(result.is_none(), "expected None when venv path does not exist");
}

#[test]
fn installed_python_version_returns_version_from_relative_symlink() {
    use std::os::unix::fs::symlink;
    let tmp = tempfile::tempdir().unwrap();
    let versions_dir = tmp.path().join("versions");
    // Create target directory: versions/3.14.5/envs/ansible
    let target = versions_dir.join("3.14.5").join("envs").join("ansible");
    std::fs::create_dir_all(&target).unwrap();
    // Create relative symlink: versions/ansible -> 3.14.5/envs/ansible
    symlink(Path::new("3.14.5/envs/ansible"), versions_dir.join("ansible")).unwrap();
    let result = installed_python_version(&versions_dir, "ansible");
    assert_eq!(result, Some("3.14.5".to_string()));
}

#[test]
fn installed_python_version_returns_version_from_absolute_symlink() {
    use std::os::unix::fs::symlink;
    let tmp = tempfile::tempdir().unwrap();
    let versions_dir = tmp.path().join("versions");
    let target = versions_dir.join("3.12.0").join("envs").join("myproject");
    std::fs::create_dir_all(&target).unwrap();
    // Create absolute symlink
    symlink(&target, versions_dir.join("myproject")).unwrap();
    let result = installed_python_version(&versions_dir, "myproject");
    assert_eq!(result, Some("3.12.0".to_string()));
}
```

- [ ] **Step 3: Confirm compile error (RED)**

```bash
cargo test -p etch-lib 'pyenv::virtualenv::tests::installed_python_version' 2>&1 | grep "^error" | head -5
```

Expected: `error[E0425]: cannot find function 'installed_python_version' in this scope`

- [ ] **Step 4: Implement `installed_python_version`**

Add this free function above the `PyenvVirtualenv` struct definition:

```rust
/// Resolve the pyenv symlink at `{versions_dir}/{name}` and return the Python
/// version embedded in its target path (the component immediately before `envs/`).
/// Returns None when the path is absent, not a symlink, or has an unexpected layout.
fn installed_python_version(versions_dir: &Path, name: &str) -> Option<String> {
    let target = std::fs::read_link(versions_dir.join(name)).ok()?;
    target
        .components()
        .zip(target.components().skip(1))
        .find_map(|(a, b)| {
            if b.as_os_str() == "envs" {
                Some(a.as_os_str().to_string_lossy().into_owned())
            } else {
                None
            }
        })
}
```

- [ ] **Step 5: Run tests (GREEN)**

```bash
cargo test -p etch-lib 'pyenv::virtualenv::tests::installed_python_version' 2>&1 | tail -8
```

Expected: 4 tests pass.

**Do NOT commit yet** — `installed_python_version` is unused in production code and will produce a `dead_code` error under `-D warnings`. Continue to Task 2 and commit both together.

---

### Task 2: `recreate:` field + `plan()` wiring (TDD — commit both Task 1 + Task 2)

**Files:**

- Modify: `lib/src/actions/pyenv/virtualenv.rs`

- [ ] **Step 1: Write failing tests for `plan()` with `recreate: true`**

Add to the `#[cfg(test)]` module. These use `#[serial]` because they manipulate `PATH` and `HOME`.

```rust
#[test]
#[serial]
fn plan_recreate_true_creates_when_no_venv() {
    // No fake pyenv needed — virtualenv_exists returns false by default
    // (pyenv not in PATH → returns false gracefully)
    let old_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", "/nonexistent");

    let action = PyenvVirtualenv {
        python_version: Some(String::from("3.14.5")),
        name: Some(String::from("ansible")),
        recreate: true,
    };
    let steps = action
        .plan(&Manifest::default(), &Contexts::default())
        .unwrap();

    std::env::set_var("PATH", old_path);

    assert_eq!(1, steps.len(), "expected 1 create step when venv absent");
    let display = steps[0].atom.to_string();
    assert!(display.contains("virtualenv"), "expected 'virtualenv' in: {display}");
    assert!(display.contains("3.14.5"), "expected version in: {display}");
    assert!(display.contains("ansible"), "expected name in: {display}");
}

#[test]
#[serial]
fn plan_recreate_false_skips_existing_venv_unchanged() {
    // Confirm recreate: false still skips (no regression)
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let fake_pyenv = tmp.path().join("pyenv");
    std::fs::write(
        &fake_pyenv,
        "#!/bin/sh\nif [ \"$1\" = \"virtualenvs\" ]; then printf 'ansible\\n'; fi\n",
    )
    .unwrap();
    std::fs::set_permissions(&fake_pyenv, std::fs::Permissions::from_mode(0o755)).unwrap();
    let old_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

    let action = PyenvVirtualenv {
        python_version: Some(String::from("3.14.5")),
        name: Some(String::from("ansible")),
        recreate: false,
    };
    let steps = action
        .plan(&Manifest::default(), &Contexts::default())
        .unwrap();
    std::env::set_var("PATH", old_path);

    assert!(steps.is_empty(), "expected no steps when recreate:false and venv exists");
}

#[test]
#[serial]
fn plan_recreate_true_skips_when_version_matches() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let tmp = tempfile::tempdir().unwrap();

    // Fake pyenv: reports ansible as existing
    let fake_pyenv = tmp.path().join("pyenv");
    std::fs::write(
        &fake_pyenv,
        "#!/bin/sh\nif [ \"$1\" = \"virtualenvs\" ]; then printf 'ansible\\n'; fi\n",
    )
    .unwrap();
    std::fs::set_permissions(&fake_pyenv, std::fs::Permissions::from_mode(0o755)).unwrap();

    // Set HOME to tmp so shellexpand resolves ~/.pyenv/versions there
    let old_home = std::env::var("HOME").unwrap_or_default();
    std::env::set_var("HOME", tmp.path());

    // Create symlink: {tmp}/.pyenv/versions/ansible -> 3.14.5/envs/ansible
    let versions_dir = tmp.path().join(".pyenv").join("versions");
    let target_dir = versions_dir.join("3.14.5").join("envs").join("ansible");
    std::fs::create_dir_all(&target_dir).unwrap();
    symlink(Path::new("3.14.5/envs/ansible"), versions_dir.join("ansible")).unwrap();

    let old_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

    let action = PyenvVirtualenv {
        python_version: Some(String::from("3.14.5")),
        name: Some(String::from("ansible")),
        recreate: true,
    };
    let steps = action
        .plan(&Manifest::default(), &Contexts::default())
        .unwrap();

    std::env::set_var("HOME", old_home);
    std::env::set_var("PATH", old_path);

    assert!(steps.is_empty(), "expected no steps when version already matches");
}

#[test]
#[serial]
fn plan_recreate_true_recreates_when_version_differs() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let tmp = tempfile::tempdir().unwrap();

    // Fake pyenv: reports ansible as existing
    let fake_pyenv = tmp.path().join("pyenv");
    std::fs::write(
        &fake_pyenv,
        "#!/bin/sh\nif [ \"$1\" = \"virtualenvs\" ]; then printf 'ansible\\n'; fi\n",
    )
    .unwrap();
    std::fs::set_permissions(&fake_pyenv, std::fs::Permissions::from_mode(0o755)).unwrap();

    let old_home = std::env::var("HOME").unwrap_or_default();
    std::env::set_var("HOME", tmp.path());

    // Symlink points to OLD version 3.14.4, not the requested 3.14.5
    let versions_dir = tmp.path().join(".pyenv").join("versions");
    let target_dir = versions_dir.join("3.14.4").join("envs").join("ansible");
    std::fs::create_dir_all(&target_dir).unwrap();
    symlink(Path::new("3.14.4/envs/ansible"), versions_dir.join("ansible")).unwrap();

    let old_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

    let action = PyenvVirtualenv {
        python_version: Some(String::from("3.14.5")),
        name: Some(String::from("ansible")),
        recreate: true,
    };
    let steps = action
        .plan(&Manifest::default(), &Contexts::default())
        .unwrap();

    std::env::set_var("HOME", old_home);
    std::env::set_var("PATH", old_path);

    assert_eq!(2, steps.len(), "expected uninstall + create steps");
    let s0 = steps[0].atom.to_string();
    let s1 = steps[1].atom.to_string();
    assert!(s0.contains("uninstall"), "step 0 should be uninstall, got: {s0}");
    assert!(s0.contains("-f"), "uninstall should use -f flag, got: {s0}");
    assert!(s0.contains("ansible"), "uninstall should name the venv, got: {s0}");
    assert!(s1.contains("virtualenv"), "step 1 should be virtualenv create, got: {s1}");
    assert!(s1.contains("3.14.5"), "step 1 should use new version, got: {s1}");
}

#[test]
#[serial]
fn plan_recreate_true_recreates_when_version_undetectable() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();

    // Fake pyenv: reports ansible as existing
    let fake_pyenv = tmp.path().join("pyenv");
    std::fs::write(
        &fake_pyenv,
        "#!/bin/sh\nif [ \"$1\" = \"virtualenvs\" ]; then printf 'ansible\\n'; fi\n",
    )
    .unwrap();
    std::fs::set_permissions(&fake_pyenv, std::fs::Permissions::from_mode(0o755)).unwrap();

    let old_home = std::env::var("HOME").unwrap_or_default();
    std::env::set_var("HOME", tmp.path());
    // No symlink created under HOME — installed_python_version returns None

    let old_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

    let action = PyenvVirtualenv {
        python_version: Some(String::from("3.14.5")),
        name: Some(String::from("ansible")),
        recreate: true,
    };
    let steps = action
        .plan(&Manifest::default(), &Contexts::default())
        .unwrap();

    std::env::set_var("HOME", old_home);
    std::env::set_var("PATH", old_path);

    assert_eq!(2, steps.len(), "expected uninstall + create as fail-safe when version undetectable");
    assert!(steps[0].atom.to_string().contains("uninstall"));
    assert!(steps[1].atom.to_string().contains("virtualenv"));
}
```

- [ ] **Step 2: Run to confirm compile error (RED)**

```bash
cargo test -p etch-lib 'pyenv::virtualenv::tests::plan_recreate' 2>&1 | grep "^error" | head -5
```

Expected: `error[E0560]: struct 'PyenvVirtualenv' has no field named 'recreate'`

- [ ] **Step 3: Add `recreate` field to the struct**

Replace the current `PyenvVirtualenv` struct in `lib/src/actions/pyenv/virtualenv.rs`:

```rust
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PyenvVirtualenv {
    /// Python version to base the virtualenv on (e.g. "3.12.0").
    pub python_version: Option<String>,
    /// Name of the virtualenv to create (e.g. "myproject").
    pub name: Option<String>,
    /// When true, delete and recreate the virtualenv if its Python version
    /// differs from `python_version`. Default false preserves existing behavior.
    #[serde(default)]
    pub recreate: bool,
}
```

- [ ] **Step 4: Replace the entire `plan()` method body**

Replace `impl Action for PyenvVirtualenv`'s `plan()` method with:

```rust
fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
    use crate::atoms::command::Exec;

    let python_version = match &self.python_version {
        Some(v) => v.clone(),
        None => bail!("pyenv.virtualenv requires 'python_version' to be specified"),
    };

    let name = match &self.name {
        Some(n) => n.clone(),
        None => bail!("pyenv.virtualenv requires 'name' to be specified"),
    };

    if Self::virtualenv_exists(&name) {
        if !self.recreate {
            return Ok(vec![]);
        }
        let versions_dir = PathBuf::from(
            shellexpand::tilde("~/.pyenv/versions").into_owned(),
        );
        let current = installed_python_version(&versions_dir, &name);
        if current.as_deref() == Some(python_version.as_str()) {
            return Ok(vec![]);
        }
        return Ok(vec![
            Step {
                atom: Box::new(Exec {
                    command: String::from("pyenv"),
                    arguments: vec![
                        String::from("uninstall"),
                        String::from("-f"),
                        name.clone(),
                    ],
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Exec {
                    command: String::from("pyenv"),
                    arguments: vec![
                        String::from("virtualenv"),
                        python_version,
                        name,
                    ],
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
        ]);
    }

    Ok(vec![Step {
        atom: Box::new(Exec {
            command: String::from("pyenv"),
            arguments: vec![String::from("virtualenv"), python_version, name],
            ..Default::default()
        }),
        initializers: vec![],
        finalizers: vec![],
    }])
}
```

- [ ] **Step 5: Run all virtualenv tests (GREEN)**

```bash
cargo test -p etch-lib 'pyenv::virtualenv::tests' 2>&1 | tail -15
```

Expected: All tests pass (original 11 + new 9 = 20 tests).

- [ ] **Step 6: Run full test suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass, lint clean.

- [ ] **Step 7: Commit (Tasks 1 + 2 together)**

```bash
git add lib/src/actions/pyenv/virtualenv.rs
git commit -m "feat(pyenv): add recreate: field to pyenv.virtualenv for version-bump idempotency"
```

---

### Task 3: Update example + docs

**Files:**

- Modify: `examples/pyenv/pyenv-virtualenv.yaml`

- [ ] **Step 1: Read current example**

```bash
cat examples/pyenv/pyenv-virtualenv.yaml
```

- [ ] **Step 2: Add `recreate: true` variant**

Append to `examples/pyenv/pyenv-virtualenv.yaml`:

```yaml
# recreate: true — delete and recreate when python_version changes.
# Idempotent: no-op when the venv already uses the correct Python version.
# Use when Python patch version bumps must be picked up automatically.
- action: pyenv.virtualenv
  python_version: "3.14.5"
  name: ansible
  recreate: true
  where: 'variables.has_devtools == "true"'
```

- [ ] **Step 3: Run full test suite**

```bash
make test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add examples/pyenv/pyenv-virtualenv.yaml
git commit -m "docs: add recreate: true example for pyenv.virtualenv"
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
  --title "feat(pyenv): add recreate: field to pyenv.virtualenv" \
  --body "$(cat <<'EOF'
## Summary
- Adds `recreate: bool` (default `false`) to `pyenv.virtualenv`
- When `recreate: true`, detects version mismatch via pyenv symlink at `~/.pyenv/versions/{name}` — no subprocess
- Emits `pyenv uninstall -f {name}` + `pyenv virtualenv {version} {name}` on mismatch
- No change to existing behavior when `recreate: false` (default)
- Unblocks `etch-config/workstation/ansible.yaml` Python version bumps

## Test plan
- [x] `installed_python_version` tests (4): absent path, non-symlink, relative symlink, absolute symlink
- [x] `plan()` recreate tests (5): create-when-absent, skip-when-matches, recreate-when-differs, recreate-when-undetectable, regression-recreate-false
- [x] All existing 11 tests pass
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

- [ ] **Step 4: Post-merge cleanup + docs status update**

```bash
git fetch --prune && git reset --hard origin/main
git branch -D <branch-name>
```

> **Do this directly on main after the PR merges — not inside the worktree.**

Update `docs/superpowers/README.md` — add row:

```markdown
| 2026-06-09 | [pyenv-recreate-virtualenv](plans/2026-06-09-pyenv-recreate-virtualenv-plan.md) | [pyenv-recreate-virtualenv](specs/2026-06-09-pyenv-recreate-virtualenv-design.md) | Done |
```

Remove the `pyenv.recreate-virtualenv` backlog entry.

Add `> **Status: DONE**` banner at the top of this plan file.

Then commit and push.
