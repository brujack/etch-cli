use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PyenvVirtualenv {
    /// Python version to base the virtualenv on (e.g. "3.12.0").
    pub python_version: Option<String>,
    /// Name of the virtualenv to create (e.g. "myproject").
    pub name: Option<String>,
}

impl PyenvVirtualenv {
    fn virtualenv_exists(name: &str) -> bool {
        std::process::Command::new("pyenv")
            .args(["virtualenvs", "--bare"])
            .output()
            .map(|o| {
                o.status.success()
                    && String::from_utf8_lossy(&o.stdout)
                        .lines()
                        .any(|line| line.trim() == name)
            })
            .unwrap_or(false)
    }
}

impl Action for PyenvVirtualenv {
    fn summarize(&self) -> String {
        match (&self.python_version, &self.name) {
            (Some(v), Some(n)) => format!("Creating pyenv virtualenv {n} (Python {v})"),
            (None, Some(n)) => format!("Creating pyenv virtualenv {n}"),
            _ => String::from("Creating pyenv virtualenv"),
        }
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let python_version = match &self.python_version {
            Some(v) => v.clone(),
            None => bail!("pyenv.virtualenv requires 'python_version' to be specified"),
        };

        let name = match &self.name {
            Some(n) => n.clone(),
            None => bail!("pyenv.virtualenv requires 'name' to be specified"),
        };

        if Self::virtualenv_exists(&name) {
            return Ok(vec![]);
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("pyenv"),
                arguments: vec![String::from("virtualenv"), python_version, name],
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

    const FAKE_VERSION: &str = "3.12.0";
    const FAKE_NAME: &str = "etch-cli-test-venv";

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: pyenv.virtualenv
  python_version: "3.12.0"
  name: myproject
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::PyenvVirtualenv(action)) => {
                assert_eq!(Some("3.12.0".to_string()), action.action.python_version);
                assert_eq!(Some("myproject".to_string()), action.action.name);
            }
            _ => panic!("PyenvVirtualenv didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn summarize_includes_version_and_name() {
        let action = PyenvVirtualenv {
            python_version: Some(String::from("3.12.0")),
            name: Some(String::from("myproject")),
        };
        let s = action.summarize();
        assert!(s.contains("3.12.0"), "expected version in: {s}");
        assert!(s.contains("myproject"), "expected name in: {s}");
        assert!(s.contains("pyenv"), "expected 'pyenv' in: {s}");
    }

    #[test]
    fn summarize_with_name_only() {
        let action = PyenvVirtualenv {
            python_version: None,
            name: Some(String::from("myproject")),
        };
        let s = action.summarize();
        assert!(s.contains("myproject"), "expected name in: {s}");
    }

    #[test]
    fn summarize_with_no_fields_returns_generic_message() {
        let action = PyenvVirtualenv::default();
        let s = action.summarize();
        assert!(s.contains("pyenv"), "expected 'pyenv' in: {s}");
        assert!(s.contains("virtualenv"), "expected 'virtualenv' in: {s}");
    }

    #[test]
    fn plan_errors_without_python_version() {
        let action = PyenvVirtualenv {
            python_version: None,
            name: Some(String::from("myproject")),
        };
        let result = action.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err(), "expected error when python_version absent");
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("python_version"),
            "expected helpful error message, got: {msg}"
        );
    }

    #[test]
    fn plan_errors_without_name() {
        let action = PyenvVirtualenv {
            python_version: Some(String::from("3.12.0")),
            name: None,
        };
        let result = action.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err(), "expected error when name absent");
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("name"),
            "expected helpful error message, got: {msg}"
        );
    }

    #[test]
    fn plan_errors_without_any_fields() {
        let action = PyenvVirtualenv::default();
        let result = action.plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err(), "expected error when no fields set");
    }

    #[test]
    #[serial]
    fn plan_returns_exec_for_nonexistent_virtualenv() {
        let action = PyenvVirtualenv {
            python_version: Some(String::from(FAKE_VERSION)),
            name: Some(String::from(FAKE_NAME)),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("pyenv"), "expected 'pyenv' in: {display}");
        assert!(
            display.contains("virtualenv"),
            "expected 'virtualenv' subcommand in: {display}"
        );
        assert!(
            display.contains(FAKE_VERSION),
            "expected version in: {display}"
        );
        assert!(display.contains(FAKE_NAME), "expected name in: {display}");
    }

    #[test]
    #[serial]
    fn plan_skips_already_existing_virtualenv() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake_pyenv = tmp.path().join("pyenv");
        // Fake pyenv: "virtualenvs --bare" prints existing virtualenvs
        std::fs::write(
            &fake_pyenv,
            "#!/bin/sh\nif [ \"$1\" = \"virtualenvs\" ]; then printf 'myproject\\nother-venv\\n'; fi\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake_pyenv, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

        let action = PyenvVirtualenv {
            python_version: Some(String::from("3.12.0")),
            name: Some(String::from("myproject")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old_path);

        assert!(
            steps.is_empty(),
            "expected no steps when virtualenv already exists"
        );
    }

    #[test]
    #[serial]
    fn plan_creates_when_name_not_in_list() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake_pyenv = tmp.path().join("pyenv");
        // Fake pyenv: "virtualenvs --bare" prints only other-venv
        std::fs::write(
            &fake_pyenv,
            "#!/bin/sh\nif [ \"$1\" = \"virtualenvs\" ]; then printf 'other-venv\\n'; fi\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake_pyenv, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

        let action = PyenvVirtualenv {
            python_version: Some(String::from("3.12.0")),
            name: Some(String::from("myproject")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old_path);

        assert_eq!(
            1,
            steps.len(),
            "expected one step when virtualenv not in list"
        );
        let display = steps[0].atom.to_string();
        assert!(display.contains("myproject"), "expected name in: {display}");
    }

    #[test]
    #[serial]
    fn plan_generates_step_when_pyenv_not_in_path() {
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", "/nonexistent");

        let action = PyenvVirtualenv {
            python_version: Some(String::from("3.12.0")),
            name: Some(String::from("myproject")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old_path);

        // virtualenv_exists returns false on error → fail-safe: generate step
        assert_eq!(1, steps.len());
    }
}
