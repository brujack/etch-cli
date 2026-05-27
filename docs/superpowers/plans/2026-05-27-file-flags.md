# file.flags Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `file.flags` action that sets and clears BSD file flags (`hidden`, `nohidden`, `uchg`, `nouchg`) on macOS using `libc::stat()` and `libc::chflags()` for idempotent flag management.

**Architecture:** A new `Chflags` atom (`lib/src/atoms/file/chflags.rs`, `#[cfg(target_os = "macos")]`) handles syscall-level idempotency and flag mutation. A new `FileFlags` action (`lib/src/actions/file/flags.rs`) wraps the atom, validates flag names at plan time, and handles `privileged: true` via the `chflags` CLI. On non-macOS, `plan()` returns an immediate error.

**Tech Stack:** Rust, `libc = "0.2"` (macOS-only dep), `#[cfg(target_os = "macos")]` gating, existing `Atom`/`Action` patterns from `file.chmod`.

---

## File Map

| File                            | Status | Purpose                                           |
| ------------------------------- | ------ | ------------------------------------------------- |
| `lib/Cargo.toml`                | Modify | Add `libc = "0.2"` to macOS-only deps             |
| `lib/src/atoms/file/chflags.rs` | Create | `Chflags` atom — libc stat+chflags                |
| `lib/src/atoms/file/mod.rs`     | Modify | Register `chflags` module and re-export           |
| `lib/src/actions/file/flags.rs` | Create | `FileFlags` action — validates, plans             |
| `lib/src/actions/file/mod.rs`   | Modify | Register `flags` module                           |
| `lib/src/actions/mod.rs`        | Modify | Wire `Actions::FileFlags` into all 4 match blocks |
| `app/tests/integration.rs`      | Modify | macOS integration test for `file.flags`           |
| `examples/file/flags.yaml`      | Create | Example manifest                                  |
| `CLAUDE.md` (repo root)         | Modify | Add `file.flags` to action catalog                |
| `README.md` (repo root)         | Modify | Add `file.flags` to action catalog table          |

---

## Task 1: Add `libc` dep + `Chflags` atom

**Files:**

