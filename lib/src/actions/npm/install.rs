use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NpmInstall {
    /// Single package name to install globally. Mutually exclusive with `list`.
    pub name: Option<String>,
    /// List of package names to install globally. Mutually exclusive with `name`.
    #[serde(default)]
    pub list: Vec<String>,
}

impl NpmInstall {
    fn package_names(&self) -> Vec<String> {
        if !self.list.is_empty() {
            self.list.clone()
        } else if let Some(name) = &self.name {
            vec![name.clone()]
        } else {
            vec![]
        }
    }

    fn npm_installed(name: &str) -> bool {
        std::process::Command::new("npm")
            .args(["list", "-g", "--depth=0", name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl Action for NpmInstall {
    fn summarize(&self) -> String {
        let packages = self.package_names();
        if packages.is_empty() {
            return String::from("Installing npm packages");
        }
        format!("Installing npm package(s): {}", packages.join(", "))
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let packages = self.package_names();
        if packages.is_empty() {
            bail!("npm.install requires either 'name' or 'list' to be specified");
        }

        let to_install: Vec<String> = packages
            .into_iter()
            .filter(|name| !Self::npm_installed(name))
            .collect();

        if to_install.is_empty() {
            return Ok(vec![]);
        }

        let mut arguments = vec![String::from("install"), String::from("-g")];
        arguments.extend(to_install);

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("npm"),
                arguments,
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
    use crate::actions::Actions;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;
    use serial_test::serial;

    const FAKE_PKG: &str = "etch_cli_not_a_real_npm_pkg_xyz_zyx_test";

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: npm.install
  name: typescript
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::NpmInstall(action)) => {
                assert_eq!(Some("typescript".to_string()), action.action.name);
                assert!(action.action.list.is_empty());
            }
            _ => panic!("NpmInstall didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_list() {
        let yaml = r#"
- action: npm.install
  list:
    - typescript
    - "@anthropic-ai/claude-code"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::NpmInstall(action)) => {
                assert_eq!(
                    vec!["typescript", "@anthropic-ai/claude-code"],
                    action.action.list
                );
                assert!(action.action.name.is_none());
            }
            _ => panic!("NpmInstall didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn summarize_includes_package_name() {
        let action = NpmInstall {
            name: Some(String::from("typescript")),
            list: vec![],
        };
        let s = action.summarize();
        assert!(s.contains("typescript"), "expected 'typescript' in: {s}");
    }

    #[test]
    fn summarize_includes_all_list_packages() {
        let action = NpmInstall {
            name: None,
            list: vec![
                String::from("typescript"),
                String::from("@anthropic-ai/claude-code"),
            ],
        };
        let s = action.summarize();
        assert!(s.contains("typescript"), "expected 'typescript' in: {s}");
        assert!(
            s.contains("@anthropic-ai/claude-code"),
            "expected '@anthropic-ai/claude-code' in: {s}"
        );
    }

    #[test]
    fn summarize_with_no_packages_returns_generic_message() {
        let action = NpmInstall::default();
        let s = action.summarize();
        assert!(s.contains("npm"), "expected 'npm' in: {s}");
    }

    #[test]
    fn plan_errors_without_name_or_list() {
        let action = NpmInstall::default();
        let result = action.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err(), "expected error when no name or list");
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("name") || msg.contains("list"),
            "expected helpful error message, got: {msg}"
        );
    }

    #[test]
    #[serial]
    fn plan_returns_exec_for_uninstalled_package() {
        let action = NpmInstall {
            name: Some(String::from(FAKE_PKG)),
            list: vec![],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("npm"),
            "expected 'npm' command in: {display}"
        );
        assert!(
            display.contains(FAKE_PKG),
            "expected package name in: {display}"
        );
        assert!(
            display.contains("install"),
            "expected 'install' subcommand in: {display}"
        );
        assert!(display.contains("-g"), "expected '-g' flag in: {display}");
    }

    #[test]
    #[serial]
    fn plan_returns_exec_for_uninstalled_list() {
        let action = NpmInstall {
            name: None,
            list: vec![String::from(FAKE_PKG), format!("{FAKE_PKG}2")],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains(FAKE_PKG),
            "expected package name in: {display}"
        );
    }

    #[test]
    fn package_names_prefers_list_when_both_set() {
        let action = NpmInstall {
            name: Some(String::from("typescript")),
            list: vec![String::from("prettier")],
        };
        let names = action.package_names();
        assert_eq!(vec!["prettier".to_string()], names);
    }

    #[test]
    fn package_names_returns_single_name_as_vec() {
        let action = NpmInstall {
            name: Some(String::from("typescript")),
            list: vec![],
        };
        let names = action.package_names();
        assert_eq!(vec!["typescript".to_string()], names);
    }

    #[test]
    fn package_names_empty_when_no_name_or_list() {
        let action = NpmInstall::default();
        assert!(action.package_names().is_empty());
    }

    #[test]
    #[serial]
    fn plan_skips_already_installed_package() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake_npm = tmp.path().join("npm");
        std::fs::write(&fake_npm, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&fake_npm, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

        let action = NpmInstall {
            name: Some(String::from("typescript")),
            list: vec![],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old_path);

        assert!(
            steps.is_empty(),
            "expected no steps when package is already installed"
        );
    }

    #[test]
    #[serial]
    fn plan_skips_already_installed_packages_in_list() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake_npm = tmp.path().join("npm");
        std::fs::write(&fake_npm, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&fake_npm, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

        let action = NpmInstall {
            name: None,
            list: vec![String::from("typescript"), String::from("prettier")],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old_path);

        assert!(
            steps.is_empty(),
            "expected no steps when all packages are already installed"
        );
    }

    #[test]
    #[serial]
    fn plan_generates_step_when_npm_not_in_path() {
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", "/nonexistent");

        let action = NpmInstall {
            name: Some(String::from("typescript")),
            list: vec![],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old_path);

        // npm_installed returns false on error → fail-safe: generate step
        assert_eq!(1, steps.len());
    }
}
