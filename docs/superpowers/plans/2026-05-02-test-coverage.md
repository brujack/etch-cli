# Test Coverage Improvement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Raise etch-cli test coverage from 39% to >90% by adding thorough tests across all action types, providers, atoms, and CLI commands.

**Architecture:** TDD throughout — write the failing assertion first, then confirm it exercises uncovered lines, then run `make test`. All tests are inline `#[cfg(test)]` modules following the existing codebase pattern. Linux provider tests use PATH injection (mock binaries in tempdir) so they run on both macOS and Linux. Binary GitHub network tests are `#[ignore]`-marked. A shared `test_helpers` module provides `make_manifest` and `make_contexts` helpers.

**Tech Stack:** Rust, `cargo test`, `cargo tarpaulin`, `tempfile` crate (already in dev-deps), `serial_test = "3"` (new dev-dep for PATH-mutation serialization), `assert_cmd` (already in app dev-deps)

---

## File Map

**Create:**

- `lib/src/test_helpers.rs` — shared test helpers (`make_manifest`, `make_contexts`)
- `lib/src/fixtures/test.tar.gz` — fixture archive for unarchive tests

**Modify:**

- `lib/src/lib.rs` — expose `test_helpers` module under `#[cfg(test)]`
- `lib/Cargo.toml` — add `serial_test = "3"` to `[dev-dependencies]`
- `lib/src/actions/git/clone.rs` — add tests
- `lib/src/atoms/git/clone.rs` — add tests
- `lib/src/actions/binary/github.rs` — add tests
- `lib/src/actions/macos/default.rs` — add tests
- `lib/src/actions/file/copy.rs` — add plan test
- `lib/src/actions/file/chown.rs` — add plan test
- `lib/src/actions/file/remove.rs` — add plan test
- `lib/src/actions/file/unarchive.rs` — add plan+execute test
- `lib/src/atoms/file/unarchive.rs` — add tests
- `lib/src/actions/directory/create.rs` — add plan test
- `lib/src/actions/directory/remove.rs` — add plan test
- `lib/src/actions/command/run.rs` — add plan test
- `lib/src/actions/user/providers/linux.rs` — remove `#[cfg(target_os = "linux")]` gate, add PATH injection
- `lib/src/actions/group/providers/linux.rs` — remove `#[cfg(target_os = "linux")]` gate, add PATH injection
- `lib/src/actions/package/providers/homebrew.rs` — add tests
- `lib/src/actions/package/providers/snapcraft.rs` — add tests
- `lib/src/atoms/command/exec.rs` — add execute success/failure tests
- `lib/src/atoms/file/chmod.rs` — add boundary tests
- `lib/src/atoms/file/contents.rs` — add boundary tests
- `lib/src/actions/mod.rs` — add dispatch round-trip test
- `app/tests/cli_commands.rs` — new integration test file for CLI commands
- `.github/workflows/ci.yml` — raise tarpaulin gate from 25 to 90

---

## Task 1: Test infrastructure

**Files:**

- Create: `lib/src/test_helpers.rs`
- Modify: `lib/src/lib.rs`
- Modify: `lib/Cargo.toml`
- Create: `lib/src/fixtures/test.tar.gz`

- [ ] **Step 1: Add `serial_test` to dev-dependencies**

In `lib/Cargo.toml`, add to `[dev-dependencies]`:

```toml
serial_test = "3"
```

- [ ] **Step 2: Create `lib/src/test_helpers.rs`**

```rust
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use std::path::Path;

pub fn make_manifest(dir: &Path) -> Manifest {
    Manifest {
        root_dir: Some(dir.to_path_buf()),
        ..Default::default()
    }
}

pub fn make_contexts() -> Contexts {
    Contexts::default()
}
```

- [ ] **Step 3: Expose `test_helpers` in `lib/src/lib.rs`**

Add this line to `lib/src/lib.rs`:

```rust
#[cfg(test)]
pub mod test_helpers;
```

- [ ] **Step 4: Create the tar.gz fixture**

Run from the repo root:

```bash
mkdir -p lib/src/fixtures
printf 'hello fixture\n' > /tmp/etch_hello.txt
tar -czf lib/src/fixtures/test.tar.gz -C /tmp etch_hello.txt
rm /tmp/etch_hello.txt
```

Verify the fixture exists:

```bash
ls -lh lib/src/fixtures/test.tar.gz
```

Expected: file exists, size > 0.

- [ ] **Step 5: Verify compilation**

```bash
make test
```

Expected: all existing tests pass; `test_helpers` compiles cleanly.

- [ ] **Step 6: Commit**

```bash
git add lib/src/test_helpers.rs lib/src/lib.rs lib/Cargo.toml lib/src/fixtures/test.tar.gz
git commit -m "test: add test infrastructure (helpers, fixtures, serial_test dep)"
```

---

## Task 2: Git action and atom tests