- Modify: `lib/Cargo.toml`
- Create: `lib/src/atoms/file/chflags.rs`
- Modify: `lib/src/atoms/file/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `lib/src/atoms/file/chflags.rs` with only the test module (struct and impl do not exist yet — tests fail to compile):

```rust
#[cfg(target_os = "macos")]
pub const UF_HIDDEN: u32 = 0x8000;
#[cfg(target_os = "macos")]
pub const UF_IMMUTABLE: u32 = 0x0002;

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::*;

    // compute_desired tests — no filesystem required

    #[test]
    fn compute_desired_hidden_sets_bit() {
        let flags = vec!["hidden".to_string()];
        let desired = compute_desired(0, &flags).unwrap();
        assert_eq!(desired & UF_HIDDEN, UF_HIDDEN);
    }

    #[test]
    fn compute_desired_nohidden_clears_bit() {
        let flags = vec!["nohidden".to_string()];
        let desired = compute_desired(UF_HIDDEN, &flags).unwrap();
        assert_eq!(desired & UF_HIDDEN, 0);
    }

    #[test]
    fn compute_desired_uchg_sets_bit() {
        let flags = vec!["uchg".to_string()];
        let desired = compute_desired(0, &flags).unwrap();
        assert_eq!(desired & UF_IMMUTABLE, UF_IMMUTABLE);
    }

    #[test]
    fn compute_desired_nouchg_clears_bit() {
        let flags = vec!["nouchg".to_string()];
        let desired = compute_desired(UF_IMMUTABLE, &flags).unwrap();
        assert_eq!(desired & UF_IMMUTABLE, 0);
    }

    #[test]
    fn compute_desired_combined_flags() {
        let flags = vec!["hidden".to_string(), "uchg".to_string()];
        let desired = compute_desired(0, &flags).unwrap();
        assert_eq!(desired & UF_HIDDEN, UF_HIDDEN);
        assert_eq!(desired & UF_IMMUTABLE, UF_IMMUTABLE);
    }

    #[test]
    fn compute_desired_nohidden_noop_when_already_clear() {
        let flags = vec!["nohidden".to_string()];
        let desired = compute_desired(0, &flags).unwrap();
        assert_eq!(desired, 0);
    }

    #[test]
    fn compute_desired_unknown_flag_errors() {
        let flags = vec!["badname".to_string()];
        let result = compute_desired(0, &flags);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown flag: badname"));
    }

    // Chflags atom tests — require a real tempfile

    #[test]
    fn plan_should_run_false_when_already_at_desired_state() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "").unwrap();

        // Clear all flags first — file starts with 0 flags.
        // plan() with nohidden on a file that has 0 flags → should_run: false
        let atom = Chflags {
            path: path.clone(),
            flags: vec!["nohidden".to_string()],
        };
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_run_true_when_flag_not_set() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "").unwrap();

        let atom = Chflags {
            path: path.clone(),
            flags: vec!["hidden".to_string()],
        };
        assert!(atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_errors_on_nonexistent_path() {
        let atom = Chflags {
            path: std::path::PathBuf::from("/nonexistent/path/file.txt"),
            flags: vec!["hidden".to_string()],
        };
        assert!(atom.plan().is_err());
    }

    #[test]
    fn execute_sets_flag_and_plan_returns_false_after() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "").unwrap();

        let mut atom = Chflags {
            path: path.clone(),
            flags: vec!["hidden".to_string()],
        };

        assert!(atom.plan().unwrap().should_run);
        atom.execute().unwrap();
        assert!(!atom.plan().unwrap().should_run);

        // Clean up: clear the hidden flag so tempdir can be removed
        let mut cleanup = Chflags {
            path: path.clone(),
            flags: vec!["nohidden".to_string()],
        };
        cleanup.execute().unwrap();
    }

    #[test]
    fn execute_clears_flag() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "").unwrap();

        // Set hidden first
        let mut set_atom = Chflags {
            path: path.clone(),
            flags: vec!["hidden".to_string()],
        };
        set_atom.execute().unwrap();

        // Now clear it
        let mut clear_atom = Chflags {
            path: path.clone(),
            flags: vec!["nohidden".to_string()],
        };
        assert!(clear_atom.plan().unwrap().should_run);
        clear_atom.execute().unwrap();
        assert!(!clear_atom.plan().unwrap().should_run);
    }

    #[test]
    fn display_includes_flags_and_path() {
        let atom = Chflags {
            path: std::path::PathBuf::from("/tmp/myfile"),
            flags: vec!["hidden".to_string()],
        };
        let s = format!("{atom}");
        assert!(s.contains("myfile"));
        assert!(s.contains("hidden"));
    }
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /Users/bruce/git-repos/personal/etch-cli
export PATH="/Users/bruce/.rustup/toolchains/nightly-aarch64-apple-darwin/bin:/opt/homebrew/bin:$PATH"
cargo test -p etch-lib --lib atoms::file::chflags 2>&1 | tail -20
```

Expected: compile error — `compute_desired`, `Chflags` not defined.

- [ ] **Step 3: Add `libc` dep and implement the `Chflags` atom**

Add to `lib/Cargo.toml` after the `[target.'cfg(unix)'.dependencies]` block:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
libc = "0.2"
```

Write full `lib/src/atoms/file/chflags.rs`:

```rust
use crate::atoms::{Atom, Outcome};
use anyhow::anyhow;
use std::ffi::CString;
use std::path::PathBuf;

pub const UF_HIDDEN: u32 = 0x8000;
pub const UF_IMMUTABLE: u32 = 0x0002;

pub struct Chflags {
    pub path: PathBuf,
    pub flags: Vec<String>,
}

pub(crate) fn compute_desired(current: u32, flags: &[String]) -> anyhow::Result<u32> {
    let mut desired = current;
    for flag in flags {
        match flag.as_str() {
            "hidden"   => desired |= UF_HIDDEN,
            "nohidden" => desired &= !UF_HIDDEN,
            "uchg"     => desired |= UF_IMMUTABLE,
            "nouchg"   => desired &= !UF_IMMUTABLE,
            other      => return Err(anyhow!("unknown flag: {other}")),
        }
    }
    Ok(desired)
}

fn get_st_flags(path: &std::path::Path) -> anyhow::Result<u32> {
    let cstr = CString::new(path.to_string_lossy().as_bytes())
        .map_err(|e| anyhow!("invalid path: {e}"))?;
    let mut sb: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::stat(cstr.as_ptr(), &mut sb) } != 0 {
        return Err(anyhow!(
            "stat({:?}) failed: {}",
            path,
            std::io::Error::last_os_error()
        ));
    }
    Ok(sb.st_flags)
}

impl std::fmt::Display for Chflags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Set BSD flags {:?} on {}",
            self.flags,
            self.path.display()
        )
    }
}

impl Atom for Chflags {
    fn plan(&self) -> anyhow::Result<Outcome> {
        let current = get_st_flags(&self.path)?;
        let desired = compute_desired(current, &self.flags)?;
        Ok(Outcome {
            side_effects: vec![],
            should_run: current != desired,
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        let current = get_st_flags(&self.path)?;
        let desired = compute_desired(current, &self.flags)?;
        let cstr = CString::new(self.path.to_string_lossy().as_bytes())
            .map_err(|e| anyhow!("invalid path: {e}"))?;
        if unsafe { libc::chflags(cstr.as_ptr(), desired as libc::c_uint) } != 0 {
            return Err(anyhow!(
                "chflags({:?}, {:#x}) failed: {}",
                self.path,
                desired,
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    // ... (tests written in Step 1)
}
```

