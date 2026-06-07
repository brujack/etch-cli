> **Status: DONE**

# etch doctor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `etch doctor` subcommand that checks system health: symlink integrity, tools in PATH, credential dir permissions, and binary version drift.

**Architecture:** Four check implementations in `lib/src/doctor/` (testable in isolation via a `DoctorCheck` trait); command in `app/src/commands/doctor.rs` wires them together and handles output/exit code. `Config` gains an optional `doctor:` section for explicit tool lists, version pins, and credential dirs.

**Tech Stack:** Rust, clap, colored, serde/serde_json, shellexpand, which, std::process::Command

---

## File Map

| File                           | Change                                                                       |
| ------------------------------ | ---------------------------------------------------------------------------- |
| `lib/src/config/mod.rs`        | Add `DoctorConfig`, `VersionPin`, `doctor: Option<DoctorConfig>` to `Config` |
| `lib/src/doctor/mod.rs`        | Create — `CheckResult`, `DoctorCheck` trait, submodule declarations          |
| `lib/src/doctor/symlinks.rs`   | Create — `SymlinkCheck`                                                      |
| `lib/src/doctor/tools.rs`      | Create — `ToolsCheck`                                                        |
| `lib/src/doctor/cred_perms.rs` | Create — `CredPermsCheck`                                                    |
| `lib/src/doctor/versions.rs`   | Create — `VersionsCheck`                                                     |
| `lib/src/lib.rs`               | Add `pub mod doctor;`                                                        |
| `app/src/commands/doctor.rs`   | Create — `Doctor` struct, `run_doctor_checks`, `EtchCommand` impl            |
| `app/src/commands/mod.rs`      | Add `mod doctor; pub(crate) use doctor::Doctor;`                             |
| `app/src/config/mod.rs`        | Add `Doctor(commands::Doctor)` to `Commands` enum                            |
| `app/src/main.rs`              | Add `Commands::Doctor(d) => d.execute(&runtime)` dispatch                    |
| `app/tests/doctor.rs`          | Create — integration tests                                                   |
| `app/tests/snapshots.rs`       | Add `etch doctor --help` snapshot                                            |
| `examples/doctor/doctor.yaml`  | Create — example manifest with doctor config                                 |

---

## Task 1: Add `DoctorConfig` to `lib/src/config/mod.rs`

**Files:**

- Modify: `lib/src/config/mod.rs`

- [ ] **Step 1: Write failing tests**

Add to the `#[cfg(test)] mod tests` block in `lib/src/config/mod.rs`:

```rust
#[test]
fn doctor_config_default_has_empty_collections() {
    let c = DoctorConfig::default();
    assert!(c.tools.is_empty());
    assert!(c.versions.is_empty());
    assert!(c.credential_dirs.is_empty());
}

#[test]
fn config_deserializes_doctor_section() {
    let yaml = r#"
doctor:
    tools:
        - kubectl
        - helm
    versions:
        - tool: ripgrep
          command: "rg --version"
          expected: "14.1.0"
    credential_dirs:
        - ~/.ssh
        - ~/.tf_creds
"#;
    let config: Config = serde_yaml_ng::from_str(yaml).unwrap();
    let doctor = config.doctor.unwrap();
    assert_eq!(doctor.tools, vec!["kubectl", "helm"]);
    assert_eq!(doctor.versions.len(), 1);
    assert_eq!(doctor.versions[0].tool, "ripgrep");
    assert_eq!(doctor.versions[0].command, "rg --version");
    assert_eq!(doctor.versions[0].expected, "14.1.0");
    assert_eq!(doctor.credential_dirs, vec!["~/.ssh", "~/.tf_creds"]);
}

#[test]
fn config_doctor_is_none_when_absent() {
    let config: Config = serde_yaml_ng::from_str("").unwrap();
    assert!(config.doctor.is_none());
}
```

- [ ] **Step 2: Run to confirm fail**

```bash
cargo test -p etch-lib config::tests 2>&1 | grep -E "FAILED|error" | head -10
```

Expected: compile error — `DoctorConfig` not defined.

- [ ] **Step 3: Implement**

Add to `lib/src/config/mod.rs` after the `ClaudeUpdateConfig` block:

```rust
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DoctorConfig {
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub versions: Vec<VersionPin>,
    #[serde(default)]
    pub credential_dirs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VersionPin {
    pub tool: String,
    pub command: String,
    pub expected: String,
}
```

Add to the `Config` struct (after `pub privilege: Privilege,`):

```rust
    #[serde(default)]
    pub doctor: Option<DoctorConfig>,
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p etch-lib config::tests 2>&1 | grep -E "test.*ok|test.*FAILED|FAILED" | head -20
```

Expected: all config tests pass.

- [ ] **Step 5: Commit**

```bash
git add lib/src/config/mod.rs
git commit -m "feat(config): add DoctorConfig and VersionPin structs

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 2: Create `lib/src/doctor/mod.rs` and register module

**Files:**

- Create: `lib/src/doctor/mod.rs`
- Modify: `lib/src/lib.rs`

- [ ] **Step 1: Create `lib/src/doctor/mod.rs`**

```rust
use crate::config::Config;
use crate::manifests::Manifest;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub label: String,
    pub passed: bool,
    pub detail: Option<String>,
}

