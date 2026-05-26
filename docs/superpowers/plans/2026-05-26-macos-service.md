# macos.service Implementation Plan

> **Status: DONE**

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `macos.service` action that idempotently loads or unloads macOS LaunchDaemons and LaunchAgents using `launchctl`.

**Architecture:** A dedicated `MacOSService` atom handles the runtime logic — resolve the service label (from the field or via `defaults read`), check the current state via `launchctl list`, and call `launchctl load/unload -w` only when the state differs. The action validates the plist file exists at plan time and resolves the privilege provider. Follows the same patterns as `git.pull` (dedicated atom, PATH-mock tests).

**Tech Stack:** Rust, `shellexpand` (tilde expansion), `whoami` (privilege check), `tempfile` + `serial_test` (tests), `tracing` (instrumentation).

---

## File Structure

| File                               | Role                                                    |
| ---------------------------------- | ------------------------------------------------------- |
| `lib/src/atoms/macos/mod.rs`       | New — declares and re-exports the Service atom          |
| `lib/src/atoms/macos/service.rs`   | New — MacOSService atom with all runtime logic          |
| `lib/src/atoms/mod.rs`             | Modify — add `pub mod macos;`                           |
| `lib/src/actions/macos/service.rs` | New — MacOSService action with plan() and serialization |
| `lib/src/actions/macos/mod.rs`     | Modify — export MacOSService                            |
| `lib/src/actions/mod.rs`           | Modify — add MacOSService enum variant + 3 match arms   |
| `examples/macos/service.yaml`      | New — example manifest                                  |
| `CLAUDE.md`                        | Modify — add macos.service to action catalog            |

---

## Task 1: MacOSService Atom

**Files:**

- Create: `lib/src/atoms/macos/mod.rs`
- Create: `lib/src/atoms/macos/service.rs`
- Modify: `lib/src/atoms/mod.rs`

The atom handles all runtime logic: label resolution, state check, and launchctl invocation. The `load: bool` field avoids the atom depending on action types.

- [ ] **Step 1: Create the macos atoms module**

Create `lib/src/atoms/macos/mod.rs`:

```rust
mod service;
pub use service::Service;
```

- [ ] **Step 2: Write the failing test for plan()**

Add to `lib/src/atoms/macos/service.rs` (create the file with this content):

```rust
use super::super::Atom;
use crate::atoms::Outcome;
use std::path::PathBuf;
use tracing::instrument;

pub struct Service {
    pub plist: PathBuf,
    pub label: Option<String>,
    pub load: bool,
    pub privileged: bool,
    pub privilege_provider: String,
}

impl std::fmt::Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let action = if self.load { "load" } else { "unload" };
        write!(f, "MacOSService {} {}", action, self.plist.display())
    }
}

impl Atom for Service {
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_service(load: bool) -> Service {
        Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: Some("com.example.test".to_string()),
            load,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        }
    }

    #[test]
    fn plan_returns_should_run_true() {
        let atom = make_service(true);
        let outcome = atom.plan().unwrap();
        assert!(outcome.should_run);
        assert!(outcome.side_effects.is_empty());
    }

    #[test]
    fn display_load_format() {
        let atom = make_service(true);
        let s = format!("{atom}");
        assert!(s.contains("load"), "expected 'load' in: {s}");
        assert!(s.contains("/tmp/test.plist"), "expected plist in: {s}");
    }

    #[test]
    fn display_unload_format() {
        let atom = make_service(false);
        let s = format!("{atom}");
        assert!(s.contains("unload"), "expected 'unload' in: {s}");
    }
}
```

- [ ] **Step 3: Wire macos into atoms/mod.rs**

In `lib/src/atoms/mod.rs`, add after `pub mod http;`:

```rust
pub mod macos;
```

- [ ] **Step 4: Run tests to verify they compile and pass**

```bash
cd lib && cargo nextest run atoms::macos
```

Expected: 3 tests pass (plan_returns_should_run_true, display_load_format, display_unload_format).

- [ ] **Step 5: Write failing tests for execute() — idempotency and label resolution**

Add below the existing tests in `lib/src/atoms/macos/service.rs`:

```rust
    use serial_test::serial;

    fn write_mock_launchctl(
        mock_dir: &std::path::Path,
        calls_file: &std::path::Path,
        list_exit: i32,
    ) {
        use std::os::unix::fs::PermissionsExt;
        let script = mock_dir.join("launchctl");
        let content = format!(
            "#!/usr/bin/env bash\nprintf 'launchctl %s\\n' \"$*\" >> '{}'\nif [[ \"$1\" == \"list\" ]]; then\n  exit {}\nfi\nexit 0\n",
            calls_file.display(),
            list_exit
        );
        std::fs::write(&script, &content).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn write_mock_defaults(
        mock_dir: &std::path::Path,
        calls_file: &std::path::Path,
        label: &str,
        exit_code: i32,
    ) {
        use std::os::unix::fs::PermissionsExt;
        let script = mock_dir.join("defaults");
        let content = format!(
            "#!/usr/bin/env bash\nprintf 'defaults %s\\n' \"$*\" >> '{}'\nprintf '{}\\n'\nexit {}\n",
            calls_file.display(),
            label,
            exit_code
        );
        std::fs::write(&script, &content).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    #[serial]
    fn execute_skips_when_already_loaded() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // list exits 0 = service is loaded; atom wants load = true → should skip
        write_mock_launchctl(mock_dir.path(), &calls_file, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: Some("com.example.test".to_string()),
            load: true,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("load -w"),
            "expected no load call in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_skips_when_already_unloaded() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // list exits 1 = service is not loaded; atom wants load = false → should skip
        write_mock_launchctl(mock_dir.path(), &calls_file, 1);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: Some("com.example.test".to_string()),
            load: false,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("unload -w"),
            "expected no unload call in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_loads_when_unloaded() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // list exits 1 = not loaded; atom wants load = true → should call load
        write_mock_launchctl(mock_dir.path(), &calls_file, 1);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: Some("com.example.test".to_string()),
            load: true,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("load -w"),
            "expected 'load -w' in log, got: {log}"
        );
        assert!(
            log.contains("/tmp/test.plist"),
            "expected plist path in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_unloads_when_loaded() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // list exits 0 = loaded; atom wants load = false → should call unload
        write_mock_launchctl(mock_dir.path(), &calls_file, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: Some("com.example.test".to_string()),
            load: false,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("unload -w"),
            "expected 'unload -w' in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_uses_explicit_label_without_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_launchctl(mock_dir.path(), &calls_file, 0);
        // No defaults mock — if defaults is called, the real one would be used (and likely fail).
        // We verify defaults is not called by checking the calls log.
        write_mock_defaults(mock_dir.path(), &calls_file, "com.example.test", 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: Some("com.example.test".to_string()), // explicit label
            load: true,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("defaults"),
            "expected defaults not called when label is explicit, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_calls_defaults_when_label_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // list exits 0 = loaded; atom wants load = true → should skip (idempotent)
        write_mock_launchctl(mock_dir.path(), &calls_file, 0);
        write_mock_defaults(mock_dir.path(), &calls_file, "com.example.test", 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: None, // no label — must call defaults
            load: true,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("defaults"),
            "expected defaults to be called when label is None, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_errors_if_defaults_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_defaults(mock_dir.path(), &calls_file, "", 1); // defaults exits non-zero

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: None,
            load: true,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_err(), "expected Err from defaults failure, got Ok");
    }

    #[test]
    #[serial]
    fn execute_errors_if_launchctl_load_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");

        // launchctl: list exits 1 (not loaded), load exits 1 (failure)
        use std::os::unix::fs::PermissionsExt;
        let script = mock_dir.join("launchctl");
        let content = format!(
            "#!/usr/bin/env bash\nprintf 'launchctl %s\\n' \"$*\" >> '{}'\nexit 1\n",
            calls_file.display()
        );
        std::fs::write(&script, &content).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: Some("com.example.test".to_string()),
            load: true,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_err(), "expected Err from launchctl failure, got Ok");
    }
```

- [ ] **Step 6: Run tests to verify they fail (todo!() in execute)**

```bash
cd lib && cargo nextest run atoms::macos
```

Expected: `plan_returns_should_run_true`, `display_load_format`, `display_unload_format` PASS. Execute tests FAIL with `not yet implemented`.