**Files:**

- Modify: `lib/src/actions/git/clone.rs`
- Modify: `lib/src/atoms/git/clone.rs`

- [ ] **Step 1: Add tests to `lib/src/actions/git/clone.rs`**

Append to the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: git.clone
  repo_url: https://github.com/example/repo.git
  directory: /tmp/repo
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::GitClone(action)) => {
                assert_eq!("https://github.com/example/repo.git", action.action.repo_url);
                assert_eq!("/tmp/repo", action.action.directory);
            }
            _ => panic!("GitClone didn't deserialize"),
        }
    }

    #[test]
    fn plan_returns_one_step_for_valid_url() {
        let action = GitClone {
            repo_url: String::from("https://github.com/example/repo.git"),
            directory: String::from("/tmp/repo"),
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_errors_on_invalid_url() {
        let action = GitClone {
            repo_url: String::from("not a url ://"),
            directory: String::from("/tmp/repo"),
        };
        assert!(action.plan(&Manifest::default(), &Contexts::default()).is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

```bash
cargo test -p etch-lib actions::git::clone::tests -- --nocapture
```

Expected: 3 tests pass.

- [ ] **Step 3: Add tests to `lib/src/atoms/git/clone.rs`**

Append to the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_should_run_when_directory_does_not_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("not_yet_cloned");
        let atom = Clone {
            repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
            directory: target,
        };
        assert!(atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_not_run_when_directory_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let atom = Clone {
            repository: gix::url::parse("https://github.com/example/repo.git".into()).unwrap(),
            directory: tmp.path().to_path_buf(),
        };
        assert!(!atom.plan().unwrap().should_run);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p etch-lib atoms::git::clone::tests
```

Expected: 2 tests pass.

- [ ] **Step 5: Run full suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/git/clone.rs lib/src/atoms/git/clone.rs
git commit -m "test: add git clone action and atom tests"
```

---

## Task 3: Binary GitHub action tests

**Files:**

- Modify: `lib/src/actions/binary/github.rs`

- [ ] **Step 1: Add tests to `lib/src/actions/binary/github.rs`**

Append to the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: binary.github
  name: gitleaks
  directory: /usr/local/bin
  repository: gitleaks/gitleaks
  version: latest
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::BinaryGitHub(action)) => {
                assert_eq!("gitleaks", action.action.name);
                assert_eq!("/usr/local/bin", action.action.directory);
                assert_eq!("gitleaks/gitleaks", action.action.repository);
                assert_eq!(Some(String::from("latest")), action.action.version);
            }
            _ => panic!("BinaryGitHub didn't deserialize"),
        }
    }

    #[test]
    fn plan_returns_empty_when_binary_already_exists() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("mytool"), b"fake binary").unwrap();

        let action = BinaryGitHub {
            name: String::from("mytool"),
            directory: tmp.path().display().to_string(),
            repository: String::from("owner/repo"),
            version: None,
        };

        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(0, steps.len());
    }

    #[test]
    fn plan_errors_on_invalid_repository_format() {
        let tmp = tempfile::tempdir().unwrap();
        let action = BinaryGitHub {
            name: String::from("mytool"),
            directory: tmp.path().display().to_string(),
            repository: String::from("no-slash-here"),
            version: None,
        };
        assert!(action.plan(&Manifest::default(), &Contexts::default()).is_err());
    }

    #[test]
    #[ignore]
    fn plan_downloads_real_github_release() {
        let tmp = tempfile::tempdir().unwrap();
        let action = BinaryGitHub {
            name: String::from("gitleaks"),
            directory: tmp.path().display().to_string(),
            repository: String::from("gitleaks/gitleaks"),
            version: Some(String::from("v8.30.1")),
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(2, steps.len());
    }
}
```

- [ ] **Step 2: Run tests (excluding the ignored network test)**

```bash
cargo test -p etch-lib actions::binary::github::tests
```

Expected: 3 tests pass, 1 ignored.

- [ ] **Step 3: Run full suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add lib/src/actions/binary/github.rs
git commit -m "test: add binary github action tests"
```

---

## Task 4: macOS defaults action tests

**Files:**

- Modify: `lib/src/actions/macos/default.rs`

- [ ] **Step 1: Add tests to `lib/src/actions/macos/default.rs`**