pub trait DoctorCheck {
    fn name(&self) -> &'static str;
    fn run(&self, config: &Config, manifests: &HashMap<String, Manifest>) -> Vec<CheckResult>;
}

pub mod cred_perms;
pub mod symlinks;
pub mod tools;
pub mod versions;
```

- [ ] **Step 2: Register in `lib/src/lib.rs`**

Add after `pub mod config;`:

```rust
pub mod doctor;
```

- [ ] **Step 3: Create stub files so `pub mod` compiles**

Create `lib/src/doctor/symlinks.rs`:

```rust
use super::{CheckResult, DoctorCheck};
use crate::config::Config;
use crate::manifests::Manifest;
use std::collections::HashMap;

pub struct SymlinkCheck;
impl DoctorCheck for SymlinkCheck {
    fn name(&self) -> &'static str { "Symlinks" }
    fn run(&self, _: &Config, _: &HashMap<String, Manifest>) -> Vec<CheckResult> { vec![] }
}
```

Create `lib/src/doctor/tools.rs`:

```rust
use super::{CheckResult, DoctorCheck};
use crate::config::Config;
use crate::manifests::Manifest;
use std::collections::HashMap;

pub struct ToolsCheck;
impl DoctorCheck for ToolsCheck {
    fn name(&self) -> &'static str { "Tools" }
    fn run(&self, _: &Config, _: &HashMap<String, Manifest>) -> Vec<CheckResult> { vec![] }
}
```

Create `lib/src/doctor/cred_perms.rs`:

```rust
use super::{CheckResult, DoctorCheck};
use crate::config::Config;
use crate::manifests::Manifest;
use std::collections::HashMap;

pub struct CredPermsCheck;
impl DoctorCheck for CredPermsCheck {
    fn name(&self) -> &'static str { "Credential dirs" }
    fn run(&self, _: &Config, _: &HashMap<String, Manifest>) -> Vec<CheckResult> { vec![] }
}
```

Create `lib/src/doctor/versions.rs`:

```rust
use super::{CheckResult, DoctorCheck};
use crate::config::Config;
use crate::manifests::Manifest;
use std::collections::HashMap;

pub struct VersionsCheck;
impl DoctorCheck for VersionsCheck {
    fn name(&self) -> &'static str { "Versions" }
    fn run(&self, _: &Config, _: &HashMap<String, Manifest>) -> Vec<CheckResult> { vec![] }
}
```

- [ ] **Step 4: Compile check**

```bash
cargo check -p etch-lib 2>&1 | grep "^error" | head -10
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add lib/src/doctor/ lib/src/lib.rs
git commit -m "feat(doctor): add DoctorCheck trait and module skeleton

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 3: Implement `SymlinkCheck`

**Files:**

- Modify: `lib/src/doctor/symlinks.rs`

- [ ] **Step 1: Write failing tests**

Replace the stub content of `lib/src/doctor/symlinks.rs` with:

```rust
use super::{CheckResult, DoctorCheck};
use crate::actions::Actions;
use crate::config::Config;
use crate::manifests::Manifest;
use std::collections::HashMap;

pub struct SymlinkCheck;

impl DoctorCheck for SymlinkCheck {
    fn name(&self) -> &'static str { "Symlinks" }

    fn run(&self, _config: &Config, manifests: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifests::Manifest;
    use std::collections::HashMap;

    fn manifest_with_link(source: &str, target: &str) -> HashMap<String, Manifest> {
        let yaml = format!(
            concat!(
                "actions:\n",
                "  - action: file.link\n",
                "    source: {source}\n",
                "    target: {target}\n",
            )
        );
        let manifest: Manifest = serde_yaml_ng::from_str(&yaml).unwrap();
        let mut map = HashMap::new();
        map.insert("test".to_string(), manifest);
        map
    }

    #[test]
    fn passes_when_target_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source_file");
        let target = tmp.path().join("symlink");
        std::fs::write(&source, b"content").unwrap();
        std::os::unix::fs::symlink(&source, &target).unwrap();

        let manifests = manifest_with_link(
            &source.display().to_string(),
            &target.display().to_string(),
        );
        let results = SymlinkCheck.run(&Config::default(), &manifests);
        assert_eq!(1, results.len());
        assert!(results[0].passed, "expected pass, got: {:?}", results[0].detail);
    }

    #[test]
    fn fails_when_target_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source_file");
        let target = tmp.path().join("nonexistent_link");

        let manifests = manifest_with_link(
            &source.display().to_string(),
            &target.display().to_string(),
        );
        let results = SymlinkCheck.run(&Config::default(), &manifests);
        assert_eq!(1, results.len());
        assert!(!results[0].passed);
        assert!(
            results[0].detail.as_deref().unwrap_or("").contains("does not exist"),
            "got: {:?}",
            results[0].detail
        );
    }

    #[test]
    fn returns_empty_for_no_file_link_actions() {
        let yaml = "actions:\n  - action: command.run\n    command: echo hi\n";
        let manifest: Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let mut manifests = HashMap::new();
        manifests.insert("test".to_string(), manifest);
        let results = SymlinkCheck.run(&Config::default(), &manifests);
        assert!(results.is_empty());
    }
}
```