Register in `lib/src/atoms/file/mod.rs` — add before the existing `mod chmod;` line:

```rust
#[cfg(target_os = "macos")]
pub mod chflags;
#[cfg(target_os = "macos")]
pub use chflags::Chflags;
```

Full updated `lib/src/atoms/file/mod.rs`:

```rust
#[cfg(target_os = "macos")]
pub mod chflags;
#[cfg(target_os = "macos")]
pub use chflags::Chflags;

mod chmod;
mod chown;
mod contents;
mod copy;
mod create;
mod decrypt;
mod link;
mod remove;
mod unarchive;

use super::Atom;
pub use chmod::Chmod;
pub use chown::Chown;
pub use contents::SetContents;
pub use copy::Copy;
pub use create::Create;
pub use decrypt::Decrypt;
pub use link::Link;
pub use remove::Remove;
pub use unarchive::Unarchive;

pub trait FileAtom: Atom {
    // Don't think this is needed? Validate soon
    fn get_path(&self) -> &std::path::PathBuf;
}
```

- [ ] **Step 4: Run tests and confirm they pass**

```bash
cargo test -p etch-lib --lib atoms::file::chflags 2>&1 | tail -20
```

Expected: all `chflags::tests::*` tests pass.

- [ ] **Step 5: Commit**

```bash
git add lib/Cargo.toml lib/src/atoms/file/chflags.rs lib/src/atoms/file/mod.rs
git commit -m "feat(atoms): add Chflags atom for BSD file flags (macOS)"
```

---

## Task 2: Add `FileFlags` action

**Files:**

