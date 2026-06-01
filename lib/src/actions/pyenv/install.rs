use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PyenvInstall {
    /// Python version to install via pyenv (e.g. "3.12.0").
    pub version: Option<String>,
}

impl PyenvInstall {
    fn version_installed(version: &str) -> bool {
        std::process::Command::new("pyenv")
            .args(["versions", "--bare"])
            .output()
            .map(|o| {
                o.status.success()
                    && String::from_utf8_lossy(&o.stdout)
                        .lines()
                        .any(|line| line.trim() == version)
            })
            .unwrap_or(false)
    }
}

impl Action for PyenvInstall {
    fn summarize(&self) -> String {
        match &self.version {
            Some(v) => format!("Installing Python {v} via pyenv"),
            None => String::from("Installing Python version via pyenv"),
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let version = match &self.version {
            Some(v) => v.clone(),
            None => bail!("pyenv.install requires 'version' to be specified"),
        };

        if Self::version_installed(&version) {
            return Ok(vec![]);
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("pyenv"),
                arguments: vec![String::from("install"), version],
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

    const FAKE_VERSION: &str = "0.0.1-etch-cli-test";

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: pyenv.install
  version: "3.12.0"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::PyenvInstall(action)) => {
                assert_eq!(Some("3.12.0".to_string()), action.action.version);
            }
            _ => panic!("PyenvInstall didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn summarize_includes_version() {
        let action = PyenvInstall {
            version: Some(String::from("3.12.0")),
        };
        let s = action.summarize();
        assert!(s.contains("3.12.0"), "expected version in: {s}");
        assert!(s.contains("pyenv"), "expected 'pyenv' in: {s}");
    }

    #[test]
    fn summarize_with_no_version_returns_generic_message() {
        let action = PyenvInstall::default();
        let s = action.summarize();
        assert!(s.contains("pyenv"), "expected 'pyenv' in: {s}");
    }

    #[test]
    fn plan_errors_without_version() {
        let action = PyenvInstall::default();
        let result = action.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err(), "expected error when no version");
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("version"),
            "expected helpful error message, got: {msg}"
        );
    }

    #[test]
    #[serial]
    fn plan_returns_exec_for_uninstalled_version() {
        let action = PyenvInstall {
            version: Some(String::from(FAKE_VERSION)),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("pyenv"),
            "expected 'pyenv' command in: {display}"
        );
        assert!(
            display.contains(FAKE_VERSION),
            "expected version in: {display}"
        );
        assert!(
            display.contains("install"),
            "expected 'install' subcommand in: {display}"
        );
    }

    #[test]
    #[serial]
    fn plan_skips_already_installed_version() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake_pyenv = tmp.path().join("pyenv");
        // Fake pyenv: for "versions --bare" prints "3.11.0\n3.12.0\n", exits 0
        std::fs::write(
            &fake_pyenv,
            "#!/bin/sh\nif [ \"$1\" = \"versions\" ]; then printf '3.11.0\\n3.12.0\\n'; fi\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake_pyenv, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

        let action = PyenvInstall {
            version: Some(String::from("3.12.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old_path);

        assert!(
            steps.is_empty(),
            "expected no steps when version is already installed"
        );
    }

    #[test]
    #[serial]
    fn plan_installs_when_version_not_in_list() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake_pyenv = tmp.path().join("pyenv");
        // Fake pyenv: "versions --bare" prints only 3.11.0
        std::fs::write(
            &fake_pyenv,
            "#!/bin/sh\nif [ \"$1\" = \"versions\" ]; then printf '3.11.0\\n'; fi\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake_pyenv, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

        let action = PyenvInstall {
            version: Some(String::from("3.12.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old_path);

        assert_eq!(
            1,
            steps.len(),
            "expected one step when version not installed"
        );
        let display = steps[0].atom.to_string();
        assert!(display.contains("3.12.0"), "expected version in: {display}");
    }

    #[test]
    #[serial]
    fn plan_generates_step_when_pyenv_not_in_path() {
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", "/nonexistent");

        let action = PyenvInstall {
            version: Some(String::from("3.12.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old_path);

        // version_installed returns false on error → fail-safe: generate step
        assert_eq!(1, steps.len());
    }
}
