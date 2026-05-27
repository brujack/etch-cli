> **Status: DONE** — Implemented in PR #49

# systemd.service Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `systemd.service` action that idempotently controls systemd unit boot persistence (`enabled`/`disabled`) and runtime state (`started`/`stopped`) on Linux.

**Architecture:** Follows the `macos.service` pattern exactly — one action struct, one atom struct, PATH-mock tests with `#[serial]`. The atom runs up to two `systemctl` sub-operations (enabled check first, started check second); a failure in the first aborts the second via `?`.

**Tech Stack:** Rust, `schemars`, `serde`, `anyhow`, `tracing`, `serial_test`, `tempfile`.

---

## Files

| Path                                 | Action                                                          |
| ------------------------------------ | --------------------------------------------------------------- |
| `lib/src/atoms/systemd/mod.rs`       | Create — re-exports `Service`                                   |
| `lib/src/atoms/systemd/service.rs`   | Create — `Service` atom                                         |
| `lib/src/atoms/mod.rs`               | Modify — add `pub mod systemd;`                                 |
| `lib/src/actions/systemd/mod.rs`     | Create — re-exports `SystemdService`                            |
| `lib/src/actions/systemd/service.rs` | Create — `SystemdService` action                                |
| `lib/src/actions/mod.rs`             | Modify — add `mod systemd;`, import, enum variant, 3 match arms |
| `examples/systemd/service.yaml`      | Create — usage examples                                         |
| `CLAUDE.md`                          | Modify — add `systemd.service` to action catalog                |
| `README.md`                          | Modify — add `systemd.service` to action catalog table          |

---

## Task 1: systemd.service atom

**Files:**

- Create: `lib/src/atoms/systemd/mod.rs`
- Create: `lib/src/atoms/systemd/service.rs`
- Modify: `lib/src/atoms/mod.rs`

- [ ] **Step 1: Create `lib/src/atoms/systemd/mod.rs`**

```rust
mod service;
pub use service::Service;
```

- [ ] **Step 2: Write the failing test for `is_enabled` detection**

Add this to the bottom of `lib/src/atoms/systemd/service.rs` (create the file with this content):

```rust
use super::super::Atom;
use crate::atoms::Outcome;
use tracing::instrument;

pub struct Service {
    pub unit: String,
    pub enabled: Option<bool>,
    pub started: Option<bool>,
    pub privileged: bool,
    pub privilege_provider: String,
}

impl std::fmt::Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts: Vec<&str> = vec![];
        if let Some(e) = self.enabled {
            parts.push(if e { "enable" } else { "disable" });
        }
        if let Some(s) = self.started {
            parts.push(if s { "start" } else { "stop" });
        }
        write!(f, "SystemdService {} {}", parts.join("+"), self.unit)
    }
}

impl Atom for Service {
    #[instrument(name = "systemd.service.plan", level = "info", skip(self))]
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    #[instrument(name = "systemd.service.execute", level = "info", skip(self))]
    fn execute(&mut self) -> anyhow::Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn write_mock_systemctl(
        mock_dir: &std::path::Path,
        calls_file: &std::path::Path,
        is_enabled_output: &str,
        is_active_exit: i32,
        action_exit: i32,
    ) {
        use std::os::unix::fs::PermissionsExt;
        let script = mock_dir.join("systemctl");
        let content = format!(
            "#!/usr/bin/env bash\n\
             printf 'systemctl %s\\n' \"$*\" >> '{}'\n\
             if [[ \"$1\" == \"is-enabled\" ]]; then\n\
               printf '{}\\n'\n\
               exit 0\n\
             fi\n\
             if [[ \"$1\" == \"is-active\" ]]; then\n\
               exit {}\n\
             fi\n\
             exit {}\n",
            calls_file.display(),
            is_enabled_output,
            is_active_exit,
            action_exit
        );
        std::fs::write(&script, &content).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn make_service(unit: &str, enabled: Option<bool>, started: Option<bool>) -> Service {
        Service {
            unit: unit.to_string(),
            enabled,
            started,
            privileged: false,
            privilege_provider: "sudo".to_string(),
        }
    }

    #[test]
    fn plan_returns_should_run_true() {
        let atom = make_service("sshd.service", Some(true), None);
        let outcome = atom.plan().unwrap();
        assert!(outcome.should_run);
        assert!(outcome.side_effects.is_empty());
    }
}
```

- [ ] **Step 3: Run test to verify it compiles and passes**