- [ ] **Step 7: Implement execute() on the Service atom**

Replace the `todo!()` in `execute` and add the `resolve_label`, `is_loaded`, `run_launchctl` methods. Replace the `impl Atom for Service` block and add the methods on `Service`:

```rust
impl Service {
    fn resolve_label(&self) -> anyhow::Result<String> {
        if let Some(label) = &self.label {
            return Ok(label.clone());
        }
        let plist_str = self.plist.to_string_lossy().to_string();
        let output = std::process::Command::new("defaults")
            .args(["read", &plist_str, "Label"])
            .output()?;
        if !output.status.success() {
            anyhow::bail!(
                "defaults read {} Label failed: {}",
                self.plist.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let label = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if label.is_empty() {
            anyhow::bail!("Label key is empty in {}", self.plist.display());
        }
        Ok(label)
    }

    fn is_loaded(&self, label: &str) -> anyhow::Result<bool> {
        let status = std::process::Command::new("launchctl")
            .args(["list", label])
            .status()?;
        Ok(status.success())
    }

    fn run_launchctl(&self, subcommand: &str) -> anyhow::Result<()> {
        let plist_str = self.plist.to_string_lossy().to_string();
        let username = whoami::username().unwrap_or_else(|_| "unknown".to_string());
        let (cmd, args) = if self.privileged && username != "root" {
            (
                self.privilege_provider.clone(),
                vec![
                    "launchctl".to_string(),
                    subcommand.to_string(),
                    "-w".to_string(),
                    plist_str,
                ],
            )
        } else {
            (
                "launchctl".to_string(),
                vec![subcommand.to_string(), "-w".to_string(), plist_str],
            )
        };
        let status = std::process::Command::new(&cmd).args(&args).status()?;
        if !status.success() {
            anyhow::bail!(
                "launchctl {} -w {} failed with {}",
                subcommand,
                self.plist.display(),
                status
            );
        }
        Ok(())
    }
}
```

Add `use tracing::instrument;` and add `#[instrument]` attributes, and replace `execute` body:

```rust
impl Atom for Service {
    #[instrument(name = "macos.service.plan", level = "info", skip(self))]
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    #[instrument(name = "macos.service.execute", level = "info", skip(self))]
    fn execute(&mut self) -> anyhow::Result<()> {
        let label = self.resolve_label()?;
        let currently_loaded = self.is_loaded(&label)?;
        if self.load == currently_loaded {
            tracing::info!("macos.service {} already in desired state, skipping", label);
            return Ok(());
        }
        let subcommand = if self.load { "load" } else { "unload" };
        self.run_launchctl(subcommand)
    }
}
```

Add `use whoami;` at the top of the file.

Complete `lib/src/atoms/macos/service.rs` should be:

```rust
use super::super::Atom;
use crate::atoms::Outcome;
use std::path::PathBuf;
use tracing::instrument;

pub struct Service {
    pub plist: PathBuf,
    pub label: Option<String>,
    pub load: bool,
    pub privileged: bool,
    pub privilege_provider: String,
}

impl std::fmt::Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let action = if self.load { "load" } else { "unload" };
        write!(f, "MacOSService {} {}", action, self.plist.display())
    }
}

impl Service {
    fn resolve_label(&self) -> anyhow::Result<String> {
        if let Some(label) = &self.label {
            return Ok(label.clone());
        }
        let plist_str = self.plist.to_string_lossy().to_string();
        let output = std::process::Command::new("defaults")
            .args(["read", &plist_str, "Label"])
            .output()?;
        if !output.status.success() {
            anyhow::bail!(
                "defaults read {} Label failed: {}",
                self.plist.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let label = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if label.is_empty() {
            anyhow::bail!("Label key is empty in {}", self.plist.display());
        }
        Ok(label)
    }

    fn is_loaded(&self, label: &str) -> anyhow::Result<bool> {
        let status = std::process::Command::new("launchctl")
            .args(["list", label])
            .status()?;
        Ok(status.success())
    }

    fn run_launchctl(&self, subcommand: &str) -> anyhow::Result<()> {
        let plist_str = self.plist.to_string_lossy().to_string();
        let username = whoami::username().unwrap_or_else(|_| "unknown".to_string());
        let (cmd, args) = if self.privileged && username != "root" {
            (
                self.privilege_provider.clone(),
                vec![
                    "launchctl".to_string(),
                    subcommand.to_string(),
                    "-w".to_string(),
                    plist_str,
                ],
            )
        } else {
            (
                "launchctl".to_string(),
                vec![subcommand.to_string(), "-w".to_string(), plist_str],
            )
        };
        let status = std::process::Command::new(&cmd).args(&args).status()?;
        if !status.success() {
            anyhow::bail!(
                "launchctl {} -w {} failed with {}",
                subcommand,
                self.plist.display(),
                status
            );
        }
        Ok(())
    }
}

impl Atom for Service {
    #[instrument(name = "macos.service.plan", level = "info", skip(self))]
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    #[instrument(name = "macos.service.execute", level = "info", skip(self))]
    fn execute(&mut self) -> anyhow::Result<()> {
        let label = self.resolve_label()?;
        let currently_loaded = self.is_loaded(&label)?;
        if self.load == currently_loaded {
            tracing::info!("macos.service {} already in desired state, skipping", label);
            return Ok(());
        }
        let subcommand = if self.load { "load" } else { "unload" };
        self.run_launchctl(subcommand)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn make_service(load: bool) -> Service {
        Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: Some("com.example.test".to_string()),
            load,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        }
    }

    fn write_mock_launchctl(
        mock_dir: &std::path::Path,
        calls_file: &std::path::Path,
        list_exit: i32,
    ) {
        use std::os::unix::fs::PermissionsExt;
        let script = mock_dir.join("launchctl");
        let content = format!(
            "#!/usr/bin/env bash\nprintf 'launchctl %s\\n' \"$*\" >> '{}'\nif [[ \"$1\" == \"list\" ]]; then\n  exit {}\nfi\nexit 0\n",
            calls_file.display(),
            list_exit
        );
        std::fs::write(&script, &content).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn write_mock_defaults(
        mock_dir: &std::path::Path,
        calls_file: &std::path::Path,
        label: &str,
        exit_code: i32,
    ) {
        use std::os::unix::fs::PermissionsExt;
        let script = mock_dir.join("defaults");
        let content = format!(
            "#!/usr/bin/env bash\nprintf 'defaults %s\\n' \"$*\" >> '{}'\nprintf '{}\\n'\nexit {}\n",
            calls_file.display(),
            label,
            exit_code
        );
        std::fs::write(&script, &content).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    fn plan_returns_should_run_true() {
        let atom = make_service(true);
        let outcome = atom.plan().unwrap();
        assert!(outcome.should_run);
        assert!(outcome.side_effects.is_empty());
    }

    #[test]
    fn display_load_format() {
        let atom = make_service(true);
        let s = format!("{atom}");
        assert!(s.contains("load"), "expected 'load' in: {s}");
        assert!(s.contains("/tmp/test.plist"), "expected plist in: {s}");
    }

    #[test]
    fn display_unload_format() {
        let atom = make_service(false);
        let s = format!("{atom}");
        assert!(s.contains("unload"), "expected 'unload' in: {s}");
    }

    #[test]
    #[serial]
    fn execute_skips_when_already_loaded() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_launchctl(mock_dir.path(), &calls_file, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: Some("com.example.test".to_string()),
            load: true,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("load -w"),
            "expected no load call in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_skips_when_already_unloaded() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_launchctl(mock_dir.path(), &calls_file, 1);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: Some("com.example.test".to_string()),
            load: false,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("unload -w"),
            "expected no unload call in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_loads_when_unloaded() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_launchctl(mock_dir.path(), &calls_file, 1);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: Some("com.example.test".to_string()),
            load: true,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("load -w"),
            "expected 'load -w' in log, got: {log}"
        );
        assert!(
            log.contains("/tmp/test.plist"),
            "expected plist path in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_unloads_when_loaded() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_launchctl(mock_dir.path(), &calls_file, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: Some("com.example.test".to_string()),
            load: false,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("unload -w"),
            "expected 'unload -w' in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_uses_explicit_label_without_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_launchctl(mock_dir.path(), &calls_file, 0);
        write_mock_defaults(mock_dir.path(), &calls_file, "com.example.test", 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: Some("com.example.test".to_string()),
            load: true,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("defaults"),
            "expected defaults not called when label is explicit, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_calls_defaults_when_label_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_launchctl(mock_dir.path(), &calls_file, 0);
        write_mock_defaults(mock_dir.path(), &calls_file, "com.example.test", 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: None,
            load: true,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("defaults"),
            "expected defaults to be called when label is None, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_errors_if_defaults_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        write_mock_defaults(mock_dir.path(), &calls_file, "", 1);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: None,
            load: true,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_err(), "expected Err from defaults failure, got Ok");
    }

    #[test]
    #[serial]
    fn execute_errors_if_launchctl_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");

        use std::os::unix::fs::PermissionsExt;
        let script = mock_dir.join("launchctl");
        let content = format!(
            "#!/usr/bin/env bash\nprintf 'launchctl %s\\n' \"$*\" >> '{}'\nexit 1\n",
            calls_file.display()
        );
        std::fs::write(&script, &content).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", mock_dir.path().display(), original_path));

        let mut atom = Service {
            plist: PathBuf::from("/tmp/test.plist"),
            label: Some("com.example.test".to_string()),
            load: true,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        };
        let result = atom.execute();
        std::env::set_var("PATH", &original_path);

        assert!(result.is_err(), "expected Err from launchctl failure, got Ok");
    }
}
```