- [ ] **Step 2: Run to confirm RED**

```bash
cargo test -p etch-lib doctor::symlinks 2>&1 | grep -E "test.*FAILED|panicked" | head -10
```

Expected: tests fail with "not yet implemented".

- [ ] **Step 3: Implement**

Replace the `todo!()` body:

```rust
    fn run(&self, _config: &Config, manifests: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        let mut results = Vec::new();

        for manifest in manifests.values() {
            for action in &manifest.actions {
                let Actions::FileLink(a) = action else { continue };

                let source = a.action.source.as_deref()
                    .or(a.action.from.as_deref())
                    .unwrap_or("(unknown)");
                let target = a.action.target.as_deref()
                    .or(a.action.to.as_deref());

                let Some(target) = target else { continue };

                let expanded = shellexpand::tilde(target).into_owned();
                let exists = std::path::Path::new(&expanded).exists();

                results.push(CheckResult {
                    label: format!("{source} → {target}"),
                    passed: exists,
                    detail: if exists {
                        None
                    } else {
                        Some(String::from("target does not exist"))
                    },
                });
            }
        }

        results
    }
```

Add `use shellexpand;` at the top of the file.

- [ ] **Step 4: Run tests**

```bash
cargo test -p etch-lib doctor::symlinks 2>&1 | grep -E "test.*ok|test.*FAILED" | head -10
```

Expected: all 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add lib/src/doctor/symlinks.rs
git commit -m "feat(doctor): implement SymlinkCheck

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 4: Implement `ToolsCheck`

**Files:**

- Modify: `lib/src/doctor/tools.rs`

- [ ] **Step 1: Write failing tests**

Replace stub content with:

```rust
use super::{CheckResult, DoctorCheck};
use crate::actions::Actions;
use crate::config::{Config, DoctorConfig};
use crate::manifests::Manifest;
use std::collections::{BTreeSet, HashMap};

pub struct ToolsCheck;

fn action_implied_tool(action: &Actions) -> Option<&'static str> {
    todo!()
}

impl DoctorCheck for ToolsCheck {
    fn name(&self) -> &'static str { "Tools" }

    fn run(&self, config: &Config, manifests: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn config_with_tools(tools: &[&str]) -> Config {
        Config {
            doctor: Some(DoctorConfig {
                tools: tools.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    #[serial]
    fn passes_for_tool_in_path() {
        let tmp = tempfile::tempdir().unwrap();
        let fake = tmp.path().join("mytestbinary_xyz");
        std::fs::write(&fake, b"#!/bin/sh").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old}", tmp.path().display()));

        let config = config_with_tools(&["mytestbinary_xyz"]);
        let results = ToolsCheck.run(&config, &HashMap::new());

        std::env::set_var("PATH", old);

        assert_eq!(1, results.len());
        assert!(results[0].passed, "expected pass for tool in PATH");
    }

    #[test]
    fn fails_for_tool_not_in_path() {
        let config = config_with_tools(&["etch_cli_nonexistent_tool_xyz"]);
        let results = ToolsCheck.run(&config, &HashMap::new());
        assert_eq!(1, results.len());
        assert!(!results[0].passed);
        assert!(
            results[0].detail.as_deref().unwrap_or("").contains("not found"),
            "got: {:?}",
            results[0].detail
        );
    }

    #[test]
    fn explicit_and_manifest_derived_merged_and_deduped() {
        // brew.bundle implies "brew"; also listed explicitly
        let yaml = "actions:\n  - action: brew.bundle\n    file: /tmp/Brewfile\n";
        let manifest: Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let mut manifests = HashMap::new();
        manifests.insert("test".to_string(), manifest);

        let config = config_with_tools(&["brew"]); // explicit duplicate
        let results = ToolsCheck.run(&config, &manifests);

        // Should have exactly one "brew" entry (deduped)
        let brew_results: Vec<_> = results.iter().filter(|r| r.label == "brew").collect();
        assert_eq!(1, brew_results.len(), "expected brew deduped to one entry");
    }

    #[test]
    fn gem_install_implies_gem() {
        let yaml = "actions:\n  - action: gem.install\n    name: bundler\n";
        let manifest: Manifest = serde_yaml_ng::from_str(yaml).unwrap();
        let mut manifests = HashMap::new();
        manifests.insert("test".to_string(), manifest);
        let results = ToolsCheck.run(&Config::default(), &manifests);
        assert!(results.iter().any(|r| r.label == "gem"), "expected gem in results");
    }

    #[test]
    fn empty_manifests_and_empty_config_returns_empty() {
        let results = ToolsCheck.run(&Config::default(), &HashMap::new());
        assert!(results.is_empty());
    }
}
```

- [ ] **Step 2: Run to confirm RED**

```bash
cargo test -p etch-lib doctor::tools 2>&1 | grep -E "FAILED|panicked" | head -10
```

Expected: fails with "not yet implemented".

- [ ] **Step 3: Implement**

Replace the two `todo!()` bodies:

```rust
fn action_implied_tool(action: &Actions) -> Option<&'static str> {
    match action {
        Actions::BrewBundle(_) | Actions::BrewUpgrade(_) | Actions::BrewCleanup(_) => Some("brew"),
        Actions::GemInstall(_) => Some("gem"),
        Actions::PipInstall(_) => Some("pip"),
        Actions::NpmInstall(_) => Some("npm"),
        Actions::MasInstall(_) | Actions::MasUpgrade(_) => Some("mas"),
        Actions::PyenvInstall(_) | Actions::PyenvVirtualenv(_) => Some("pyenv"),
        Actions::RubyInstall(_) => Some("ruby-install"),
        Actions::ClaudeInstall(_)
        | Actions::ClaudeUpgrade(_)
        | Actions::ClaudePluginUpdate(_) => Some("claude"),
        _ => None,
    }
}

impl DoctorCheck for ToolsCheck {
    fn name(&self) -> &'static str { "Tools" }

    fn run(&self, config: &Config, manifests: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        let mut tools: BTreeSet<String> = BTreeSet::new();

        for manifest in manifests.values() {
            for action in &manifest.actions {
                if let Some(tool) = action_implied_tool(action) {
                    tools.insert(tool.to_string());
                }
            }
        }

        if let Some(ref doctor) = config.doctor {
            for tool in &doctor.tools {
                tools.insert(tool.clone());
            }
        }

        tools
            .into_iter()
            .map(|tool| {
                let found = which::which(&tool).is_ok();
                CheckResult {
                    label: tool.clone(),
                    passed: found,
                    detail: if found {
                        None
                    } else {
                        Some(String::from("not found in PATH"))
                    },
                }
            })
            .collect()
    }
}
```

Add `use which;` at the top.

- [ ] **Step 4: Run tests**

```bash
cargo test -p etch-lib doctor::tools 2>&1 | grep -E "test.*ok|test.*FAILED" | head -10
```

Expected: all 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add lib/src/doctor/tools.rs
git commit -m "feat(doctor): implement ToolsCheck

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 5: Implement `CredPermsCheck`

**Files:**

- Modify: `lib/src/doctor/cred_perms.rs`

- [ ] **Step 1: Write failing tests**

Replace stub content with:

```rust
use super::{CheckResult, DoctorCheck};
use crate::config::{Config, DoctorConfig};
use crate::manifests::Manifest;
use std::collections::HashMap;

pub struct CredPermsCheck;

impl DoctorCheck for CredPermsCheck {
    fn name(&self) -> &'static str { "Credential dirs" }

    fn run(&self, config: &Config, _manifests: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn config_with_dirs(dirs: &[&str]) -> Config {
        Config {
            doctor: Some(DoctorConfig {
                credential_dirs: dirs.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn passes_for_dir_with_mode_700() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let dir = tmp.path().display().to_string();

        let results = CredPermsCheck.run(&config_with_dirs(&[&dir]), &HashMap::new());
        assert_eq!(1, results.len());
        assert!(results[0].passed, "expected pass for 700, got: {:?}", results[0].detail);
    }

    #[test]
    fn fails_for_dir_with_wrong_mode() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        let dir = tmp.path().display().to_string();

        let results = CredPermsCheck.run(&config_with_dirs(&[&dir]), &HashMap::new());
        assert_eq!(1, results.len());
        assert!(!results[0].passed);
        let detail = results[0].detail.as_deref().unwrap_or("");
        assert!(detail.contains("755"), "expected '755' in detail, got: {detail}");
        assert!(detail.contains("700"), "expected '700' in detail, got: {detail}");
    }

    #[test]
    fn skips_nonexistent_dir() {
        let results = CredPermsCheck.run(
            &config_with_dirs(&["/tmp/etch_nonexistent_cred_dir_xyz"]),
            &HashMap::new(),
        );
        assert!(results.is_empty(), "nonexistent dir should produce no result");
    }

    #[test]
    fn returns_empty_when_no_credential_dirs_configured() {
        let results = CredPermsCheck.run(&Config::default(), &HashMap::new());
        assert!(results.is_empty());
    }
}
```

- [ ] **Step 2: Run to confirm RED**

```bash
cargo test -p etch-lib doctor::cred_perms 2>&1 | grep -E "FAILED|panicked" | head -10
```

Expected: fails with "not yet implemented".

- [ ] **Step 3: Implement**

Replace the `todo!()` body:

```rust
    fn run(&self, config: &Config, _manifests: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        use std::os::unix::fs::PermissionsExt;

        let dirs = match &config.doctor {
            Some(d) if !d.credential_dirs.is_empty() => &d.credential_dirs,
            _ => return vec![],
        };

        dirs.iter()
            .filter_map(|dir| {
                let expanded = shellexpand::tilde(dir).into_owned();
                let path = std::path::Path::new(&expanded);

                if !path.exists() {
                    return None; // skip — machine may not have this credential type
                }

                match std::fs::metadata(path) {
                    Ok(meta) => {
                        let mode = meta.permissions().mode() & 0o777;
                        let passed = mode == 0o700;
                        Some(CheckResult {
                            label: format!("{dir} ({mode:03o})"),
                            passed,
                            detail: if passed {
                                None
                            } else {
                                Some(format!("mode {mode:03o}, expected 700"))
                            },
                        })
                    }
                    Err(e) => Some(CheckResult {
                        label: dir.clone(),
                        passed: false,
                        detail: Some(format!("cannot read metadata: {e}")),
                    }),
                }
            })
            .collect()
    }
```