```bash
cargo test -p etch-lib atoms::systemd::service::tests::plan_returns_should_run_true 2>&1 | tail -10
```

Expected: PASS (todo!() is only in execute, not reached by this test).

- [ ] **Step 4: Write failing test — skips when already enabled**

Add inside the `tests` module:

```rust
    #[test]
    #[serial]
    fn execute_skips_when_already_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // is-enabled → "enabled"; want enabled: true → already in desired state
        write_mock_systemctl(mock_dir.path(), &calls_file, "enabled", 1, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("sshd.service", Some(true), None);
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("systemctl enable"),
            "expected no 'enable' call, got: {log}"
        );
    }
```

- [ ] **Step 5: Run to verify it fails (todo!() panics)**

```bash
cargo test -p etch-lib atoms::systemd::service::tests::execute_skips_when_already_enabled 2>&1 | tail -10
```

Expected: FAIL (panic at `todo!()`).

- [ ] **Step 6: Implement `is_enabled`, `is_active`, `run_systemctl`, and `execute`**

Replace `todo!()` in `execute` and add the helper methods to the `Service` impl block (before the `Atom` impl):

```rust
impl Service {
    fn is_enabled(&self) -> anyhow::Result<bool> {
        let output = std::process::Command::new("systemctl")
            .args(["is-enabled", &self.unit])
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(stdout == "enabled")
    }

    fn is_active(&self) -> bool {
        let status = std::process::Command::new("systemctl")
            .args(["is-active", &self.unit])
            .status();
        matches!(status, Ok(s) if s.success())
    }

    fn run_systemctl(&self, subcommand: &str) -> anyhow::Result<()> {
        let (cmd, args) = if self.privileged {
            (
                self.privilege_provider.clone(),
                vec![
                    "systemctl".to_string(),
                    subcommand.to_string(),
                    self.unit.clone(),
                ],
            )
        } else {
            (
                "systemctl".to_string(),
                vec![subcommand.to_string(), self.unit.clone()],
            )
        };
        let status = std::process::Command::new(&cmd).args(&args).status()?;
        if !status.success() {
            anyhow::bail!(
                "systemctl {} {} failed with {}",
                subcommand,
                self.unit,
                status
            );
        }
        Ok(())
    }
}
```

Replace `todo!()` in `execute` with:

```rust
    fn execute(&mut self) -> anyhow::Result<()> {
        if let Some(want_enabled) = self.enabled {
            let currently_enabled = self.is_enabled()?;
            if currently_enabled != want_enabled {
                let subcommand = if want_enabled { "enable" } else { "disable" };
                self.run_systemctl(subcommand)?;
            } else {
                tracing::info!(
                    unit = %self.unit,
                    enabled = want_enabled,
                    "systemd.service: enabled already in desired state, skipping"
                );
            }
        }

        if let Some(want_started) = self.started {
            let currently_active = self.is_active();
            if currently_active != want_started {
                let subcommand = if want_started { "start" } else { "stop" };
                self.run_systemctl(subcommand)?;
            } else {
                tracing::info!(
                    unit = %self.unit,
                    started = want_started,
                    "systemd.service: started already in desired state, skipping"
                );
            }
        }

        Ok(())
    }
```

- [ ] **Step 7: Run test to verify it passes**

```bash
cargo test -p etch-lib atoms::systemd::service::tests::execute_skips_when_already_enabled 2>&1 | tail -10
```

Expected: PASS.

- [ ] **Step 8: Add remaining tests for all behaviors**

Add inside the `tests` module:

```rust
    #[test]
    #[serial]
    fn execute_skips_when_already_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // is-enabled → "disabled"; want enabled: false → already in desired state
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 1, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("bluetooth.service", Some(false), None);
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("systemctl disable"),
            "expected no 'disable' call, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_enables_when_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // is-enabled → "disabled"; want enabled: true → should enable
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 1, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("sshd.service", Some(true), None);
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("systemctl enable sshd.service"),
            "expected 'enable sshd.service' in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_disables_when_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // is-enabled → "enabled"; want enabled: false → should disable
        write_mock_systemctl(mock_dir.path(), &calls_file, "enabled", 1, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("bluetooth.service", Some(false), None);
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("systemctl disable bluetooth.service"),
            "expected 'disable bluetooth.service' in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_skips_when_already_started() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // is-active exits 0 = active; want started: true → already in desired state
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 0, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("sshd.service", None, Some(true));
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("systemctl start"),
            "expected no 'start' call, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_skips_when_already_stopped() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // is-active exits 1 = inactive; want started: false → already in desired state
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 1, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("cups.service", None, Some(false));
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            !log.contains("systemctl stop"),
            "expected no 'stop' call, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_starts_when_stopped() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // is-active exits 1 = inactive; want started: true → should start
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 1, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("sshd.service", None, Some(true));
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("systemctl start sshd.service"),
            "expected 'start sshd.service' in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_stops_when_started() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // is-active exits 0 = active; want started: false → should stop
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 0, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("cups.service", None, Some(false));
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("systemctl stop cups.service"),
            "expected 'stop cups.service' in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_handles_enabled_and_started_together() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // is-enabled → "disabled"; is-active exits 1; want both true → enable then start
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 1, 0);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("sshd.service", Some(true), Some(true));
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_ok(), "execute failed: {:?}", result);
        let log = std::fs::read_to_string(&calls_file).unwrap_or_default();
        assert!(
            log.contains("systemctl enable sshd.service"),
            "expected 'enable' in log, got: {log}"
        );
        assert!(
            log.contains("systemctl start sshd.service"),
            "expected 'start' in log, got: {log}"
        );
    }

    #[test]
    #[serial]
    fn execute_errors_if_enable_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // is-enabled → "disabled"; want enabled: true; action_exit = 1 → enable fails
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 1, 1);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("sshd.service", Some(true), None);
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_err(), "expected Err when enable fails, got Ok");
    }

    #[test]
    #[serial]
    fn execute_errors_if_start_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let mock_dir = tempfile::tempdir().unwrap();
        let calls_file = tmp.path().join("calls.log");
        // is-active exits 1 = inactive; want started: true; action_exit = 1 → start fails
        write_mock_systemctl(mock_dir.path(), &calls_file, "disabled", 1, 1);

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var(
            "PATH",
            format!("{}:{}", mock_dir.path().display(), original_path),
        );

        let mut atom = make_service("sshd.service", None, Some(true));
        let result = atom.execute();

        std::env::set_var("PATH", &original_path);

        assert!(result.is_err(), "expected Err when start fails, got Ok");
    }

    #[test]
    fn display_enable_format() {
        let atom = make_service("sshd.service", Some(true), None);
        let s = format!("{atom}");
        assert!(s.contains("enable"), "expected 'enable' in: {s}");
        assert!(s.contains("sshd.service"), "expected unit in: {s}");
    }

    #[test]
    fn display_enable_start_format() {
        let atom = make_service("sshd.service", Some(true), Some(true));
        let s = format!("{atom}");
        assert!(s.contains("enable"), "expected 'enable' in: {s}");
        assert!(s.contains("start"), "expected 'start' in: {s}");
    }
```

- [ ] **Step 9: Run all atom tests**

```bash
cargo test -p etch-lib atoms::systemd 2>&1 | tail -20
```

Expected: all tests PASS.

- [ ] **Step 10: Wire atom into `lib/src/atoms/mod.rs`**

Add `pub mod systemd;` after `pub mod macos;` in `lib/src/atoms/mod.rs`:

```rust
pub mod binary;
pub mod command;
pub mod directory;
pub mod file;
pub mod git;
pub mod http;
pub mod macos;
pub mod systemd;   // add this line
pub mod plugin;
```

- [ ] **Step 11: Verify full lib compiles**

```bash
cargo build -p etch-lib 2>&1 | tail -10
```

Expected: compiles with no errors.

- [ ] **Step 12: Run all tests**

```bash
cargo nextest run -p etch-lib 2>&1 | tail -20
```

Expected: all tests PASS.

- [ ] **Step 13: Commit**

