use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Resolve the pyenv symlink at `{versions_dir}/{name}` and return the Python
/// version embedded in its target path (the component immediately before `envs/`).
/// Returns None when the path is absent, not a symlink, or has an unexpected layout.
fn installed_python_version(versions_dir: &Path, name: &str) -> Option<String> {
    let target = std::fs::read_link(versions_dir.join(name)).ok()?;
    target
        .components()
        .zip(target.components().skip(1))
        .find_map(|(a, b)| {
            if b.as_os_str() == "envs" {
                Some(a.as_os_str().to_string_lossy().into_owned())
            } else {
                None
            }
        })
}

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PyenvVirtualenv {
    /// Python version to base the virtualenv on (e.g. "3.12.0").
    pub python_version: Option<String>,
    /// Name of the virtualenv to create (e.g. "myproject").
    pub name: Option<String>,
    /// When true, delete and recreate the virtualenv if its Python version
    /// differs from `python_version`. Default false preserves existing behavior.
    #[serde(default)]
    pub recreate: bool,
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
            if !self.recreate {
                return Ok(vec![]);
            }
            let versions_dir = PathBuf::from(shellexpand::tilde("~/.pyenv/versions").into_owned());
            let current = installed_python_version(&versions_dir, &name);
            if current
                .as_deref()
                .is_none_or(|v| v == python_version.as_str())
            {
                return Ok(vec![]);
            }
            return Ok(vec![
                Step {
                    atom: Box::new(Exec {
                        command: String::from("pyenv"),
                        arguments: vec![
                            String::from("uninstall"),
                            String::from("-f"),
                            name.clone(),
                        ],
                        ..Default::default()
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                },
                Step {
                    atom: Box::new(Exec {
                        command: String::from("pyenv"),
                        arguments: vec![String::from("virtualenv"), python_version, name],
                        ..Default::default()
                    }),
                    initializers: vec![],
                    finalizers: vec![],
                },
            ]);
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
            recreate: false,
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
            recreate: false,
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
            recreate: false,
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
            recreate: false,
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
            recreate: false,
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
            recreate: false,
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
            recreate: false,
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
            recreate: false,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old_path);

        // virtualenv_exists returns false on error → fail-safe: generate step
        assert_eq!(1, steps.len());
    }

    // ── installed_python_version helper tests ──────────────────────────────

    #[test]
    fn installed_python_version_returns_none_when_not_a_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let versions_dir = tmp.path().join("versions");
        std::fs::create_dir_all(versions_dir.join("ansible")).unwrap();
        let result = installed_python_version(&versions_dir, "ansible");
        assert!(result.is_none(), "expected None for non-symlink path");
    }

    #[test]
    fn installed_python_version_returns_none_when_venv_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let versions_dir = tmp.path().join("versions");
        std::fs::create_dir_all(&versions_dir).unwrap();
        let result = installed_python_version(&versions_dir, "ansible");
        assert!(
            result.is_none(),
            "expected None when venv path does not exist"
        );
    }

    #[test]
    fn installed_python_version_returns_version_from_relative_symlink() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let versions_dir = tmp.path().join("versions");
        let target = versions_dir.join("3.14.5").join("envs").join("ansible");
        std::fs::create_dir_all(&target).unwrap();
        symlink(
            Path::new("3.14.5/envs/ansible"),
            versions_dir.join("ansible"),
        )
        .unwrap();
        let result = installed_python_version(&versions_dir, "ansible");
        assert_eq!(result, Some("3.14.5".to_string()));
    }

    #[test]
    fn installed_python_version_returns_version_from_absolute_symlink() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let versions_dir = tmp.path().join("versions");
        let target = versions_dir.join("3.12.0").join("envs").join("myproject");
        std::fs::create_dir_all(&target).unwrap();
        symlink(&target, versions_dir.join("myproject")).unwrap();
        let result = installed_python_version(&versions_dir, "myproject");
        assert_eq!(result, Some("3.12.0".to_string()));
    }

    // ── plan() recreate tests ──────────────────────────────────────────────

    #[test]
    #[serial]
    fn plan_recreate_true_creates_when_no_venv() {
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", "/nonexistent");

        let action = PyenvVirtualenv {
            python_version: Some(String::from("3.14.5")),
            name: Some(String::from("ansible")),
            recreate: true,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old_path);

        assert_eq!(1, steps.len(), "expected 1 create step when venv absent");
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("virtualenv"),
            "expected 'virtualenv' in: {display}"
        );
        assert!(display.contains("3.14.5"), "expected version in: {display}");
        assert!(display.contains("ansible"), "expected name in: {display}");
    }

    #[test]
    #[serial]
    fn plan_recreate_false_skips_existing_venv_unchanged() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let fake_pyenv = tmp.path().join("pyenv");
        std::fs::write(
            &fake_pyenv,
            "#!/bin/sh\nif [ \"$1\" = \"virtualenvs\" ]; then printf 'ansible\\n'; fi\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake_pyenv, std::fs::Permissions::from_mode(0o755)).unwrap();
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

        let action = PyenvVirtualenv {
            python_version: Some(String::from("3.14.5")),
            name: Some(String::from("ansible")),
            recreate: false,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        std::env::set_var("PATH", old_path);

        assert!(
            steps.is_empty(),
            "expected no steps when recreate:false and venv exists"
        );
    }

    #[test]
    #[serial]
    fn plan_recreate_true_skips_when_version_matches() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let tmp = tempfile::tempdir().unwrap();

        let fake_pyenv = tmp.path().join("pyenv");
        std::fs::write(
            &fake_pyenv,
            "#!/bin/sh\nif [ \"$1\" = \"virtualenvs\" ]; then printf 'ansible\\n'; fi\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake_pyenv, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old_home = std::env::var("HOME").unwrap_or_default();
        std::env::set_var("HOME", tmp.path());

        let versions_dir = tmp.path().join(".pyenv").join("versions");
        let target_dir = versions_dir.join("3.14.5").join("envs").join("ansible");
        std::fs::create_dir_all(&target_dir).unwrap();
        symlink(
            Path::new("3.14.5/envs/ansible"),
            versions_dir.join("ansible"),
        )
        .unwrap();

        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

        let action = PyenvVirtualenv {
            python_version: Some(String::from("3.14.5")),
            name: Some(String::from("ansible")),
            recreate: true,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("HOME", old_home);
        std::env::set_var("PATH", old_path);

        assert!(
            steps.is_empty(),
            "expected no steps when version already matches"
        );
    }

    #[test]
    #[serial]
    fn plan_recreate_true_recreates_when_version_differs() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let tmp = tempfile::tempdir().unwrap();

        let fake_pyenv = tmp.path().join("pyenv");
        std::fs::write(
            &fake_pyenv,
            "#!/bin/sh\nif [ \"$1\" = \"virtualenvs\" ]; then printf 'ansible\\n'; fi\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake_pyenv, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old_home = std::env::var("HOME").unwrap_or_default();
        std::env::set_var("HOME", tmp.path());

        let versions_dir = tmp.path().join(".pyenv").join("versions");
        let target_dir = versions_dir.join("3.14.4").join("envs").join("ansible");
        std::fs::create_dir_all(&target_dir).unwrap();
        symlink(
            Path::new("3.14.4/envs/ansible"),
            versions_dir.join("ansible"),
        )
        .unwrap();

        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

        let action = PyenvVirtualenv {
            python_version: Some(String::from("3.14.5")),
            name: Some(String::from("ansible")),
            recreate: true,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("HOME", old_home);
        std::env::set_var("PATH", old_path);

        assert_eq!(2, steps.len(), "expected uninstall + create steps");
        let s0 = steps[0].atom.to_string();
        let s1 = steps[1].atom.to_string();
        assert!(
            s0.contains("uninstall"),
            "step 0 should be uninstall, got: {s0}"
        );
        assert!(s0.contains("-f"), "uninstall should use -f flag, got: {s0}");
        assert!(
            s0.contains("ansible"),
            "uninstall should name the venv, got: {s0}"
        );
        assert!(
            s1.contains("virtualenv"),
            "step 1 should be virtualenv create, got: {s1}"
        );
        assert!(
            s1.contains("3.14.5"),
            "step 1 should use new version, got: {s1}"
        );
    }

    #[test]
    #[serial]
    fn plan_recreate_true_recreates_when_version_undetectable() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();

        let fake_pyenv = tmp.path().join("pyenv");
        std::fs::write(
            &fake_pyenv,
            "#!/bin/sh\nif [ \"$1\" = \"virtualenvs\" ]; then printf 'ansible\\n'; fi\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake_pyenv, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old_home = std::env::var("HOME").unwrap_or_default();
        std::env::set_var("HOME", tmp.path());
        // No symlink created — installed_python_version returns None

        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

        let action = PyenvVirtualenv {
            python_version: Some(String::from("3.14.5")),
            name: Some(String::from("ansible")),
            recreate: true,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("HOME", old_home);
        std::env::set_var("PATH", old_path);

        assert!(
            steps.is_empty(),
            "expected no steps when version undetectable (safe skip)"
        );
    }
}
