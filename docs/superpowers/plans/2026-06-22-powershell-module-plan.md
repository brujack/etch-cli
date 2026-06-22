# powershell.module Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `powershell.module` action that idempotently installs PowerShell modules from PSGallery using `pwsh`.

**Architecture:** New action module at `lib/src/actions/powershell/` following the gem/npm/pip pattern: struct with `name`/`list`/`scope` fields, per-module idempotency check via `Get-Module -ListAvailable`, single `Exec` step batching all uninstalled modules into one `pwsh -Command "Install-Module ..."` call. Registered in the central `Actions` enum in `lib/src/actions/mod.rs`.

**Tech Stack:** Rust, serde/schemars, etch-lib action pattern, cargo nextest, PATH-mock testing pattern (tempdir fake `pwsh` binary).

## Global Constraints

- Repo root: `/Users/bruce/git-repos/personal/etch-cli/`
- Pattern: follow gem/npm/pip exactly — struct layout, test names, idempotency guard, `list` preferred over `name`
- `PowerShellScope` enum: variants `CurrentUser` (default) and `AllUsers` — PascalCase, matches PowerShell convention
- Idempotency check per module: `pwsh -Command "if (Get-Module -ListAvailable -Name '<name>') { exit 0 } else { exit 1 }"`
- Install step: `pwsh -Command "Install-Module -Name '<m1>','<m2>' -Scope <scope> -Force -AllowClobber"`
- `unwrap_or(false)` on pwsh errors → fail-safe generates step (matches gem/npm/pip)
- `make test` = `cargo fmt --check + clippy + cargo test` (pre-commit runs this)
- Coverage gate: ≥81% (CI only; do not run tarpaulin locally)

---

## Session Verification

```bash
make test                                                                    # full suite passes
cargo test -p etch-lib all_major_action_variants_can_be_deserialized        # new YAML entry parses
cargo test -p etch-lib all_action_variants_display                          # "powershell.module" in output
cargo test -p etch-lib powershell                                            # all powershell tests pass
```

---

### Task 1: Create PowershellModule struct, impl Action, and unit tests

```yaml-task
id: 1
description: Create lib/src/actions/powershell/mod.rs and module.rs with struct, scope enum, helper methods, impl Action, and all unit tests (direct struct deserialization, no Actions enum dependency)
role: executor
model: sonnet
tdd: required
acceptance:
  - cmd: cargo test -p etch-lib powershell
    exit_code: 0
  - cmd: make lint
    exit_code: 0
max_retries: 3
files_touched:
  - lib/src/actions/powershell/mod.rs
  - lib/src/actions/powershell/module.rs
depends_on: []
parallel_group: wave-1
```

**Files:**

- `lib/src/actions/powershell/mod.rs` — module re-export
- `lib/src/actions/powershell/module.rs` — struct + impl Action + tests

**Steps:**

- [ ] Create `lib/src/actions/powershell/mod.rs`:

```rust
mod module;
pub use module::PowershellModule;
```

- [ ] Write the test stubs first in `lib/src/actions/powershell/module.rs` to establish RED state. Create the file with the struct, enum, and test module containing `todo!()` stubs:

```rust
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerShellScope {
    #[default]
    CurrentUser,
    AllUsers,
}

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowershellModule {
    /// Single module name to install. Mutually exclusive with `list`.
    pub name: Option<String>,
    /// List of module names to install. Mutually exclusive with `name`.
    #[serde(default)]
    pub list: Vec<String>,
    /// Install scope: CurrentUser (default) or AllUsers.
    #[serde(default)]
    pub scope: PowerShellScope,
}

impl PowershellModule {
    fn module_names(&self) -> Vec<String> {
        todo!()
    }

    fn module_installed(_name: &str) -> bool {
        todo!()
    }
}

impl Action for PowershellModule {
    fn summarize(&self) -> String {
        todo!()
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;
    use serial_test::serial;

    const FAKE_MODULE: &str = "etch_cli_not_a_real_ps_module_xyz_zyx_test";

    #[test]
    fn it_deserializes_name() { todo!() }
    #[test]
    fn it_deserializes_list() { todo!() }
    #[test]
    fn it_deserializes_scope() { todo!() }
    #[test]
    fn scope_defaults_to_current_user() { todo!() }
    #[test]
    fn summarize_includes_module_name() { todo!() }
    #[test]
    fn summarize_includes_all_list_modules() { todo!() }
    #[test]
    fn summarize_with_no_modules_returns_generic_message() { todo!() }
    #[test]
    fn module_names_prefers_list_when_both_set() { todo!() }
    #[test]
    fn module_names_returns_single_name_as_vec() { todo!() }
    #[test]
    fn module_names_empty_when_no_name_or_list() { todo!() }
    #[test]
    fn plan_errors_without_name_or_list() { todo!() }
    #[test]
    #[serial]
    fn plan_returns_exec_for_uninstalled_module() { todo!() }
    #[test]
    #[serial]
    fn plan_returns_exec_for_uninstalled_list() { todo!() }
    #[test]
    #[serial]
    fn plan_skips_already_installed_module() { todo!() }
    #[test]
    #[serial]
    fn plan_skips_already_installed_modules_in_list() { todo!() }
    #[test]
    #[serial]
    fn plan_generates_step_when_pwsh_not_in_path() { todo!() }
    #[test]
    #[serial]
    fn plan_includes_scope_in_command() { todo!() }
    #[test]
    #[serial]
    fn plan_includes_force_and_allowclobber() { todo!() }
}
```

- [ ] Run `cargo test -p etch-lib powershell` — confirm all tests panic with `todo!()` (RED state).

- [ ] Implement `module_names()` and `module_installed()`:

```rust
impl PowershellModule {
    fn module_names(&self) -> Vec<String> {
        if !self.list.is_empty() {
            self.list.clone()
        } else if let Some(name) = &self.name {
            vec![name.clone()]
        } else {
            vec![]
        }
    }

    fn module_installed(name: &str) -> bool {
        std::process::Command::new("pwsh")
            .args([
                "-Command",
                &format!(
                    "if (Get-Module -ListAvailable -Name '{}') {{ exit 0 }} else {{ exit 1 }}",
                    name
                ),
            ])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
```

- [ ] Implement `summarize()` and `plan()`:

```rust
impl Action for PowershellModule {
    fn summarize(&self) -> String {
        let modules = self.module_names();
        if modules.is_empty() {
            return String::from("Installing PowerShell modules");
        }
        format!("Installing PowerShell module(s): {}", modules.join(", "))
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let modules = self.module_names();
        if modules.is_empty() {
            bail!("powershell.module requires either 'name' or 'list' to be specified");
        }

        let to_install: Vec<String> = modules
            .into_iter()
            .filter(|name| !Self::module_installed(name))
            .collect();

        if to_install.is_empty() {
            return Ok(vec![]);
        }

        let scope = match self.scope {
            PowerShellScope::CurrentUser => "CurrentUser",
            PowerShellScope::AllUsers => "AllUsers",
        };

        let module_list = to_install
            .iter()
            .map(|n| format!("'{}'", n))
            .collect::<Vec<_>>()
            .join(",");

        let command_str = format!(
            "Install-Module -Name {} -Scope {} -Force -AllowClobber",
            module_list, scope
        );

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("pwsh"),
                arguments: vec![String::from("-Command"), command_str],
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}
```

- [ ] Replace all `todo!()` stubs with full test implementations:

```rust
    #[test]
    fn it_deserializes_name() {
        let action: PowershellModule =
            serde_yaml_ng::from_str("name: oh-my-posh\n").unwrap();
        assert_eq!(Some("oh-my-posh".to_string()), action.name);
        assert!(action.list.is_empty());
        assert_eq!(PowerShellScope::CurrentUser, action.scope);
    }

    #[test]
    fn it_deserializes_list() {
        let action: PowershellModule =
            serde_yaml_ng::from_str("list:\n  - Az\n  - oh-my-posh\n").unwrap();
        assert_eq!(vec!["Az".to_string(), "oh-my-posh".to_string()], action.list);
        assert!(action.name.is_none());
    }

    #[test]
    fn it_deserializes_scope() {
        let action: PowershellModule =
            serde_yaml_ng::from_str("name: Az\nscope: AllUsers\n").unwrap();
        assert_eq!(PowerShellScope::AllUsers, action.scope);
    }

    #[test]
    fn scope_defaults_to_current_user() {
        assert_eq!(PowerShellScope::CurrentUser, PowershellModule::default().scope);
    }

    #[test]
    fn summarize_includes_module_name() {
        let action = PowershellModule {
            name: Some(String::from("oh-my-posh")),
            list: vec![],
            scope: PowerShellScope::CurrentUser,
        };
        let s = action.summarize();
        assert!(s.contains("oh-my-posh"), "expected 'oh-my-posh' in: {s}");
    }

    #[test]
    fn summarize_includes_all_list_modules() {
        let action = PowershellModule {
            name: None,
            list: vec![String::from("Az"), String::from("oh-my-posh")],
            scope: PowerShellScope::CurrentUser,
        };
        let s = action.summarize();
        assert!(s.contains("Az"), "expected 'Az' in: {s}");
        assert!(s.contains("oh-my-posh"), "expected 'oh-my-posh' in: {s}");
    }

    #[test]
    fn summarize_with_no_modules_returns_generic_message() {
        let s = PowershellModule::default().summarize();
        assert!(s.contains("PowerShell"), "expected 'PowerShell' in: {s}");
    }

    #[test]
    fn module_names_prefers_list_when_both_set() {
        let action = PowershellModule {
            name: Some(String::from("Az")),
            list: vec![String::from("oh-my-posh")],
            scope: PowerShellScope::CurrentUser,
        };
        assert_eq!(vec!["oh-my-posh".to_string()], action.module_names());
    }

    #[test]
    fn module_names_returns_single_name_as_vec() {
        let action = PowershellModule {
            name: Some(String::from("Az")),
            list: vec![],
            scope: PowerShellScope::CurrentUser,
        };
        assert_eq!(vec!["Az".to_string()], action.module_names());
    }

    #[test]
    fn module_names_empty_when_no_name_or_list() {
        assert!(PowershellModule::default().module_names().is_empty());
    }

    #[test]
    fn plan_errors_without_name_or_list() {
        let result =
            PowershellModule::default().plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("name") || msg.contains("list"),
            "expected helpful error, got: {msg}"
        );
    }

    #[test]
    #[serial]
    fn plan_returns_exec_for_uninstalled_module() {
        // FAKE_MODULE does not exist; real or absent pwsh both return non-zero for check
        let action = PowershellModule {
            name: Some(String::from(FAKE_MODULE)),
            list: vec![],
            scope: PowerShellScope::CurrentUser,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("pwsh"), "expected 'pwsh' in: {display}");
        assert!(
            display.contains(FAKE_MODULE),
            "expected module name in: {display}"
        );
        assert!(
            display.contains("Install-Module"),
            "expected 'Install-Module' in: {display}"
        );
    }

    #[test]
    #[serial]
    fn plan_returns_exec_for_uninstalled_list() {
        let action = PowershellModule {
            name: None,
            list: vec![String::from(FAKE_MODULE), format!("{FAKE_MODULE}2")],
            scope: PowerShellScope::CurrentUser,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains(FAKE_MODULE),
            "expected module name in: {display}"
        );
    }

    #[test]
    #[serial]
    fn plan_skips_already_installed_module() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let fake_pwsh = tmp.path().join("pwsh");
        // Fake pwsh exits 0 unconditionally — simulates module already installed
        std::fs::write(&fake_pwsh, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&fake_pwsh, std::fs::Permissions::from_mode(0o755)).unwrap();
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));
        let action = PowershellModule {
            name: Some(String::from("oh-my-posh")),
            list: vec![],
            scope: PowerShellScope::CurrentUser,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        std::env::set_var("PATH", old_path);
        assert!(
            steps.is_empty(),
            "expected no steps when module already installed"
        );
    }

    #[test]
    #[serial]
    fn plan_skips_already_installed_modules_in_list() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let fake_pwsh = tmp.path().join("pwsh");
        std::fs::write(&fake_pwsh, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&fake_pwsh, std::fs::Permissions::from_mode(0o755)).unwrap();
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));
        let action = PowershellModule {
            name: None,
            list: vec![String::from("Az"), String::from("oh-my-posh")],
            scope: PowerShellScope::CurrentUser,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        std::env::set_var("PATH", old_path);
        assert!(
            steps.is_empty(),
            "expected no steps when all modules already installed"
        );
    }

    #[test]
    #[serial]
    fn plan_generates_step_when_pwsh_not_in_path() {
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", "/nonexistent");
        let action = PowershellModule {
            name: Some(String::from("oh-my-posh")),
            list: vec![],
            scope: PowerShellScope::CurrentUser,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        std::env::set_var("PATH", old_path);
        // unwrap_or(false) → not installed → generates step
        assert_eq!(1, steps.len());
    }

    #[test]
    #[serial]
    fn plan_includes_scope_in_command() {
        let action = PowershellModule {
            name: Some(String::from(FAKE_MODULE)),
            list: vec![],
            scope: PowerShellScope::AllUsers,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("AllUsers"), "expected 'AllUsers' in: {display}");
    }

    #[test]
    #[serial]
    fn plan_includes_force_and_allowclobber() {
        let action = PowershellModule {
            name: Some(String::from(FAKE_MODULE)),
            list: vec![],
            scope: PowerShellScope::CurrentUser,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("-Force"), "expected '-Force' in: {display}");
        assert!(
            display.contains("-AllowClobber"),
            "expected '-AllowClobber' in: {display}"
        );
    }
```