- [ ] **Step 8: Run all atom tests to verify they pass**

```bash
cd lib && cargo nextest run atoms::macos
```

Expected: all 11 tests pass.

- [ ] **Step 9: Commit**

```bash
git add lib/src/atoms/macos/mod.rs lib/src/atoms/macos/service.rs lib/src/atoms/mod.rs
git commit -m "feat(macos-service): add MacOSService atom with idempotent load/unload"
```

---

## Task 2: MacOSService Action + Enum Wiring

**Files:**

- Create: `lib/src/actions/macos/service.rs`
- Modify: `lib/src/actions/macos/mod.rs`
- Modify: `lib/src/actions/mod.rs`

- [ ] **Step 1: Write the failing deserialization test**

Create `lib/src/actions/macos/service.rs`:

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::utilities;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MacOSServiceState {
    Loaded,
    Unloaded,
}

#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacOSService {
    pub plist: String,
    pub label: Option<String>,
    pub state: MacOSServiceState,
    #[serde(default)]
    pub privileged: bool,
}

impl Action for MacOSService {
    fn summarize(&self) -> String {
        let action = match self.state {
            MacOSServiceState::Loaded => "load",
            MacOSServiceState::Unloaded => "unload",
        };
        format!("{} service {}", action, self.plist)
    }

    fn plan(&self, _: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized_loaded() {
        let yaml = r#"
- action: macos.service
  plist: /System/Library/LaunchDaemons/com.apple.ssh.plist
  state: loaded
  privileged: true
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSService(action)) => {
                assert_eq!(
                    "/System/Library/LaunchDaemons/com.apple.ssh.plist",
                    action.action.plist
                );
                assert_eq!(MacOSServiceState::Loaded, action.action.state);
                assert!(action.action.privileged);
                assert!(action.action.label.is_none());
            }
            _ => panic!("MacOSService didn't deserialize"),
        }
    }

    #[test]
    fn it_can_be_deserialized_unloaded() {
        let yaml = r#"
- action: macos.service
  plist: ~/Library/LaunchAgents/com.myapp.agent.plist
  state: unloaded
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSService(action)) => {
                assert_eq!(
                    "~/Library/LaunchAgents/com.myapp.agent.plist",
                    action.action.plist
                );
                assert_eq!(MacOSServiceState::Unloaded, action.action.state);
                assert!(!action.action.privileged);
            }
            _ => panic!("MacOSService didn't deserialize"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_explicit_label() {
        let yaml = r#"
- action: macos.service
  plist: /Library/LaunchDaemons/com.myapp.daemon.plist
  label: com.myapp.daemon
  state: loaded
  privileged: true
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSService(action)) => {
                assert_eq!(
                    Some("com.myapp.daemon".to_string()),
                    action.action.label
                );
            }
            _ => panic!("MacOSService didn't deserialize"),
        }
    }

    #[test]
    fn privileged_defaults_to_false() {
        let yaml = r#"
- action: macos.service
  plist: /tmp/test.plist
  state: loaded
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::MacOSService(action)) => {
                assert!(!action.action.privileged);
            }
            _ => panic!("MacOSService didn't deserialize"),
        }
    }
}
```

- [ ] **Step 2: Wire MacOSService into actions/macos/mod.rs**

In `lib/src/actions/macos/mod.rs`, replace the current content:

```rust
mod default;
mod service;
pub use default::MacOSDefault;
pub use service::{MacOSService, MacOSServiceState};
```

- [ ] **Step 3: Wire MacOSService into the Actions enum**

In `lib/src/actions/mod.rs`:

**a)** Change the import line (currently line 16):

```rust
use crate::actions::macos::MacOSDefault;
```

to:

```rust
use crate::actions::macos::{MacOSDefault, MacOSService};
```

**b)** Add to the `Actions` enum, after the `MacOSDefault` variant (after line 190):

```rust
    #[serde(rename = "macos.service")]
    MacOSService(ConditionalVariantAction<MacOSService>),
