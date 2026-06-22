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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;
    use serial_test::serial;

    const FAKE_MODULE: &str = "etch_cli_not_a_real_ps_module_xyz_zyx_test";

    #[test]
    fn it_deserializes_name() {
        let action: PowershellModule = serde_yaml_ng::from_str("name: oh-my-posh\n").unwrap();
        assert_eq!(Some("oh-my-posh".to_string()), action.name);
        assert!(action.list.is_empty());
        assert_eq!(PowerShellScope::CurrentUser, action.scope);
    }

    #[test]
    fn it_deserializes_list() {
        let action: PowershellModule =
            serde_yaml_ng::from_str("list:\n  - Az\n  - oh-my-posh\n").unwrap();
        assert_eq!(
            vec!["Az".to_string(), "oh-my-posh".to_string()],
            action.list
        );
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
        assert_eq!(
            PowerShellScope::CurrentUser,
            PowershellModule::default().scope
        );
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
        let result = PowershellModule::default().plan(&Manifest::default(), &Contexts::default());
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
        assert!(
            display.contains("AllUsers"),
            "expected 'AllUsers' in: {display}"
        );
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
        assert!(
            display.contains("-Force"),
            "expected '-Force' in: {display}"
        );
        assert!(
            display.contains("-AllowClobber"),
            "expected '-AllowClobber' in: {display}"
        );
    }
}