- [ ] Run `cargo test -p etch-lib powershell` — confirm all 18 tests pass (GREEN).
- [ ] Run `make lint` — confirm fmt and clippy clean.
- [ ] Invoke `caveman:caveman-commit` for the commit message. Commit.

**Interfaces:**

- Produces: `PowershellModule` struct with `pub name: Option<String>`, `pub list: Vec<String>`, `pub scope: PowerShellScope`; `PowerShellScope` enum with variants `CurrentUser`, `AllUsers`; both exported from `lib/src/actions/powershell/mod.rs` as `pub use module::PowershellModule`

---

### Task 2: Register PowershellModule in Actions enum and add dispatch tests

```yaml-task
id: 2
description: Register powershell module in lib/src/actions/mod.rs (mod decl, use import, enum variant, 4 match arms) and add Actions-based deserialization test to module.rs; update three dispatch test counts and YAML lists
role: executor
model: sonnet
tdd: required
acceptance:
  - cmd: make test
    exit_code: 0
  - cmd: cargo test -p etch-lib all_major_action_variants_can_be_deserialized
    exit_code: 0
  - cmd: cargo test -p etch-lib all_action_variants_inner_ref_and_deref
    exit_code: 0
  - cmd: cargo test -p etch-lib all_action_variants_display
    exit_code: 0
max_retries: 3
files_touched:
  - lib/src/actions/mod.rs
  - lib/src/actions/powershell/module.rs
depends_on: [1]
parallel_group: wave-2
```

**Files:**

- `lib/src/actions/mod.rs` — registration + dispatch test updates
- `lib/src/actions/powershell/module.rs` — add `it_can_be_deserialized` test via `Actions` enum

**Steps:**

- [ ] Read `lib/src/actions/mod.rs` before editing to get exact byte content for Edit matches.

- [ ] Add `mod powershell;` after `mod pip;` (alphabetical: pip < powershell < pyenv):

    Find the line `mod pip;` and insert immediately after it:

    ```
    mod powershell;
    ```

- [ ] Add `use powershell::PowershellModule;` after the `use pip::...` import line. Read the file to find the exact `use pip::` line and insert after it:

    ```rust
    use powershell::PowershellModule;
    ```