```

**c)** Add to `inner_ref()` match arms, after `Actions::MacOSDefault(a) => a,`:

```rust
            Actions::MacOSService(a) => a,
```

**d)** Add to `Deref::deref()` match arms, after `Actions::MacOSDefault(a) => a,`:

```rust
            Actions::MacOSService(a) => a,
```

**e)** Add to `Display::fmt()` match arms, after `Actions::MacOSDefault(_) => "macos.default",`:

```rust
            Actions::MacOSService(_) => "macos.service",
```

- [ ] **Step 4: Run the deserialization tests to verify they pass**

```bash
cd lib && cargo nextest run actions::macos::service
```

Expected: all 4 deserialization tests PASS (`plan()` has `todo!()` but is not called by these tests).

- [ ] **Step 5: Write the failing plan() tests**

Add to the `tests` module in `lib/src/actions/macos/service.rs`:

```rust
    #[test]
    fn plan_errors_if_plist_missing() {
        let action = MacOSService {
            plist: "/nonexistent/path/test.plist".to_string(),
            label: None,
            state: MacOSServiceState::Loaded,
            privileged: false,
        };
        assert!(action
            .plan(&Manifest::default(), &Contexts::default())
            .is_err());
    }

    #[test]
    fn plan_returns_one_step_when_plist_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let plist = tmp.path().join("test.plist");
        std::fs::write(
            &plist,
            "<?xml version=\"1.0\"?><plist><dict><key>Label</key><string>test</string></dict></plist>",
        )
        .unwrap();
        let action = MacOSService {
            plist: plist.to_str().unwrap().to_string(),
            label: Some("com.example.test".to_string()),
            state: MacOSServiceState::Loaded,
            privileged: false,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_step_display_contains_load() {
        let tmp = tempfile::tempdir().unwrap();
        let plist = tmp.path().join("test.plist");
        std::fs::write(&plist, "").unwrap();
        let action = MacOSService {
            plist: plist.to_str().unwrap().to_string(),
            label: Some("com.example.test".to_string()),
            state: MacOSServiceState::Loaded,
            privileged: false,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("load"),
            "expected 'load' in atom display: {display}"
        );
    }

    #[test]
    fn plan_step_display_contains_unload() {
        let tmp = tempfile::tempdir().unwrap();
        let plist = tmp.path().join("test.plist");
        std::fs::write(&plist, "").unwrap();
        let action = MacOSService {
            plist: plist.to_str().unwrap().to_string(),
            label: Some("com.example.test".to_string()),
            state: MacOSServiceState::Unloaded,
            privileged: false,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("unload"),
            "expected 'unload' in atom display: {display}"
        );
    }
```

Add these imports to the `tests` module (after `use super::*;` and `use crate::actions::Actions;`):

```rust
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;
```

- [ ] **Step 6: Run plan tests to verify they fail**

```bash
cd lib && cargo nextest run actions::macos::service
```

Expected: deserialization tests PASS. `plan_errors_if_plist_missing` and `plan_returns_one_step_when_plist_exists` FAIL with `not yet implemented`.

- [ ] **Step 7: Implement plan() in the action**

Replace the `plan` method body:

```rust
    fn plan(&self, _: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        let expanded = shellexpand::tilde(&self.plist).to_string();
        let path = PathBuf::from(&expanded);
        if !path.exists() {
            anyhow::bail!("plist file does not exist: {}", expanded);
        }
        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());
        let load = matches!(self.state, MacOSServiceState::Loaded);
        Ok(vec![Step {
            atom: Box::new(crate::atoms::macos::Service {
                plist: path,
                label: self.label.clone(),
                load,
                privileged: self.privileged,
                privilege_provider,
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
```

No extra `use` statement needed — call `shellexpand::tilde(...)` directly. The `shellexpand` crate is already in `lib/Cargo.toml` and accessible by crate path without a `use` declaration (same pattern as `lib/src/manifests/providers/local.rs`).

- [ ] **Step 8: Run all action tests to verify they pass**

```bash
cd lib && cargo nextest run actions::macos::service
```

Expected: all tests pass (deserialization tests + plan tests).

- [ ] **Step 9: Run the full test suite**

```bash
cd lib && cargo nextest run
```

Expected: all tests pass. If any compile errors appear in `actions/mod.rs`, verify all three match arms (`inner_ref`, `Deref`, `Display`) include `Actions::MacOSService`.

- [ ] **Step 10: Commit**

```bash
git add lib/src/actions/macos/service.rs lib/src/actions/macos/mod.rs lib/src/actions/mod.rs
git commit -m "feat(macos-service): add MacOSService action and wire into Actions enum"
```

---

## Task 3: Example File and CLAUDE.md

**Files:**

- Create: `examples/macos/service.yaml`
- Modify: `CLAUDE.md`

- [ ] **Step 1: Create the example directory and file**

```bash
mkdir -p examples/macos
```

Create `examples/macos/service.yaml`:

```yaml
actions:
    # Enable the SSH server daemon (requires privileged: true for system daemons).
    # The label is automatically extracted from the plist when label: is omitted.
    - action: macos.service
      plist: /System/Library/LaunchDaemons/com.apple.ssh.plist
      state: loaded
      privileged: true

    # Enable Apple Remote Desktop (ARD).
    - action: macos.service
      plist: /System/Library/LaunchDaemons/com.apple.RemoteDesktop.PrivilegeProxy.plist
      state: loaded
      privileged: true

    # Provide an explicit label to skip plist label extraction.
    # Use this when defaults read is unavailable or the plist is protected.
    - action: macos.service
      plist: /Library/LaunchDaemons/com.myapp.daemon.plist
      label: com.myapp.daemon
      state: loaded
      privileged: true

    # User LaunchAgent — no privileged needed.
    # Installs in ~/Library/LaunchAgents, runs in the user session.
    - action: macos.service
      plist: ~/Library/LaunchAgents/com.myapp.agent.plist
      state: loaded

    # Disable a user agent.
    - action: macos.service
      plist: ~/Library/LaunchAgents/com.myapp.agent.plist
      state: unloaded

    # Conditional — only on macOS.
    - action: macos.service
      plist: /System/Library/LaunchDaemons/com.apple.ssh.plist
      state: loaded
      privileged: true
      where: 'os.name == "macos"'
```

- [ ] **Step 2: Add macos.service to the CLAUDE.md action catalog**

In `CLAUDE.md`, find the action catalog table (the table that lists actions like `file.copy`, `git.clone`, `macos.default`). Add a row for `macos.service`:

```
| `macos.service` | Load or unload a macOS LaunchDaemon/LaunchAgent | `plist`, `state`, `label` (opt), `privileged` |
```

The exact position in the table: add after the `macos.default` row to keep macos.\* grouped.

- [ ] **Step 3: Run the full test suite to confirm no regressions**

```bash
cd lib && cargo nextest run
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add examples/macos/service.yaml CLAUDE.md
git commit -m "docs(macos-service): add example file and action catalog entry"
```

---

## Post-Merge Docs Update (do on main AFTER the PR merges — not inside the worktree)

After the PR merges, on the main branch:

- [ ] Mark the plan as Done in `docs/superpowers/README.md` (change `Pending` to `Done` for the `macos-service` row and add the plan file link)
- [ ] Add `> **Status: DONE**` banner at the top of this plan file