- Create: `lib/src/actions/file/flags.rs`
- Modify: `lib/src/actions/file/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `lib/src/actions/file/flags.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn it_can_be_deserialized() {
        use crate::actions::Actions;
        let yaml = r#"
- action: file.flags
  path: /tmp/testfile
  flags: [hidden, uchg]
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileFlags(action)) => {
                assert_eq!("/tmp/testfile", action.action.path);
                assert_eq!(vec!["hidden", "uchg"], action.action.flags);
                assert!(!action.action.config.privileged);
            }
            _ => panic!("FileFlags didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_privileged() {
        use crate::actions::Actions;
        let yaml = r#"
- action: file.flags
  path: /tmp/testfile
  flags: [nohidden]
  privileged: true
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileFlags(action)) => {
                assert!(action.action.config.privileged);
            }
            _ => panic!("FileFlags didn't deserialize to the correct type"),
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn plan_errors_on_unknown_flag() {
        use super::FileFlags;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;
        let action = FileFlags {
            path: String::from("/tmp/testfile"),
            flags: vec!["badname".to_string()],
            config: FileActionConfig { privileged: false },
        };
        let result = action.plan(
            &crate::manifests::Manifest::default(),
            &crate::contexts::Contexts::default(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown flag: badname"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn plan_returns_chflags_step_when_not_privileged() {
        use super::FileFlags;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "").unwrap();

        let action = FileFlags {
            path: path.display().to_string(),
            flags: vec!["hidden".to_string()],
            config: FileActionConfig { privileged: false },
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        assert!(steps[0].atom.to_string().contains("hidden"));
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn plan_errors_on_non_macos() {
        use super::FileFlags;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;
        let action = FileFlags {
            path: String::from("/tmp/testfile"),
            flags: vec!["hidden".to_string()],
            config: FileActionConfig { privileged: false },
        };
        let result = action.plan(
            &crate::manifests::Manifest::default(),
            &crate::contexts::Contexts::default(),
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("only supported on macOS"));
    }

    #[test]
    fn summarize_includes_path_and_flags() {
        use super::FileFlags;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;
        let action = FileFlags {
            path: String::from("/tmp/myfile"),
            flags: vec!["uchg".to_string()],
            config: FileActionConfig { privileged: false },
        };
        let s = action.summarize();
        assert!(s.contains("/tmp/myfile"));
        assert!(s.contains("uchg"));
    }
}
```

Note: the deserialization tests reference `Actions::FileFlags` which doesn't exist yet — these fail at compile time.

- [ ] **Step 2: Add `pub mod flags;` to `lib/src/actions/file/mod.rs`**

Append `pub mod flags;` to the top of `lib/src/actions/file/mod.rs`:

```rust
pub mod chmod;
pub mod chown;
pub mod copy;
pub mod download;
pub mod flags;       // ← add this line
pub mod link;
pub mod remove;
pub mod unarchive;
// ... rest of file unchanged
```

- [ ] **Step 3: Run tests to confirm they fail as expected**

```bash
cargo test -p etch-lib --lib actions::file::flags 2>&1 | tail -20
```

Expected: compile error — `Actions::FileFlags` not defined yet.

- [ ] **Step 4: Implement `FileFlags`**

Full `lib/src/actions/file/flags.rs`:

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{FileAction, FileActionConfig};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFlags {
    pub path: String,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(flatten)]
    pub config: FileActionConfig,
}

const VALID_FLAGS: &[&str] = &["hidden", "nohidden", "uchg", "nouchg"];

fn validate_flags(flags: &[String]) -> anyhow::Result<()> {
    for flag in flags {
        if !VALID_FLAGS.contains(&flag.as_str()) {
            return Err(anyhow!("unknown flag: {flag}"));
        }
    }
    Ok(())
}

impl FileAction for FileFlags {
    fn file_action_config(&self) -> &FileActionConfig {
        &self.config
    }
}

impl Action for FileFlags {
    fn summarize(&self) -> String {
        format!("Set BSD flags {:?} on {}", self.flags, self.path)
    }

    fn plan(&self, _: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        #[cfg(not(target_os = "macos"))]
        return Err(anyhow!("file.flags is only supported on macOS"));

        #[cfg(target_os = "macos")]
        {
            validate_flags(&self.flags)?;

            if self.config.privileged {
                use crate::atoms::command::Exec;
                use crate::atoms::file::chflags::compute_desired;
                use crate::utilities;
                use std::ffi::CString;

                // Read current flags to determine if a change is needed and
                // to compute the full desired flag set (chflags replaces all
                // user flags — passing only the delta would clobber others).
                let cstr = CString::new(self.path.as_bytes())
                    .map_err(|e| anyhow!("invalid path: {e}"))?;
                let mut sb: libc::stat = unsafe { std::mem::zeroed() };
                if unsafe { libc::stat(cstr.as_ptr(), &mut sb) } != 0 {
                    return Err(anyhow!(
                        "stat({:?}) failed: {}",
                        self.path,
                        std::io::Error::last_os_error()
                    ));
                }
                let current = sb.st_flags;
                let desired = compute_desired(current, &self.flags)?;

                if current == desired {
                    return Ok(vec![]);
                }

                // Convert desired bitmask back to chflags flag names.
                let mut names: Vec<&str> = Vec::new();
                if desired & crate::atoms::file::chflags::UF_HIDDEN != 0 {
                    names.push("hidden");
                }
                if desired & crate::atoms::file::chflags::UF_IMMUTABLE != 0 {
                    names.push("uchg");
                }
                let flags_str = if names.is_empty() {
                    "none".to_string()
                } else {
                    names.join(",")
                };

                let privilege_provider = utilities::get_privilege_provider(contexts)
                    .unwrap_or_else(|| "sudo".to_string());
                return Ok(vec![Step {
                    atom: Box::new(Exec {
                        command: "chflags".into(),
                        arguments: vec![flags_str, self.path.clone()],
                        privileged: true,
                        privilege_provider,
                        ..Default::default()
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                }]);
            }

            Ok(vec![Step {
                atom: Box::new(crate::atoms::file::Chflags {
                    path: self.path.clone().into(),
                    flags: self.flags.clone(),
                }),
                initializers: vec![],
                finalizers: vec![],
            }])
        }
    }
}

#[cfg(test)]
mod tests {
    // ... (tests written in Step 1)
}
```

- [ ] **Step 5: Run tests — they still fail (Actions::FileFlags missing)**

```bash
cargo test -p etch-lib --lib actions::file::flags 2>&1 | tail -20
```

Expected: still compile error on `Actions::FileFlags` in deserialization tests.

- [ ] **Step 6: Wire `Actions::FileFlags` (needed to unblock tests)**

See Task 3 — complete it now before the tests can pass.

---

## Task 3: Wire `Actions::FileFlags` into `lib/src/actions/mod.rs`

**Files:**

- Modify: `lib/src/actions/mod.rs`

This task is required to unblock Task 2's tests. Do it after writing `flags.rs`.

- [ ] **Step 1: Add import**

After the existing `use file::chmod::FileChmod;` line (around line 26), add:

```rust
use file::flags::FileFlags;
```

- [ ] **Step 2: Add enum variant**

In the `Actions` enum, after the `FileChmod` variant, add:

```rust
#[serde(rename = "file.flags")]
FileFlags(ConditionalVariantAction<FileFlags>),
```

- [ ] **Step 3: Add to `inner_ref()`**

In the `inner_ref()` match block, after `Actions::FileChmod(a) => a,`, add:

```rust
Actions::FileFlags(a) => a,
```

- [ ] **Step 4: Add to `notify()`**

In the `notify()` match block, after `Actions::FileChmod(a) => &a.notify,`, add:

```rust
Actions::FileFlags(a) => &a.notify,
```

- [ ] **Step 5: Add to `Deref`**

In the `Deref` match block, after `Actions::FileChmod(a) => a,`, add:

```rust
Actions::FileFlags(a) => a,
```

- [ ] **Step 6: Add to `Display`**

In the `Display` match block, after `Actions::FileChmod(_) => "file.chmod",`, add:

```rust
Actions::FileFlags(_) => "file.flags",
```

- [ ] **Step 7: Update the existing `all_major_action_variants_can_be_deserialized` test**

In the test's YAML string (around line 487), add `file.flags` to the list. Also update the assertion count from `23` to `24`:

```yaml
- action: file.flags
  path: /tmp/f
  flags: [hidden]
```

Also update `all_action_variants_inner_ref_and_deref` test YAML and assertion from `26` to `27`, adding:

```yaml
- action: file.flags
  path: /tmp/f
  flags: [hidden]
```

And update `actions_display_names` test to add `"file.flags"` in both the YAML and the `expected_names` array, updating count from `25` to `26`.

- [ ] **Step 8: Run all lib tests**

```bash
cargo test -p etch-lib 2>&1 | tail -30
```

Expected: all tests pass, including new `flags::tests::*` tests.

- [ ] **Step 9: Commit**

```bash
git add lib/src/actions/file/flags.rs lib/src/actions/file/mod.rs lib/src/actions/mod.rs
git commit -m "feat(actions): add file.flags action for BSD file flags (macOS)"
```

---

## Task 4: Integration test

**Files:**

- Modify: `app/tests/integration.rs`

- [ ] **Step 1: Write the failing tests**

Append to `app/tests/integration.rs`:

```rust
// ─── file.flags (macOS only) ──────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn ls_flags(path: &std::path::Path) -> String {
    let output = std::process::Command::new("ls")
        .args(["-lO", path.to_str().unwrap()])
        .output()
        .expect("ls -lO failed");
    String::from_utf8(output.stdout).expect("non-UTF8 output")
}

#[test]
#[cfg(target_os = "macos")]
fn file_flags_sets_hidden() {
    let dir = tempdir().unwrap();

    let target = dir.path().join("flagtest.txt");
    fs::write(&target, "flagged").unwrap();

    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: file.flags\n    path: {}\n    flags: [hidden]\n",
            target.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();

    let flags_output = ls_flags(&target);
    assert!(
        flags_output.contains("hidden"),
        "expected 'hidden' in ls -lO output, got: {flags_output}"
    );

    // Clean up: clear the hidden flag so the tempdir can be removed.
    std::process::Command::new("chflags")
        .args(["nohidden", target.to_str().unwrap()])
        .status()
        .unwrap();
}

#[test]
#[cfg(target_os = "macos")]
fn file_flags_is_idempotent() {
    let dir = tempdir().unwrap();

    let target = dir.path().join("flagtest.txt");
    fs::write(&target, "flagged").unwrap();

    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: file.flags\n    path: {}\n    flags: [hidden]\n",
            target.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();
    apply(dir.path()).success(); // second apply must also succeed

    let flags_output = ls_flags(&target);
    assert!(flags_output.contains("hidden"));

    std::process::Command::new("chflags")
        .args(["nohidden", target.to_str().unwrap()])
        .status()
        .unwrap();
}

#[test]
#[cfg(target_os = "macos")]
fn file_flags_clears_hidden() {
    let dir = tempdir().unwrap();

    let target = dir.path().join("flagtest.txt");
    fs::write(&target, "flagged").unwrap();

    // Set the flag manually first.
    std::process::Command::new("chflags")
        .args(["hidden", target.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(ls_flags(&target).contains("hidden"));

    fs::write(
        dir.path().join("test.yaml"),
        format!(
            "actions:\n  - action: file.flags\n    path: {}\n    flags: [nohidden]\n",
            target.display()
        ),
    )
    .unwrap();

    apply(dir.path()).success();

    assert!(
        !ls_flags(&target).contains("hidden"),
        "expected 'hidden' to be cleared"
    );
}
```

- [ ] **Step 2: Run the integration tests to confirm they fail**

```bash
cargo test -p etch-cli --test integration file_flags 2>&1 | tail -20
```

Expected: tests run but fail — `file.flags` deserialization fails because `Actions::FileFlags` must be wired up (Task 3 must be done first). If Task 3 is complete, the tests may fail because the `etch` binary needs to be rebuilt.

Build the binary first:

```bash
cargo build 2>&1 | tail -10
```

Then re-run:

```bash
cargo test -p etch-cli --test integration file_flags 2>&1 | tail -20
```

Expected: all three tests pass.

- [ ] **Step 3: Run full test suite**

```bash
cargo test 2>&1 | tail -30
```

Expected: all tests pass.

- [ ] **Step 4: Run lint**

```bash
cargo clippy --all-targets -- -D warnings 2>&1 | tail -20
```

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add app/tests/integration.rs
git commit -m "test(integration): add macOS integration tests for file.flags"
```

---

## Task 5: Example manifest + docs

**Files:**

- Create: `examples/file/flags.yaml`
- Modify: `CLAUDE.md` (repo root) — action catalog
- Modify: `README.md` — action catalog table

- [ ] **Step 1: Create example manifest**

Write `examples/file/flags.yaml`:

```yaml
# Unhide ~/Library (macOS hides it by default)
- action: file.flags
  path: "{{ user.home_dir }}/Library"
  flags: [nohidden]
  where: 'os.name == "macos"'

# Protect SSH private key from accidental modification
- action: file.flags
  path: "{{ user.home_dir }}/.ssh/id_ed25519"
  flags: [uchg]
  where: 'os.name == "macos"'

# Clear immutable before modifying, then re-set
- action: file.flags
  path: "{{ user.home_dir }}/.ssh/id_ed25519"
  flags: [nouchg]
  where: 'os.name == "macos"'

# Combine: clear hidden AND set immutable in one call
- action: file.flags
  path: "{{ user.home_dir }}/important-dir"
  flags: [nohidden, uchg]
  where: 'os.name == "macos"'
```

- [ ] **Step 2: Update CLAUDE.md action catalog**

In `CLAUDE.md`, add `file.flags` to the Action Catalog table after the `file.chmod` row:

```markdown
| `file.flags` | Set/clear BSD file flags (macOS only) | `path`, `flags` (list: `hidden`/`nohidden`/`uchg`/`nouchg`), `privileged` (bool). Delta semantics — bits not mentioned are untouched. Only valid on macOS; `plan()` errors on Linux. |
```

- [ ] **Step 3: Update README.md action catalog**

In `README.md`, update the `file.*` row in the action catalog table:

```markdown
| `file.copy` / `file.link` / `file.chmod` / `file.flags` | Manage files, permissions, and BSD flags |
```

- [ ] **Step 4: Commit**

```bash
git add examples/file/flags.yaml CLAUDE.md README.md
git commit -m "docs(file-flags): add example manifest and action catalog entries"
```

---

## Post-merge (on main, not in worktree)

- [ ] Update `docs/superpowers/README.md`: set `file.flags` row status to **Done**
- [ ] Add `> **Status: DONE**` banner at the top of this plan file

These steps must be done directly on main after the PR merges — committing them inside the worktree causes a merge conflict during squash-merge.