- [ ] Add the enum variant. Read the enum to find the `PipInstall` variant block and insert `PowershellModule` immediately after it (alphabetical: PipInstall < PowershellModule < PyenvInstall). The new variant block:

    ```rust
        #[serde(rename = "powershell.module")]
        PowershellModule(ConditionalVariantAction<PowershellModule>),
    ```

- [ ] Add match arm in `inner_ref()`. Find all existing `inner_ref` match arms (they follow the pattern `Actions::Variant(a) => a.inner_ref()`). Insert after the `PipInstall` arm:

    ```rust
                Actions::PowershellModule(a) => a.inner_ref(),
    ```

- [ ] Add match arm in `notify` accessor. Same pattern — insert after `PipInstall` arm:

    ```rust
                Actions::PowershellModule(a) => a.notify,
    ```

- [ ] Add match arm in `Deref` impl. Same pattern — insert after `PipInstall` arm:

    ```rust
                Actions::PowershellModule(a) => a,
    ```

- [ ] Add match arm in `Display` impl. Same pattern — insert after `PipInstall` arm. The Display arms use `=> "action.name"`:

    ```rust
                Actions::PowershellModule(_) => "powershell.module",
    ```

- [ ] **Update `all_major_action_variants_can_be_deserialized`** (currently 35 actions):
    - Add entry to the YAML block after the `- action: zsh.oh-my-zsh` entry (or in alphabetical/logical position):
        ```yaml
        - action: powershell.module
          name: oh-my-posh
        ```
    - Update count: `assert_eq!(35, manifest.actions.len())` → `assert_eq!(36, manifest.actions.len())`

- [ ] **Update `all_action_variants_inner_ref_and_deref`** (currently 49 actions):
    - Add entry to the YAML block after the `- action: terraform.tfenv` entry:
        ```yaml
        - action: powershell.module
          name: oh-my-posh
        ```
    - Update count: `assert_eq!(49, manifest.actions.len())` → `assert_eq!(50, manifest.actions.len())`

- [ ] **Update `all_action_variants_display`** (currently 48 actions):
    - Add entry to the YAML block after the `- action: terraform.tfenv` entry:
        ```yaml
        - action: powershell.module
          name: oh-my-posh
        ```
    - Update count: `assert_eq!(48, manifest.actions.len())` → `assert_eq!(49, manifest.actions.len())`
    - Add assertion after the last `names.contains` assertion:
        ```rust
                assert!(names.contains(&"powershell.module".to_string()));
        ```

- [ ] Add `it_can_be_deserialized` test to `lib/src/actions/powershell/module.rs`. Add at the top of the `#[cfg(test)] mod tests` block (after imports, before `FAKE_MODULE`):

    ```rust
        #[test]
        fn it_can_be_deserialized() {
            use crate::actions::Actions;
            let yaml = "- action: powershell.module\n  name: oh-my-posh\n";
            let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
            match actions.pop() {
                Some(Actions::PowershellModule(action)) => {
                    assert_eq!(Some("oh-my-posh".to_string()), action.action.name);
                    assert!(action.action.list.is_empty());
                }
                _ => panic!("PowershellModule didn't deserialize to the correct type"),
            }
        }

        #[test]
        fn it_can_be_deserialized_with_list() {
            use crate::actions::Actions;
            let yaml = concat!(
                "- action: powershell.module\n",
                "  list:\n",
                "    - Az\n",
                "    - oh-my-posh\n",
            );
            let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
            match actions.pop() {
                Some(Actions::PowershellModule(action)) => {
                    assert_eq!(
                        vec!["Az".to_string(), "oh-my-posh".to_string()],
                        action.action.list
                    );
                }
                _ => panic!("PowershellModule didn't deserialize to the correct type"),
            }
        }
    ```

- [ ] Run `make test` — confirm full suite passes (GREEN).
- [ ] Invoke `caveman:caveman-commit`. Commit.

**Interfaces:**