Add `use shellexpand;` at the top.

- [ ] **Step 4: Run tests**

```bash
cargo test -p etch-lib doctor::cred_perms 2>&1 | grep -E "test.*ok|test.*FAILED" | head -10
```

Expected: all 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add lib/src/doctor/cred_perms.rs
git commit -m "feat(doctor): implement CredPermsCheck

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 6: Implement `VersionsCheck`

**Files:**

- Modify: `lib/src/doctor/versions.rs`

- [ ] **Step 1: Write failing tests**

Replace stub content with:

```rust
use super::{CheckResult, DoctorCheck};
use crate::actions::Actions;
use crate::config::{Config, DoctorConfig, VersionPin};
use crate::manifests::Manifest;
use std::collections::HashMap;
use std::process::Command;

pub struct VersionsCheck;

fn run_version_command(cmd: &str) -> Option<String> {
    todo!()
}

impl DoctorCheck for VersionsCheck {
    fn name(&self) -> &'static str { "Versions" }

    fn run(&self, config: &Config, manifests: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::os::unix::fs::PermissionsExt;

    fn make_fake_binary(dir: &std::path::Path, name: &str, output: &str) -> std::path::PathBuf {
        let bin = dir.join(name);
        std::fs::write(&bin, format!("#!/bin/sh\nprintf '%s\\n' '{}'\n", output)).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        bin
    }

    fn config_with_pin(tool: &str, command: &str, expected: &str) -> Config {
        Config {
            doctor: Some(DoctorConfig {
                versions: vec![VersionPin {
                    tool: tool.to_string(),
                    command: command.to_string(),
                    expected: expected.to_string(),
                }],
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    #[serial]
    fn passes_when_output_contains_expected() {
        let tmp = tempfile::tempdir().unwrap();
        let name = "fake_version_tool_pass_xyz";
        make_fake_binary(tmp.path(), name, "mytool 1.2.3");

        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old}", tmp.path().display()));

        let config = config_with_pin("mytool", &format!("{name}"), "1.2.3");
        let results = VersionsCheck.run(&config, &HashMap::new());

        std::env::set_var("PATH", old);

        assert_eq!(1, results.len());
        assert!(results[0].passed, "expected pass, got: {:?}", results[0].detail);
    }

    #[test]
    #[serial]
    fn fails_when_output_does_not_contain_expected() {
        let tmp = tempfile::tempdir().unwrap();
        let name = "fake_version_tool_fail_xyz";
        make_fake_binary(tmp.path(), name, "mytool 9.9.9");

        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old}", tmp.path().display()));

        let config = config_with_pin("mytool", &format!("{name}"), "1.2.3");
        let results = VersionsCheck.run(&config, &HashMap::new());

        std::env::set_var("PATH", old);

        assert_eq!(1, results.len());
        assert!(!results[0].passed);
        assert!(
            results[0].detail.as_deref().unwrap_or("").contains("got"),
            "got: {:?}",
            results[0].detail
        );
    }

    #[test]
    fn fails_when_command_not_found() {
        let config = config_with_pin(
            "nonexistent_tool",
            "etch_cli_nonexistent_tool_xyz --version",
            "1.0.0",
        );
        let results = VersionsCheck.run(&config, &HashMap::new());
        assert_eq!(1, results.len());
        assert!(!results[0].passed);
        assert!(
            results[0].detail.as_deref().unwrap_or("").contains("not found"),
            "got: {:?}",
            results[0].detail
        );
    }

    #[test]
    #[serial]
    fn skips_binary_github_with_latest_version() {
        // "latest" is not a pinned version — should not generate a check
        let tmp = tempfile::tempdir().unwrap();
        let name = "fake_bin_latest_xyz";
        make_fake_binary(tmp.path(), name, "tool latest");

        let yaml = format!(
            concat!(
                "actions:\n",
                "  - action: binary.github\n",
                "    name: {name}\n",
                "    directory: /tmp\n",
                "    repository: owner/repo\n",
                "    version: latest\n",
            )
        );
        let manifest: Manifest = serde_yaml_ng::from_str(&yaml).unwrap();
        let mut manifests = HashMap::new();
        manifests.insert("test".to_string(), manifest);

        let results = VersionsCheck.run(&Config::default(), &manifests);
        assert!(results.is_empty(), "version: latest should produce no check");
    }

    #[test]
    fn returns_empty_with_no_config_and_no_binary_atoms() {
        let results = VersionsCheck.run(&Config::default(), &HashMap::new());
        assert!(results.is_empty());
    }
}
```

- [ ] **Step 2: Run to confirm RED**

```bash
cargo test -p etch-lib doctor::versions 2>&1 | grep -E "FAILED|panicked" | head -10
```

Expected: fails with "not yet implemented".

- [ ] **Step 3: Implement**

Replace both `todo!()` bodies:

```rust
fn run_version_command(cmd: &str) -> Option<String> {
    let output = Command::new("sh").args(["-c", cmd]).output().ok()?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let trimmed = combined.trim().to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

impl DoctorCheck for VersionsCheck {
    fn name(&self) -> &'static str { "Versions" }

    fn run(&self, config: &Config, manifests: &HashMap<String, Manifest>) -> Vec<CheckResult> {
        let mut results = Vec::new();

        // binary.github and binary.url atoms with a non-latest version pin
        for manifest in manifests.values() {
            for action in &manifest.actions {
                let (binary_name, version) = match action {
                    Actions::BinaryGitHub(a) => {
                        (a.action.name.as_str(), a.action.version.as_deref())
                    }
                    Actions::BinaryUrl(a) => {
                        (a.action.name.as_str(), a.action.version.as_deref())
                    }
                    _ => continue,
                };

                let Some(version) = version else { continue };
                if version == "latest" { continue; }

                let cmd = format!("{binary_name} --version");
                let label = format!("{binary_name} {version}");

                match run_version_command(&cmd) {
                    Some(output) => {
                        let passed = output.contains(version);
                        let first_line = output.lines().next().unwrap_or(&output).to_string();
                        results.push(CheckResult {
                            label,
                            passed,
                            detail: if passed {
                                None
                            } else {
                                Some(format!("got \"{first_line}\", expected \"{version}\""))
                            },
                        });
                    }
                    None => {
                        results.push(CheckResult {
                            label,
                            passed: false,
                            detail: Some(String::from("command not found")),
                        });
                    }
                }
            }
        }

        // Explicit version pins from config
        if let Some(ref doctor) = config.doctor {
            for pin in &doctor.versions {
                let label = format!("{} {}", pin.tool, pin.expected);
                match run_version_command(&pin.command) {
                    Some(output) => {
                        let passed = output.contains(&pin.expected);
                        let first_line = output.lines().next().unwrap_or(&output).to_string();
                        results.push(CheckResult {
                            label,
                            passed,
                            detail: if passed {
                                None
                            } else {
                                Some(format!(
                                    "got \"{first_line}\", expected \"{}\"",
                                    pin.expected
                                ))
                            },
                        });
                    }
                    None => {
                        results.push(CheckResult {
                            label,
                            passed: false,
                            detail: Some(String::from("command not found")),
                        });
                    }
                }
            }
        }

        results
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p etch-lib doctor::versions 2>&1 | grep -E "test.*ok|test.*FAILED" | head -10
```

Expected: all 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add lib/src/doctor/versions.rs
git commit -m "feat(doctor): implement VersionsCheck

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 7: Create `app/src/commands/doctor.rs`

**Files:**

- Create: `app/src/commands/doctor.rs`

- [ ] **Step 1: Write the file**

```rust
use super::EtchCommand;
use crate::Runtime;
use clap::Parser;
use colored::Colorize;
use etch_lib::config::Config;
use etch_lib::doctor::cred_perms::CredPermsCheck;
use etch_lib::doctor::symlinks::SymlinkCheck;
use etch_lib::doctor::tools::ToolsCheck;
use etch_lib::doctor::versions::VersionsCheck;
use etch_lib::doctor::{CheckResult, DoctorCheck};
use etch_lib::manifests::load;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Parser, Debug, Default)]
pub(crate) struct Doctor {
    /// Output results as JSON
    #[arg(long)]
    pub json: bool,

    /// Only show failing checks
    #[arg(long)]
    pub missing_only: bool,
}

pub(crate) fn run_doctor_checks(
    config: &Config,
    contexts: &etch_lib::contexts::Contexts,
) -> anyhow::Result<Vec<(&'static str, Vec<CheckResult>)>> {
    let manifests = if let Some(first) = config.manifest_paths.first() {
        match crate::manifests::resolve(first) {
            Some(path) => load(path, contexts).unwrap_or_default(),
            None => HashMap::new(),
        }
    } else {
        HashMap::new()
    };

    Ok(vec![
        ("Symlinks", SymlinkCheck.run(config, &manifests)),
        ("Tools", ToolsCheck.run(config, &manifests)),
        ("Credential dirs", CredPermsCheck.run(config, &manifests)),
        ("Versions", VersionsCheck.run(config, &manifests)),
    ])
}

#[derive(Serialize)]
struct JsonCheckResult {
    label: String,
    passed: bool,
    detail: Option<String>,
}

#[derive(Serialize)]
struct JsonOutput {
    checks: Vec<JsonCheckResult>,
    summary: JsonSummary,
}

#[derive(Serialize)]
struct JsonSummary {
    passed: usize,
    failed: usize,
}

fn render_human(sections: &[(&'static str, Vec<CheckResult>)], missing_only: bool) {
    let total_passed = sections
        .iter()
        .flat_map(|(_, r)| r.iter())
        .filter(|r| r.passed)
        .count();
    let total_failed = sections
        .iter()
        .flat_map(|(_, r)| r.iter())
        .filter(|r| !r.passed)
        .count();

    for (section_name, results) in sections {
        if results.is_empty() {
            continue;
        }
        let has_failures = results.iter().any(|r| !r.passed);
        if missing_only && !has_failures {
            continue;
        }
        println!("{section_name}");
        for r in results {
            if missing_only && r.passed {
                continue;
            }
            if r.passed {
                println!("  {} {}", "✓".green(), r.label);
            } else {
                let detail = r.detail.as_deref().unwrap_or("failed");
                println!("  {} {}  [{}]", "✗".red(), r.label, detail);
            }
        }
        println!();
    }
    println!("{total_passed} passed, {total_failed} failed");
}

fn render_json(sections: &[(&'static str, Vec<CheckResult>)]) -> anyhow::Result<()> {
    let checks: Vec<JsonCheckResult> = sections
        .iter()
        .flat_map(|(_, results)| {
            results.iter().map(|r| JsonCheckResult {
                label: r.label.clone(),
                passed: r.passed,
                detail: r.detail.clone(),
            })
        })
        .collect();
    let passed = checks.iter().filter(|c| c.passed).count();
    let failed = checks.len() - passed;
    let output = JsonOutput {
        checks,
        summary: JsonSummary { passed, failed },
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

impl EtchCommand for Doctor {
    #[cfg(not(tarpaulin_include))]
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()> {
        let sections = run_doctor_checks(&runtime.config, &runtime.contexts)?;
        let any_failed = sections
            .iter()
            .flat_map(|(_, r)| r.iter())
            .any(|r| !r.passed);

        if self.json {
            render_json(&sections)?;
        } else {
            render_human(&sections, self.missing_only);
        }

        if any_failed {
            std::process::exit(1);
        }
        Ok(())
    }

    #[cfg(tarpaulin_include)]
    fn execute(&self, _runtime: &Runtime) -> anyhow::Result<()> {
        unreachable!()
    }
}
```