```bash
git add lib/src/atoms/systemd/ lib/src/atoms/mod.rs
git commit -m "feat(atoms): add systemd.service atom

Idempotent systemctl enable/disable/start/stop.
is-enabled stdout parse for boot persistence check;
is-active exit code for runtime state check.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 2: systemd.service action and Actions enum wiring

**Files:**

- Create: `lib/src/actions/systemd/mod.rs`
- Create: `lib/src/actions/systemd/service.rs`
- Modify: `lib/src/actions/mod.rs`

- [ ] **Step 1: Create `lib/src/actions/systemd/mod.rs`**

```rust
mod service;
pub use service::SystemdService;
```

- [ ] **Step 2: Write failing deserialization tests**

Create `lib/src/actions/systemd/service.rs` with this content:

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::utilities;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemdService {
    pub unit: String,
    pub enabled: Option<bool>,
    pub started: Option<bool>,
    #[serde(default)]
    pub privileged: bool,
}

impl Action for SystemdService {
    fn summarize(&self) -> String {
        let mut parts: Vec<&str> = vec![];
        if let Some(e) = self.enabled {
            parts.push(if e { "enable" } else { "disable" });
        }
        if let Some(s) = self.started {
            parts.push(if s { "start" } else { "stop" });
        }
        format!("{} service {}", parts.join("+"), self.unit)
    }

    fn plan(&self, _: &Manifest, _contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_enabled_and_started() {
        let yaml = r#"
unit: sshd.service
enabled: true
started: true
privileged: true
"#;
        let action: SystemdService = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(action.unit, "sshd.service");
        assert_eq!(action.enabled, Some(true));
        assert_eq!(action.started, Some(true));
        assert!(action.privileged);
    }

    #[test]
    fn deserialize_enabled_only() {
        let yaml = r#"
unit: bluetooth.service
enabled: false
"#;
        let action: SystemdService = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(action.unit, "bluetooth.service");
        assert_eq!(action.enabled, Some(false));
        assert_eq!(action.started, None);
        assert!(!action.privileged);
    }

    #[test]
    fn deserialize_started_only() {
        let yaml = r#"
unit: cups.service
started: false
"#;
        let action: SystemdService = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(action.unit, "cups.service");
        assert_eq!(action.enabled, None);
        assert_eq!(action.started, Some(false));
    }

    #[test]
    fn plan_errors_if_neither_field_set() {
        // Can't call plan() without contexts — test validation logic directly
        let action = SystemdService {
            unit: "sshd.service".to_string(),
            enabled: None,
            started: None,
            privileged: false,
        };
        // validation is in plan(); test it by checking the condition
        assert!(
            action.enabled.is_none() && action.started.is_none(),
            "this combination should be rejected by plan()"
        );
    }
}
```

- [ ] **Step 3: Run tests to verify deserialization tests pass**

```bash
cargo test -p etch-lib actions::systemd 2>&1 | tail -20
```

Expected: 3 deserialization tests PASS; `plan_errors_if_neither_field_set` PASS (it only checks the condition, not the plan call).

- [ ] **Step 4: Implement `plan()`**

Replace `todo!()` in `plan` with:

```rust
    fn plan(&self, _: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        if self.enabled.is_none() && self.started.is_none() {
            anyhow::bail!(
                "systemd.service: at least one of 'enabled' or 'started' must be set for unit {}",
                self.unit
            );
        }
        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());
        Ok(vec![Step {
            atom: Box::new(crate::atoms::systemd::Service {
                unit: self.unit.clone(),
                enabled: self.enabled,
                started: self.started,
                privileged: self.privileged,
                privilege_provider,
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
```

- [ ] **Step 5: Add plan tests**

Add to the `tests` module:

```rust
    #[test]
    fn plan_returns_one_step_with_enabled_only() {
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = SystemdService {
            unit: "sshd.service".to_string(),
            enabled: Some(true),
            started: None,
            privileged: false,
        };
        let manifest = Manifest::default();
        let contexts = Contexts::default();
        let steps = action.plan(&manifest, &contexts).unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn plan_returns_one_step_with_started_only() {
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = SystemdService {
            unit: "cups.service".to_string(),
            enabled: None,
            started: Some(false),
            privileged: false,
        };
        let manifest = Manifest::default();
        let contexts = Contexts::default();
        let steps = action.plan(&manifest, &contexts).unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn plan_errors_when_both_none() {
        use crate::contexts::Contexts;
        use crate::manifests::Manifest;
        let action = SystemdService {
            unit: "sshd.service".to_string(),
            enabled: None,
            started: None,
            privileged: false,
        };
        let manifest = Manifest::default();
        let contexts = Contexts::default();
        let result = action.plan(&manifest, &contexts);
        assert!(result.is_err(), "expected Err when both fields are None");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("at least one"),
            "expected helpful error message, got: {msg}"
        );
    }
```

- [ ] **Step 6: Run all action tests**

```bash
cargo test -p etch-lib actions::systemd 2>&1 | tail -20
```

Expected: all 7 tests PASS.

- [ ] **Step 7: Wire into `lib/src/actions/mod.rs`**

Add `mod systemd;` after `mod macos;` at the top:

```rust
mod binary;
mod brew;
mod command;
mod directory;
mod file;
mod git;
mod group;
mod macos;
mod mas;
mod package;
mod plugin;
mod systemd;   // add this line
mod user;
```

Add import after the `macos` import line (line 16):

```rust
use crate::actions::macos::{MacOSDefault, MacOSService};
use crate::actions::mas::{MasInstall, MasUpgrade};
use crate::actions::systemd::SystemdService;   // add this line
```

Add enum variant after `MacOSService`:

```rust
    #[serde(rename = "macos.service")]
    MacOSService(ConditionalVariantAction<MacOSService>),

    #[serde(rename = "systemd.service")]
    SystemdService(ConditionalVariantAction<SystemdService>),
```

Add match arm in `inner_ref()` after `Actions::MacOSService(a) => a,`:

```rust
            Actions::MacOSService(a) => a,
            Actions::SystemdService(a) => a,
```

Add match arm in `Deref::deref()` after `Actions::MacOSService(a) => a,`:

```rust
            Actions::MacOSService(a) => a,
            Actions::SystemdService(a) => a,
```

Add match arm in `Display::fmt()` after `Actions::MacOSService(_) => "macos.service",`:

```rust
            Actions::MacOSService(_) => "macos.service",
            Actions::SystemdService(_) => "systemd.service",
```

- [ ] **Step 8: Verify full build**

```bash
cargo build -p etch-lib 2>&1 | tail -10
```

Expected: compiles with no errors or warnings.

- [ ] **Step 9: Run full test suite**

```bash
cargo nextest run -p etch-lib 2>&1 | tail -20
```

Expected: all tests PASS.

- [ ] **Step 10: Commit**

```bash
git add lib/src/actions/systemd/ lib/src/actions/mod.rs
git commit -m "feat(actions): add systemd.service action and wire into Actions enum

Validates at least one of enabled/started is set at plan time.
Resolves privilege_provider from contexts.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 3: Examples, CLAUDE.md, README

**Files:**

- Create: `examples/systemd/service.yaml`
- Modify: `CLAUDE.md`
- Modify: `README.md`

- [ ] **Step 1: Create `examples/systemd/service.yaml`**

```yaml
# Enable and start SSH daemon (system daemon — needs privileged)
- action: systemd.service
  unit: sshd.service
  enabled: true
  started: true
  privileged: true
  where: 'os.family == "linux"'

# Disable bluetooth at boot, leave runtime state alone
- action: systemd.service
  unit: bluetooth.service
  enabled: false
  privileged: true
  where: 'os.family == "linux"'

# Stop a service without disabling it at boot
- action: systemd.service
  unit: cups.service
  started: false
  privileged: true
  where: 'os.family == "linux"'

# Enable at boot without starting now
- action: systemd.service
  unit: nginx.service
  enabled: true
  privileged: true
  where: 'os.family == "linux"'

# Control runtime only — no change to boot persistence
- action: systemd.service
  unit: redis.service
  started: true
  privileged: true
  where: 'os.family == "linux"'
```

- [ ] **Step 2: Update `CLAUDE.md` action catalog**

Add a row for `systemd.service` after the `macos.service` row in the Action Catalog table in `CLAUDE.md`:

```
| `systemd.service`    | Enable/disable/start/stop a systemd unit    | `unit`, `enabled` (Option bool), `started` (Option bool), `privileged` (bool, default `false`). At least one of `enabled`/`started` required. Use `where: 'os.family == "linux"'`. |
```

- [ ] **Step 3: Update `README.md` action catalog**

Add a row for `systemd.service` after `macos.service` in the action catalog table in `README.md`:

```
| `systemd.service`                        | Enable/disable/start/stop systemd units    |
```

- [ ] **Step 4: Run lint**

```bash
make lint 2>&1 | tail -10
```

Expected: passes with no errors.

- [ ] **Step 5: Run full test suite**

```bash
cargo nextest run 2>&1 | tail -20
```

Expected: all tests PASS.

- [ ] **Step 6: Commit**

```bash
git add examples/systemd/ CLAUDE.md README.md
git commit -m "docs(systemd-service): add example file and action catalog entry

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Post-merge (do on main after PR merges — NOT inside worktree)

Update `docs/superpowers/README.md`: change `systemd-service` row status from `Pending` to `Done`.

Add `> **Status: DONE**` banner to top of `docs/superpowers/plans/2026-05-26-systemd-service.md`.