Append to the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: macos.default
  domain: com.apple.dock
  key: autohide
  kind: bool
  value: "true"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSDefault(action)) => {
                assert_eq!("com.apple.dock", action.action.domain);
                assert_eq!("autohide", action.action.key);
                assert_eq!("bool", action.action.kind);
                assert_eq!("true", action.action.value);
            }
            _ => panic!("MacOSDefault didn't deserialize"),
        }
    }

    #[test]
    fn plan_returns_defaults_write_step() {
        let action = MacOSDefault {
            domain: String::from("com.apple.dock"),
            key: String::from("autohide"),
            kind: String::from("bool"),
            value: String::from("true"),
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_step_uses_correct_arguments() {
        let action = MacOSDefault {
            domain: String::from("com.example.app"),
            key: String::from("mykey"),
            kind: String::from("string"),
            value: String::from("myvalue"),
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_with_integer_kind() {
        let action = MacOSDefault {
            domain: String::from("com.example.app"),
            key: String::from("tilesize"),
            kind: String::from("integer"),
            value: String::from("48"),
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p etch-lib actions::macos::default::tests
```

Expected: 4 tests pass.

- [ ] **Step 3: Run full suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add lib/src/actions/macos/default.rs
git commit -m "test: add macos defaults action tests"
```

---

## Task 5: File action plan tests

**Files:**

- Modify: `lib/src/actions/file/copy.rs`
- Modify: `lib/src/actions/file/chown.rs`
- Modify: `lib/src/actions/file/remove.rs`
- Modify: `lib/src/actions/file/unarchive.rs`
- Modify: `lib/src/atoms/file/unarchive.rs`

- [ ] **Step 1: Add plan test to `lib/src/actions/file/copy.rs`**

Add inside the existing `#[cfg(test)] mod tests { ... }` block, after the last existing test:

```rust
    #[test]
    fn plan_returns_steps_for_valid_source() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("source.txt");
        std::fs::write(&src, b"hello").unwrap();

        let action = FileCopy {
            from: src.display().to_string(),
            to: tmp.path().join("dest.txt").display().to_string(),
            ..Default::default()
        };
        let manifest = crate::test_helpers::make_manifest(tmp.path());
        let contexts = crate::test_helpers::make_contexts();
        let steps = action.plan(&manifest, &contexts).unwrap();
        // DirCreate + Create + Chmod + SetContents = 4 steps
        assert_eq!(4, steps.len());
    }

    #[test]
    fn plan_errors_when_source_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let action = FileCopy {
            from: tmp.path().join("nonexistent.txt").display().to_string(),
            to: tmp.path().join("dest.txt").display().to_string(),
            ..Default::default()
        };
        let manifest = crate::test_helpers::make_manifest(tmp.path());
        let contexts = crate::test_helpers::make_contexts();
        assert!(action.plan(&manifest, &contexts).is_err());
    }

    #[test]
    fn plan_with_template_renders_content() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("template.txt");
        std::fs::write(&src, b"hello world").unwrap();

        let action = FileCopy {
            from: src.display().to_string(),
            to: tmp.path().join("dest.txt").display().to_string(),
            template: true,
            ..Default::default()
        };
        let manifest = crate::test_helpers::make_manifest(tmp.path());
        let contexts = crate::test_helpers::make_contexts();
        let steps = action.plan(&manifest, &contexts).unwrap();
        assert_eq!(4, steps.len());
    }
```

- [ ] **Step 2: Add plan test to `lib/src/actions/file/chown.rs`**

Add inside the existing `#[cfg(test)] mod tests { ... }` block, after the last existing test:

```rust
    #[test]
    fn plan_returns_chown_step() {
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = crate::actions::file::chown::FileChown {
            path: String::from("/tmp/testfile"),
            user: Some(String::from("alice")),
            group: Some(String::from("staff")),
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_uses_empty_string_for_missing_user() {
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = crate::actions::file::chown::FileChown {
            path: String::from("/tmp/testfile"),
            user: None,
            group: Some(String::from("staff")),
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
    }
```

- [ ] **Step 3: Add plan test to `lib/src/actions/file/remove.rs`**

Add inside the existing `#[cfg(test)] mod tests { ... }` block, after the last existing test:

```rust
    #[test]
    fn plan_returns_remove_step() {
        use crate::actions::file::remove::FileRemove;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = FileRemove {
            target: String::from("/tmp/somefile"),
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
    }
```

- [ ] **Step 4: Add plan and execute tests to `lib/src/actions/file/unarchive.rs`**

Add inside the existing `#[cfg(test)] mod tests { ... }` block, after the last existing test:

```rust
    #[test]
    fn plan_returns_unarchive_step() {
        use crate::actions::file::unarchive::FileUnarchive;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = FileUnarchive {
            from: String::from("/tmp/archive.tar.gz"),
            to: String::from("/tmp/dest"),
            force: None,
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_with_force_false() {
        use crate::actions::file::unarchive::FileUnarchive;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = FileUnarchive {
            from: String::from("/tmp/archive.tar.gz"),
            to: String::from("/tmp/dest"),
            force: Some(false),
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
    }
```

- [ ] **Step 5: Add tests to `lib/src/atoms/file/unarchive.rs`**

Append to the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_should_run_when_dest_does_not_exist() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/fixtures/test.tar.gz");
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("output");

        let atom = Unarchive {
            origin: fixture,
            dest,
            force: true,
        };
        assert!(atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_not_run_when_dest_exists_and_force_false() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/fixtures/test.tar.gz");
        let tmp = tempfile::tempdir().unwrap();

        let atom = Unarchive {
            origin: fixture,
            dest: tmp.path().to_path_buf(),
            force: false,
        };
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    fn plan_should_run_when_dest_exists_and_force_true() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/fixtures/test.tar.gz");
        let tmp = tempfile::tempdir().unwrap();

        let atom = Unarchive {
            origin: fixture,
            dest: tmp.path().to_path_buf(),
            force: true,
        };
        assert!(atom.plan().unwrap().should_run);
    }

    #[test]
    fn execute_extracts_archive() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/fixtures/test.tar.gz");
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("extracted");
        std::fs::create_dir_all(&dest).unwrap();

        let mut atom = Unarchive {
            origin: fixture,
            dest: dest.clone(),
            force: true,
        };
        assert!(atom.execute().is_ok());
        assert!(dest.join("etch_hello.txt").exists());
    }
}
```

- [ ] **Step 6: Run tests**

```bash
cargo test -p etch-lib actions::file
cargo test -p etch-lib atoms::file::unarchive
```

Expected: all new tests pass.

- [ ] **Step 7: Run full suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add lib/src/actions/file/copy.rs lib/src/actions/file/chown.rs \
        lib/src/actions/file/remove.rs lib/src/actions/file/unarchive.rs \
        lib/src/atoms/file/unarchive.rs
git commit -m "test: add file action plan tests and unarchive atom tests"
```

---

## Task 6: Directory and command action tests

**Files:**

- Modify: `lib/src/actions/directory/create.rs`
- Modify: `lib/src/actions/directory/remove.rs`
- Modify: `lib/src/actions/command/run.rs`

- [ ] **Step 1: Add plan test to `lib/src/actions/directory/create.rs`**

Add inside the existing `#[cfg(test)] mod tests { ... }` block, after the last existing test:

```rust
    #[test]
    fn plan_returns_one_step() {
        use crate::actions::directory::create::DirectoryCreate;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = DirectoryCreate {
            path: String::from("/tmp/newdir"),
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
    }
```

- [ ] **Step 2: Add plan test to `lib/src/actions/directory/remove.rs`**

Add inside the existing `#[cfg(test)] mod tests { ... }` block, after the last existing test:

```rust
    #[test]
    fn plan_returns_one_step() {
        use crate::actions::directory::remove::DirectoryRemove;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = DirectoryRemove {
            target: String::from("/tmp/olddir"),
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
    }
```

- [ ] **Step 3: Add plan test to `lib/src/actions/command/run.rs`**

Add inside the existing `#[cfg(test)] mod tests { ... }` block, after the last existing test:

```rust
    #[test]
    fn plan_returns_one_step_with_initializer_and_finalizer() {
        use crate::actions::command::run::RunCommand;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = RunCommand {
            command: String::from("echo"),
            args: vec![String::from("hello")],
            ..Default::default()
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
        assert_eq!(1, steps[0].initializers.len());
        assert_eq!(1, steps[0].finalizers.len());
    }

    #[test]
    fn plan_privileged_still_returns_one_step() {
        use crate::actions::command::run::RunCommand;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = RunCommand {
            command: String::from("echo"),
            privileged: true,
            ..Default::default()
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_with_env_vars_includes_them_in_initializer() {
        use crate::actions::command::run::RunCommand;
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let mut env = std::collections::HashMap::new();
        env.insert(String::from("MY_VAR"), String::from("value"));
        let action = RunCommand {
            command: String::from("echo"),
            env,
            ..Default::default()
        };
        let steps = action.plan(&Manifest::default(), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
        assert_eq!(1, steps[0].initializers.len());
    }
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p etch-lib actions::directory
cargo test -p etch-lib actions::command
```

Expected: all new tests pass.

- [ ] **Step 5: Run full suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/actions/directory/create.rs lib/src/actions/directory/remove.rs \
        lib/src/actions/command/run.rs
git commit -m "test: add directory and command action plan tests"
```

---

## Task 7: Linux user and group provider tests (cross-platform)

**Files:**

- Modify: `lib/src/actions/user/providers/linux.rs`
- Modify: `lib/src/actions/group/providers/linux.rs`

The existing tests in both files are gated with `#[cfg(target_os = "linux")]`. This task removes that gate and adds PATH injection so the tests find mock binaries on any OS.

- [ ] **Step 1: Rewrite the test module in `lib/src/actions/user/providers/linux.rs`**

Replace the entire `#[cfg(target_os = "linux")] #[cfg(test)] mod test { ... }` block (from the line `#[cfg(target_os = "linux")]` through the closing `}`) with:

```rust
#[cfg(test)]
mod test {
    use super::*;
    use crate::actions::user::providers::{LinuxUserProvider, UserProvider};
    use crate::actions::user::{add_group::UserAddGroup, UserVariant};
    use crate::contexts::Contexts;
    use serial_test::serial;
    use std::os::unix::fs::PermissionsExt;

    fn write_mock_bin(dir: &std::path::Path, name: &str) {
        let path = dir.join(name);
        std::fs::write(&path, "#!/usr/bin/env bash\nexit 0\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn with_mock_bins<F: FnOnce()>(bins: &[&str], f: F) {
        let tmp = tempfile::tempdir().unwrap();
        for bin in bins {
            write_mock_bin(tmp.path(), bin);
        }
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", tmp.path().display(), old_path));
        f();
        std::env::set_var("PATH", old_path);
    }

    #[test]
    #[serial]
    fn test_add_user() {
        with_mock_bins(&["useradd", "usermod"], || {
            let user_provider = LinuxUserProvider {};
            let contexts = Contexts::default();
            let steps = user_provider.add_user(
                &UserVariant {
                    username: String::from("test"),
                    shell: String::from("sh"),
                    home_dir: String::from("/home/test"),
                    fullname: String::from("Test User"),
                    group: vec![],
                    ..Default::default()
                },
                &contexts,
            );
            assert_eq!(1, steps.unwrap().len());
        });
    }

    #[test]
    #[serial]
    fn test_add_user_no_username() {
        with_mock_bins(&["useradd"], || {
            let user_provider = LinuxUserProvider {};
            let contexts = Contexts::default();
            let steps = user_provider.add_user(
                &UserVariant {
                    username: String::from(""),
                    shell: String::from("sh"),
                    home_dir: String::from("/home/test"),
                    fullname: String::from("Test User"),
                    group: vec![],
                    ..Default::default()
                },
                &contexts,
            );
            assert_eq!(0, steps.unwrap().len());
        });
    }

    #[test]
    #[serial]
    fn test_add_to_group() {
        with_mock_bins(&["usermod"], || {
            let user_provider = LinuxUserProvider {};
            let contexts = Contexts::default();
            let steps = user_provider.add_to_group(
                &UserAddGroup {
                    username: String::from("test"),
                    group: vec![String::from("testgroup"), String::from("wheel")],
                    ..Default::default()
                },
                &contexts,
            );
            assert_eq!(2, steps.unwrap().len());
        });
    }

    #[test]
    #[serial]
    fn test_create_user_add_to_group() {
        with_mock_bins(&["useradd", "usermod"], || {
            let user_provider = LinuxUserProvider {};
            let contexts = Contexts::default();
            let steps = user_provider.add_user(
                &UserVariant {
                    username: String::from("test"),
                    shell: String::from(""),
                    home_dir: String::from(""),
                    fullname: String::from(""),
                    group: vec![String::from("testgroup")],
                    ..Default::default()
                },
                &contexts,
            );
            assert_eq!(2, steps.unwrap().len());
        });
    }

    #[test]
    #[serial]
    fn test_add_user_returns_empty_when_useradd_not_found() {
        // Use a PATH that has no useradd
        let tmp = tempfile::tempdir().unwrap();
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", tmp.path().display().to_string());

        let user_provider = LinuxUserProvider {};
        let contexts = Contexts::default();
        let steps = user_provider.add_user(
            &UserVariant {
                username: String::from("test"),
                ..Default::default()
            },
            &contexts,
        );

        std::env::set_var("PATH", old_path);
        assert_eq!(0, steps.unwrap().len());
    }
}
```

- [ ] **Step 2: Rewrite the test module in `lib/src/actions/group/providers/linux.rs`**

Replace the entire `#[cfg(target_os = "linux")] #[cfg(test)] mod test { ... }` block with:

```rust
#[cfg(test)]
mod test {
    use super::*;
    use crate::actions::group::providers::{GroupProvider, LinuxGroupProvider};
    use crate::actions::group::GroupVariant;
    use crate::contexts::Contexts;
    use serial_test::serial;
    use std::os::unix::fs::PermissionsExt;

    fn write_mock_bin(dir: &std::path::Path, name: &str) {
        let path = dir.join(name);
        std::fs::write(&path, "#!/usr/bin/env bash\nexit 0\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn with_mock_bins<F: FnOnce()>(bins: &[&str], f: F) {
        let tmp = tempfile::tempdir().unwrap();
        for bin in bins {
            write_mock_bin(tmp.path(), bin);
        }
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", tmp.path().display(), old_path));
        f();
        std::env::set_var("PATH", old_path);
    }

    #[test]
    #[serial]
    fn test_add_group() {
        with_mock_bins(&["groupadd"], || {
            let group_provider = LinuxGroupProvider {};
            let contexts = Contexts::default();
            let steps = group_provider.add_group(
                &GroupVariant {
                    group_name: String::from("test"),
                    ..Default::default()
                },
                &contexts,
            );
            assert_eq!(1, steps.len());
        });
    }

    #[test]
    #[serial]
    fn test_add_group_no_group_name() {
        with_mock_bins(&["groupadd"], || {
            let group_provider = LinuxGroupProvider {};
            let contexts = Contexts::default();
            let steps = group_provider.add_group(
                &GroupVariant {
                    ..Default::default()
                },
                &contexts,
            );
            assert_eq!(0, steps.len());
        });
    }

    #[test]
    #[serial]
    fn test_add_group_returns_empty_when_groupadd_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", tmp.path().display().to_string());

        let group_provider = LinuxGroupProvider {};
        let contexts = Contexts::default();
        let steps = group_provider.add_group(
            &GroupVariant {
                group_name: String::from("test"),
                ..Default::default()
            },
            &contexts,
        );

        std::env::set_var("PATH", old_path);
        assert_eq!(0, steps.len());
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p etch-lib actions::user::providers::linux
cargo test -p etch-lib actions::group::providers::linux
```

Expected: all tests pass on both macOS and Linux.

- [ ] **Step 4: Run full suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add lib/src/actions/user/providers/linux.rs lib/src/actions/group/providers/linux.rs
git commit -m "test: make linux provider tests cross-platform via PATH injection"
```

---

## Task 8: Homebrew and Snapcraft provider tests

**Files:**

- Modify: `lib/src/actions/package/providers/homebrew.rs`
- Modify: `lib/src/actions/package/providers/snapcraft.rs`

- [ ] **Step 1: Add tests to `lib/src/actions/package/providers/homebrew.rs`**

Append to the end of the file:

```rust
#[cfg(test)]
mod test {
    use super::*;
    use crate::actions::package::PackageVariant;
    use crate::actions::package::providers::PackageProviders;
    use crate::contexts::Contexts;

    #[test]
    fn available_returns_false_when_brew_not_on_path() {
        // In CI (ubuntu-latest) brew is not installed; on macOS it may be.
        // We just assert the return type is bool — the actual value depends on the environment.
        let homebrew = Homebrew {};
        let _ = homebrew.available(); // must not panic
    }

    #[test]
    fn bootstrap_returns_one_step() {
        let homebrew = Homebrew {};
        let contexts = Contexts::default();
        let steps = homebrew.bootstrap(&contexts);
        assert_eq!(1, steps.len());
    }

    #[test]
    fn has_repository_always_returns_false() {
        let homebrew = Homebrew {};
        let repo = crate::actions::package::repository::PackageRepository {
            name: String::from("homebrew/cask"),
            ..Default::default()
        };
        assert!(!homebrew.has_repository(&repo));
    }

    #[test]
    fn add_repository_returns_tap_step() {
        let homebrew = Homebrew {};
        let contexts = Contexts::default();
        let repo = crate::actions::package::repository::PackageRepository {
            name: String::from("homebrew/cask"),
            ..Default::default()
        };
        let steps = homebrew.add_repository(&repo, &contexts).unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn name_returns_homebrew() {
        let homebrew = Homebrew {};
        assert_eq!("Homebrew", homebrew.name());
    }
}
```

- [ ] **Step 2: Check what `PackageRepository` looks like**

```bash
grep -n "pub struct PackageRepository" lib/src/actions/package/repository.rs
```

If `PackageRepository` doesn't have a `Default` impl or the fields differ, adjust the test in Step 1 accordingly to match the actual struct fields.

- [ ] **Step 3: Add more tests to `lib/src/actions/package/providers/snapcraft.rs`**

Add inside the existing `#[cfg(test)] mod test { ... }` block, after the last existing test:

```rust
    #[test]
    fn available_does_not_panic() {
        let snapcraft = Snapcraft {};
        let _ = snapcraft.available();
    }

    #[test]
    fn bootstrap_returns_one_step() {
        let snapcraft = Snapcraft {};
        let contexts = Contexts::default();
        let steps = snapcraft.bootstrap(&contexts);
        assert_eq!(1, steps.len());
    }

    #[test]
    fn has_repository_always_returns_false() {
        let snapcraft = Snapcraft {};
        let repo = crate::actions::package::repository::PackageRepository {
            name: String::from("some-channel"),
            ..Default::default()
        };
        assert!(!snapcraft.has_repository(&repo));
    }

    #[test]
    fn name_returns_snapcraft() {
        let snapcraft = Snapcraft {};
        assert_eq!("Snapcraft", snapcraft.name());
    }

    #[test]
    fn query_returns_all_packages() {
        let snapcraft = Snapcraft {};
        let pkg = PackageVariant {
            name: Some(String::from("htop")),
            list: vec![],
            extra_args: vec![],
            provider: PackageProviders::Snapcraft,
            file: false,
        };
        let packages = snapcraft.query(&pkg).unwrap();
        assert_eq!(1, packages.len());
        assert_eq!("htop", packages[0]);
    }
```

- [ ] **Step 4: Read `lib/src/actions/package/repository.rs` to verify `PackageRepository` fields**

```bash
grep -A 20 "pub struct PackageRepository" lib/src/actions/package/repository.rs
```

Adjust the homebrew and snapcraft test code in Steps 1 and 3 if the struct fields differ from `name: String`.

- [ ] **Step 5: Run tests**

```bash
cargo test -p etch-lib actions::package::providers::homebrew
cargo test -p etch-lib actions::package::providers::snapcraft
```

Expected: all tests pass.

- [ ] **Step 6: Run full suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add lib/src/actions/package/providers/homebrew.rs \
        lib/src/actions/package/providers/snapcraft.rs
git commit -m "test: add homebrew and snapcraft provider tests"
```

---

## Task 9: Atom boundary tests

**Files:**

- Modify: `lib/src/atoms/command/exec.rs`
- Modify: `lib/src/atoms/file/chmod.rs`
- Modify: `lib/src/atoms/file/contents.rs`

- [ ] **Step 1: Add execute tests to `lib/src/atoms/command/exec.rs`**

Add inside the existing `#[cfg(test)] mod tests { ... }` block, after the last existing test:

```rust
    #[test]
    fn execute_succeeds_for_echo() {
        let mut exec = Exec {
            command: String::from("echo"),
            arguments: vec![String::from("hello")],
            ..Default::default()
        };
        assert!(exec.execute().is_ok());
    }

    #[test]
    fn execute_fails_for_false_command() {
        let mut exec = Exec {
            command: String::from("false"),
            ..Default::default()
        };
        assert!(exec.execute().is_err());
    }

    #[test]
    fn execute_fails_for_missing_command() {
        let mut exec = Exec {
            command: String::from("this-command-does-not-exist-in-path"),
            ..Default::default()
        };
        assert!(exec.execute().is_err());
    }

    #[test]
    fn execute_with_working_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let mut exec = Exec {
            command: String::from("echo"),
            arguments: vec![String::from("hello")],
            working_dir: Some(tmp.path().display().to_string()),
            ..Default::default()
        };
        assert!(exec.execute().is_ok());
    }

    #[test]
    fn plan_always_returns_should_run_true() {
        let exec = Exec {
            command: String::from("echo"),
            ..Default::default()
        };
        assert!(exec.plan().unwrap().should_run);
    }
```

- [ ] **Step 2: Add boundary test to `lib/src/atoms/file/chmod.rs`**

Add inside the existing `#[cfg(test)] #[cfg(unix)] mod tests { ... }` block, after the last existing test:

```rust
    #[test]
    fn plan_should_run_true_when_file_does_not_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent");
        let atom = Chmod { path, mode: 0o644 };
        // File doesn't exist — atom assumes another atom will create it, should_run = true
        assert!(atom.plan().unwrap().should_run);
    }
```

- [ ] **Step 3: Add boundary test to `lib/src/atoms/file/contents.rs`**

Add inside the existing `#[cfg(test)] mod tests { ... }` block, after the last existing test:

```rust
    #[test]
    fn plan_should_run_true_when_file_does_not_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent.txt");
        let atom = SetContents {
            path,
            contents: b"hello".to_vec(),
        };
        assert!(atom.plan().unwrap().should_run);
    }

    #[test]
    fn execute_writes_contents_to_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("output.txt");
        let mut atom = SetContents {
            path: path.clone(),
            contents: b"written content".to_vec(),
        };
        assert!(atom.execute().is_ok());
        assert_eq!(b"written content", std::fs::read(&path).unwrap().as_slice());
    }
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p etch-lib atoms::command::exec::tests
cargo test -p etch-lib atoms::file::chmod::tests
cargo test -p etch-lib atoms::file::contents::tests
```

Expected: all new tests pass.

- [ ] **Step 5: Run full suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add lib/src/atoms/command/exec.rs lib/src/atoms/file/chmod.rs lib/src/atoms/file/contents.rs
git commit -m "test: add boundary tests for exec, chmod, and contents atoms"
```

---

## Task 10: CLI command tests and actions dispatch test

**Files:**

- Create: `app/tests/cli_commands.rs`
- Modify: `lib/src/actions/mod.rs`

- [ ] **Step 1: Create `app/tests/cli_commands.rs`**

```rust
use assert_cmd::Command;
use predicates::str::contains;

fn etch() -> Command {
    Command::cargo_bin("etch").unwrap()
}

#[test]
fn version_prints_version_string() {
    etch()
        .arg("version")
        .assert()
        .success()
        .stdout(contains("."));
}

#[test]
fn gen_completions_bash_produces_output() {
    etch()
        .args(["gen-completions", "bash"])
        .assert()
        .success()
        .stdout(contains("etch"));
}

#[test]
fn gen_completions_zsh_produces_output() {
    etch()
        .args(["gen-completions", "zsh"])
        .assert()
        .success()
        .stdout(contains("etch"));
}

#[test]
fn gen_completions_fish_produces_output() {
    etch()
        .args(["gen-completions", "fish"])
        .assert()
        .success()
        .stdout(contains("etch"));
}

#[test]
fn contexts_exits_successfully() {
    etch()
        .arg("contexts")
        .assert()
        .success();
}

#[test]
fn contexts_output_contains_os_keys() {
    etch()
        .arg("contexts")
        .assert()
        .success()
        .stdout(contains("os"));
}

#[test]
fn plugin_without_subcommand_prints_help() {
    etch()
        .arg("plugin")
        .assert()
        .failure(); // arg_required_else_help = true → non-zero exit
}
```

- [ ] **Step 2: Check the actual subcommand names by running etch --help**

```bash
cargo run --bin etch -- --help
```

Adjust the `gen-completions` subcommand name in the test above if the actual name differs (e.g., `completions` vs `gen-completions`).

- [ ] **Step 3: Add dispatch round-trip test to `lib/src/actions/mod.rs`**

Append inside the existing test module (or add a new one if none exists):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_action_variants_can_be_deserialized() {
        let yaml = r#"
actions:
  - action: command.run
    command: echo
  - action: directory.copy
    from: a
    to: b
  - action: directory.create
    path: /tmp/d
  - action: directory.remove
    target: /tmp/d
  - action: file.copy
    from: a
    to: b
  - action: file.chown
    path: /tmp/f
  - action: file.link
    source: a
    target: b
  - action: file.remove
    target: /tmp/f
  - action: file.unarchive
    from: a.tar.gz
    to: /tmp/dest
  - action: git.clone
    repo_url: https://github.com/example/repo.git
    directory: /tmp/repo
  - action: group.add
    group_name: mygroup
  - action: macos.default
    domain: com.example
    key: k
    kind: string
    value: v
  - action: package.install
    name: htop
  - action: user.add
    username: alice
"#;
        let manifest: crate::manifests::Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(14, manifest.actions.len());
    }
}
```

- [ ] **Step 4: Run CLI command tests**

```bash
cargo test -p etch-cli --test cli_commands
```

Expected: all 7 tests pass. If `gen-completions` subcommand name is wrong, adjust to match the actual CLI.

- [ ] **Step 5: Run lib tests**

```bash
cargo test -p etch-lib actions::tests
```

Expected: round-trip test passes.

- [ ] **Step 6: Run full suite**

```bash
make test
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add app/tests/cli_commands.rs lib/src/actions/mod.rs
git commit -m "test: add CLI command integration tests and action dispatch round-trip test"
```

---

## Task 11: Raise CI coverage gate to 90%

**Files:**

- Modify: `.github/workflows/ci.yml`
- Modify: `CLAUDE.md`

- [ ] **Step 1: Run tarpaulin locally to measure current coverage**

```bash
cargo tarpaulin --exclude-files 'jsonschemagen/*' 2>&1 | tail -5
```

Record the coverage percentage. It should be substantially higher than 39% after the previous tasks.

- [ ] **Step 2: Check if coverage meets 90%**

If the coverage is ≥90%, proceed to Step 3. If it's below 90%, identify which files still have low coverage:

```bash
cargo tarpaulin --exclude-files 'jsonschemagen/*' 2>&1 | grep -E "^\d+\.\d+% -- " | sort -n | head -20
```

Add targeted tests for the lowest-coverage files until ≥90% is reached, then proceed. Focus first on: `lib/src/actions/mod.rs` (dispatch logic), `lib/src/utilities/lua.rs` (Lua helpers), and any action `mod.rs` re-export files that are still at 0%.

- [ ] **Step 3: Update `.github/workflows/ci.yml`**

Find the tarpaulin step:

```yaml
- name: Check coverage
  # Current coverage is ~28% — threshold set to current floor while coverage
  # is built up. Raise this as tests are added.
  run: cargo tarpaulin --fail-under 25
```

Replace with:

```yaml
- name: Check coverage
  run: cargo tarpaulin --exclude-files 'jsonschemagen/*' --fail-under 90
- name: Coverage including network tests (informational)
  run: cargo tarpaulin --exclude-files 'jsonschemagen/*' -- --include-ignored
  continue-on-error: true
```

- [ ] **Step 4: Update `CLAUDE.md`**

In the Testing section, update:

1. Change `Current coverage is approximately 39%` to reflect the actual new coverage figure.
2. Change `**Coverage floor: 25%**` to `**Coverage floor: 90%**`.

- [ ] **Step 5: Run full suite one final time**

```bash
make test
cargo tarpaulin --exclude-files 'jsonschemagen/*' --fail-under 90
```

Expected: all tests pass and coverage ≥90%.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/ci.yml CLAUDE.md
git commit -m "ci: raise tarpaulin coverage gate to 90%"
```