- [ ] **Step 2: Compile check**

```bash
cargo check -p etch-cli 2>&1 | grep "^error" | head -10
```

Expected: errors about `Doctor` not found in the commands module — not syntax errors.

- [ ] **Step 3: Register in `app/src/commands/mod.rs`**

Add after `mod update; pub(crate) use update::Update;`:

```rust
mod doctor;
pub(crate) use doctor::Doctor;
```

- [ ] **Step 4: Register in `app/src/config/mod.rs` Commands enum**

Add to `#[derive(Debug, Subcommand)] pub enum Commands {`:

```rust
    /// Check system health
    Doctor(commands::Doctor),
```

- [ ] **Step 5: Register dispatch in `app/src/main.rs`**

Add to the `match &runtime.args.command {` block in `pub(crate) fn execute()`:

```rust
        Commands::Doctor(d) => d.execute(&runtime),
```

- [ ] **Step 6: Compile check**

```bash
cargo check -p etch-cli 2>&1 | grep "^error" | head -10
```

Expected: no errors.

- [ ] **Step 7: Smoke test**

```bash
cargo run --bin etch -- doctor --help 2>&1 | head -15
```

Expected: help text showing `--json` and `--missing-only` flags.

- [ ] **Step 8: Commit**

```bash
git add app/src/commands/doctor.rs \
        app/src/commands/mod.rs \
        app/src/config/mod.rs \
        app/src/main.rs
git commit -m "feat: add etch doctor subcommand

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 8: Integration tests and snapshot

**Files:**

- Create: `app/tests/doctor.rs`
- Modify: `app/tests/snapshots.rs`

- [ ] **Step 1: Create `app/tests/doctor.rs`**

```rust
use assert_cmd::Command;
use tempfile::tempdir;

fn etch() -> Command {
    Command::cargo_bin("etch").unwrap()
}