- Consumes: `PowershellModule` from `lib/src/actions/powershell/mod.rs` (Task 1)
- Produces: `Actions::PowershellModule` variant registered and dispatchable; display name `"powershell.module"`

---

### Task 3: Add example YAML and update action catalogs

```yaml-task
id: 3
description: Add examples/powershell/powershell-module.yaml and update powershell.module row in CLAUDE.md and README.md action catalog tables (docs-only, no behavior change)
role: executor
model: sonnet
tdd: not-applicable
acceptance:
  - cmd: test -f examples/powershell/powershell-module.yaml
    exit_code: 0
  - cmd: grep -q "powershell.module" CLAUDE.md
    exit_code: 0
  - cmd: grep -q "powershell.module" README.md
    exit_code: 0
max_retries: 2
files_touched:
  - examples/powershell/powershell-module.yaml
  - CLAUDE.md
  - README.md
depends_on: []
parallel_group: wave-1
```

**Files:**

- `examples/powershell/powershell-module.yaml` — example manifest
- `CLAUDE.md` — action catalog table row
- `README.md` — action catalog table row

**Steps:**

- [ ] Read `examples/npm/npm-install.yaml` to understand the example format (comments on every field, one entry per option combination).

- [ ] Create `examples/powershell/powershell-module.yaml`:

```yaml
# powershell.module — install PowerShell modules from PSGallery using pwsh
# Idempotent: skips modules already installed (Get-Module -ListAvailable check)
# Requires: pwsh (PowerShell Core) in PATH

# Install a single module (CurrentUser scope by default)
- action: powershell.module
  name: oh-my-posh # module name as listed in PSGallery

# Install multiple modules in one action (preferred for batching)
- action: powershell.module
  list:
      - Az # Azure PowerShell module
      - AWSPowerShell.NetCore # AWS PowerShell module
      - Microsoft.Graph # Microsoft Graph SDK
      - oh-my-posh # prompt theme engine

# Explicit CurrentUser scope (same as default; shown for clarity)
- action: powershell.module
  name: PSReadLine
  scope: CurrentUser # installs to user profile; no admin required

# AllUsers scope — installs system-wide; requires admin rights at apply time
- action: powershell.module
  name: PowerShellGet
  scope: AllUsers # requires elevated privileges when etch apply runs
```

- [ ] Read `CLAUDE.md` to find the Action Catalog table. Add a row for `powershell.module` in alphabetical order (after `pip.install`, before `pyenv.install`). The table uses the format:

    ```
    | `powershell.module` | Install PowerShell modules from PSGallery | `name`, `list`, `scope` |
    ```

    Match the exact column structure of surrounding rows.

- [ ] Read `README.md` to find its Action Catalog table. Add the same row in the same alphabetical position.

- [ ] Run `make lint` — confirm clippy/fmt clean (YAML and docs changes only; no Rust changes).
- [ ] Invoke `caveman:caveman-commit`. Commit.

---

### Task 4: Update plan index

```yaml-task
id: 4
description: Update docs/superpowers/README.md to link the new plan file and set status to Done (docs-only, no behavior change)
role: executor
model: haiku
tdd: not-applicable
acceptance:
  - cmd: grep -q "powershell-module" docs/superpowers/README.md
    exit_code: 0
max_retries: 2
files_touched:
  - docs/superpowers/README.md
depends_on: [1, 2, 3]
parallel_group: wave-3
```

**Files:**

- `docs/superpowers/README.md` — update plan index row

**Steps:**

- [ ] Read `docs/superpowers/README.md` to find the current row for powershell-module (Status: Pending, no plan link).

- [ ] Update the row to add the plan link and set status to Done:

    ```
    | 2026-06-22 | [powershell-module](plans/2026-06-22-powershell-module-plan.md) | [powershell-module](specs/2026-06-22-powershell-module-design.md) | Done |
    ```

- [ ] Also remove `powershell.module` from the Backlog table (it was listed there before this implementation).

- [ ] Invoke `caveman:caveman-commit`. Commit.
