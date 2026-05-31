use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipInstall {
    /// Single package name to install. Mutually exclusive with `list`.
    pub name: Option<String>,
    /// List of package names to install. Mutually exclusive with `name`.
    #[serde(default)]
    pub list: Vec<String>,
    /// Exact version (e.g. "2.28.0"). Only meaningful when installing a single package.
    pub version: Option<String>,
    /// Path to a virtualenv. If set, uses `<virtualenv>/bin/pip` instead of `pip`.
    pub virtualenv: Option<String>,
}

impl PipInstall {
    fn package_names(&self) -> Vec<String> {
        if !self.list.is_empty() {
            self.list.clone()
        } else if let Some(name) = &self.name {
            vec![name.clone()]
        } else {
            vec![]
        }
    }

    fn pip_bin(&self) -> String {
        if let Some(venv) = &self.virtualenv {
            let expanded = shellexpand::tilde(venv);
            format!("{}/bin/pip", expanded)
        } else {
            String::from("pip3")
        }
    }

    fn package_installed(pip: &str, name: &str) -> bool {
        std::process::Command::new(pip)
            .args(["show", name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl Action for PipInstall {
    fn summarize(&self) -> String {
        let packages = self.package_names();
        if packages.is_empty() {
            return String::from("Installing pip packages");
        }
        format!("Installing pip package(s): {}", packages.join(", "))
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let packages = self.package_names();
        if packages.is_empty() {
            bail!("pip.install requires either 'name' or 'list' to be specified");
        }

        let pip = self.pip_bin();

        let to_install: Vec<String> = packages
            .into_iter()
            .filter(|name| !Self::package_installed(&pip, name))
            .collect();

        if to_install.is_empty() {
            return Ok(vec![]);
        }

        let mut arguments = vec![String::from("install")];
        if to_install.len() == 1 {
            if let Some(v) = &self.version {
                arguments.push(format!("{}=={}", to_install[0], v));
            } else {
                arguments.extend(to_install);
            }
        } else {
            arguments.extend(to_install);
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: pip,
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

    const FAKE_PKG: &str = "etch_cli_not_a_real_pip_pkg_xyz_zyx_test";

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: pip.install
  name: requests
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::PipInstall(action)) => {
                assert_eq!(Some("requests".to_string()), action.action.name);
                assert!(action.action.list.is_empty());
                assert!(action.action.version.is_none());
                assert!(action.action.virtualenv.is_none());
            }
            _ => panic!("PipInstall didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_list() {
        let yaml = r#"
- action: pip.install
  list:
    - requests
    - flask
    - pytest
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::PipInstall(action)) => {
                assert_eq!(vec!["requests", "flask", "pytest"], action.action.list);
                assert!(action.action.name.is_none());
            }
            _ => panic!("PipInstall didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_version() {
        let yaml = r#"
- action: pip.install
  name: requests
  version: "2.28.0"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::PipInstall(action)) => {
                assert_eq!(Some("requests".to_string()), action.action.name);
                assert_eq!(Some("2.28.0".to_string()), action.action.version);
            }
            _ => panic!("PipInstall didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_virtualenv() {
        let yaml = r#"
- action: pip.install
  name: requests
  virtualenv: /tmp/venv
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::PipInstall(action)) => {
                assert_eq!(Some("requests".to_string()), action.action.name);
                assert_eq!(Some("/tmp/venv".to_string()), action.action.virtualenv);
            }
            _ => panic!("PipInstall didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn summarize_includes_package_name() {
        let action = PipInstall {
            name: Some(String::from("requests")),
            list: vec![],
            version: None,
            virtualenv: None,
        };
        let s = action.summarize();
        assert!(s.contains("requests"), "expected 'requests' in: {s}");
    }

    #[test]
    fn summarize_includes_all_list_packages() {
        let action = PipInstall {
            name: None,
            list: vec![String::from("requests"), String::from("flask")],
            version: None,
            virtualenv: None,
        };
        let s = action.summarize();
        assert!(s.contains("requests"), "expected 'requests' in: {s}");
        assert!(s.contains("flask"), "expected 'flask' in: {s}");
    }

    #[test]
    fn summarize_with_no_packages_returns_generic_message() {
        let action = PipInstall::default();
        let s = action.summarize();
        assert!(s.contains("pip"), "expected 'pip' in: {s}");
    }

    #[test]
    fn plan_errors_without_name_or_list() {
        let action = PipInstall::default();
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
        let action = PipInstall {
            name: Some(String::from(FAKE_PKG)),
            list: vec![],
            version: None,
            virtualenv: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("pip"),
            "expected 'pip' command in: {display}"
        );
        assert!(
            display.contains(FAKE_PKG),
            "expected package name in: {display}"
        );
        assert!(
            display.contains("install"),
            "expected 'install' subcommand in: {display}"
        );
    }

    #[test]
    #[serial]
    fn plan_returns_exec_for_uninstalled_list() {
        let action = PipInstall {
            name: None,
            list: vec![String::from(FAKE_PKG), format!("{FAKE_PKG}2")],
            version: None,
            virtualenv: None,
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
    #[serial]
    fn plan_includes_version_in_package_specifier() {
        let action = PipInstall {
            name: Some(String::from(FAKE_PKG)),
            list: vec![],
            version: Some(String::from("1.0.0")),
            virtualenv: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains(&format!("{}==1.0.0", FAKE_PKG)),
            "expected name==version specifier in: {display}"
        );
    }

    #[test]
    fn plan_uses_venv_pip_when_virtualenv_set() {
        let action = PipInstall {
            name: Some(String::from(FAKE_PKG)),
            list: vec![],
            version: None,
            virtualenv: Some(String::from("/tmp/testvenv")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("/tmp/testvenv/bin/pip"),
            "expected venv pip path in: {display}"
        );
    }

    #[test]
    fn package_names_prefers_list_when_both_set() {
        let action = PipInstall {
            name: Some(String::from("flask")),
            list: vec![String::from("requests")],
            version: None,
            virtualenv: None,
        };
        let names = action.package_names();
        assert_eq!(vec!["requests".to_string()], names);
    }

    #[test]
    fn package_names_returns_single_name_as_vec() {
        let action = PipInstall {
            name: Some(String::from("requests")),
            list: vec![],
            version: None,
            virtualenv: None,
        };
        let names = action.package_names();
        assert_eq!(vec!["requests".to_string()], names);
    }

    #[test]
    fn package_names_empty_when_no_name_or_list() {
        let action = PipInstall::default();
        assert!(action.package_names().is_empty());
    }

    #[test]
    fn pip_bin_defaults_to_pip3() {
        let action = PipInstall::default();
        assert_eq!("pip3", action.pip_bin());
    }

    #[test]
    fn pip_bin_uses_venv_path_when_virtualenv_set() {
        let action = PipInstall {
            name: None,
            list: vec![],
            version: None,
            virtualenv: Some(String::from("/opt/venv")),
        };
        assert_eq!("/opt/venv/bin/pip", action.pip_bin());
    }

    #[test]
    #[serial]
    fn plan_skips_already_installed_package() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake_pip = tmp.path().join("pip3");
        // Fake pip3 exits 0 for any invocation — simulates package being installed
        std::fs::write(&fake_pip, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&fake_pip, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

        let action = PipInstall {
            name: Some(String::from("requests")),
            list: vec![],
            version: None,
            virtualenv: None,
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
        let fake_pip = tmp.path().join("pip3");
        std::fs::write(&fake_pip, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&fake_pip, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

        let action = PipInstall {
            name: None,
            list: vec![String::from("requests"), String::from("flask")],
            version: None,
            virtualenv: None,
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
}