#[test]
fn doctor_help_renders() {
    etch()
        .args(["doctor", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("--json"))
        .stdout(predicates::str::contains("--missing-only"));
}

#[test]
fn doctor_with_empty_config_exits_zero() {
    // No manifest paths, no doctor config — nothing to check — all sections empty
    let tmp = tempdir().unwrap();
    let config = tmp.path().join("etch.yaml");
    std::fs::write(&config, "").unwrap();

    etch()
        .args(["-c", &config.display().to_string(), "doctor"])
        .assert()
        .success();
}

#[test]
fn doctor_with_failing_cred_dir_exits_one() {
    let tmp = tempdir().unwrap();
    use std::os::unix::fs::PermissionsExt;

    // Create a dir with wrong permissions
    let cred_dir = tmp.path().join("bad_cred_dir");
    std::fs::create_dir(&cred_dir).unwrap();
    std::fs::set_permissions(&cred_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    let config = tmp.path().join("etch.yaml");
    std::fs::write(
        &config,
        format!(
            "doctor:\n  credential_dirs:\n    - {}\n",
            cred_dir.display()
        ),
    )
    .unwrap();

    etch()
        .args(["-c", &config.display().to_string(), "doctor"])
        .assert()
        .failure(); // exit code 1
}

#[test]
fn doctor_json_flag_outputs_valid_json() {
    let tmp = tempdir().unwrap();
    let config = tmp.path().join("etch.yaml");
    std::fs::write(&config, "").unwrap();

    let output = etch()
        .args(["-c", &config.display().to_string(), "doctor", "--json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("output should be valid JSON");
    assert!(parsed.get("checks").is_some(), "JSON should have 'checks' key");
    assert!(parsed.get("summary").is_some(), "JSON should have 'summary' key");
}

#[test]
fn doctor_missing_only_suppresses_passing_checks() {
    let tmp = tempdir().unwrap();
    let config = tmp.path().join("etch.yaml");
    // Add a tool that definitely exists
    std::fs::write(&config, "doctor:\n  tools:\n    - sh\n").unwrap();

    // With --missing-only, sh should not appear in output (it exists)
    let output = etch()
        .args(["-c", &config.display().to_string(), "doctor", "--missing-only"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("✓"),
        "expected no passing checks in --missing-only output, got:\n{stdout}"
    );
}
```

- [ ] **Step 2: Run integration tests**

```bash
cargo test -p etch-cli --test doctor 2>&1 | grep -E "test.*ok|test.*FAILED" | head -20
```

Expected: all 5 tests pass.

- [ ] **Step 3: Add snapshot test**

In `app/tests/snapshots.rs`, add a test following the existing pattern:

```rust
#[test]
fn doctor_help() {
    let output = Command::cargo_bin("etch")
        .unwrap()
        .args(["doctor", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    insta::assert_snapshot!(stdout);
}
```

- [ ] **Step 4: Generate snapshot**

```bash
INSTA_UPDATE=new cargo test --test snapshots doctor_help 2>&1 | tail -5
cargo insta accept
```

- [ ] **Step 5: Run all tests**

```bash
make test
```

Expected: all tests pass, lint clean.

- [ ] **Step 6: Commit**

```bash
git add app/tests/doctor.rs \
        app/tests/snapshots.rs \
        app/tests/snapshots/
git commit -m "test(doctor): add integration tests and help snapshot

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Task 9: Add example manifest and open PR

**Files:**

- Create: `examples/doctor/doctor.yaml`

- [ ] **Step 1: Create example**

```bash
mkdir -p examples/doctor
```

Create `examples/doctor/doctor.yaml`:

```yaml
# etch doctor checks system health independently of manifests.
# Configured via the doctor: key in etch.yaml (not in manifests).
# This file shows what to add to your etch.yaml — not a standalone manifest.

# Add to ~/.config/etch/etch.yaml:

# doctor:
#   # Explicit tools to verify exist in PATH (beyond manifest-derived tools)
#   tools:
#     - kubectl
#     - helm
#     - jq
#
#   # Binary version pins — checked against command output substring match
#   versions:
#     - tool: ripgrep
#       command: "rg --version"
#       expected: "14.1.0"
#     - tool: fd
#       command: "fd --version"
#       expected: "9.0"
#
#   # Directories to verify have mode 700 (skipped if dir does not exist)
#   credential_dirs:
#     - ~/.ssh
#     - ~/.tf_creds
#     - ~/.tsh

# Manifest-derived tool checks (no config needed):
# - brew.bundle / brew.upgrade / brew.cleanup → checks for brew
# - gem.install → checks for gem
# - pip.install → checks for pip
# - npm.install → checks for npm
# - mas.install / mas.upgrade → checks for mas
# - pyenv.install / pyenv.virtualenv → checks for pyenv
# - ruby.install → checks for ruby-install
# - claude.install / claude.upgrade / claude.plugin.update → checks for claude
#
# binary.github / binary.url atoms with version: set (not "latest") →
#   runs <name> --version and checks output contains the pinned version

actions: []
```

- [ ] **Step 2: Run make test one final time**

```bash
make test
```

Expected: all tests pass, lint clean.

- [ ] **Step 3: Commit and push**

```bash
git add examples/doctor/
git commit -m "docs(examples): add etch doctor configuration example

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"

git checkout -b feat/etch-doctor
git push -u origin feat/etch-doctor
gh pr create --repo brujack/etch-cli \
  --title "feat: add etch doctor subcommand" \
  --body "$(cat <<'EOF'
## Summary

- Adds `etch doctor` subcommand for system health validation
- Four checks: symlink integrity (file.link targets exist), tool existence in PATH, credential dir permissions (mode 700), binary version drift
- Config via `doctor:` key in `etch.yaml` — explicit tools, version pins, credential dirs
- Manifest-derived tool checks automatically inferred from action types
- `--json` flag for machine-readable output; `--missing-only` to suppress passing checks
- Exit code 0 = all pass, 1 = any fail

## Test plan

- [ ] Unit tests for all four checks pass
- [ ] Integration tests in `app/tests/doctor.rs` pass
- [ ] Snapshot test updated
- [ ] `make test` clean
- [ ] CI passes

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 4: Monitor CI**

```bash
gh pr checks --repo brujack/etch-cli --watch
```

If any check fails: `gh run view --repo brujack/etch-cli --log-failed`. Fix, commit, push.

---

## Post-Merge (do on `main` after PR auto-merges — NOT in worktree)

- [ ] Update plan index in `docs/superpowers/README.md` — add row and mark Done
- [ ] Add `> **Status: DONE**` banner to this plan file
- [ ] Update CLAUDE.md: increment action count (doctor is a subcommand, not an action — but verify coverage figure after CI)
- [ ] Commit directly on main: `docs: mark etch-doctor Done in plan index`
