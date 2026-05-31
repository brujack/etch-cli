use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GemInstall {
    /// Single gem name to install. Mutually exclusive with `list`.
    pub name: Option<String>,
    /// List of gem names to install. Mutually exclusive with `name`.
    #[serde(default)]
    pub list: Vec<String>,
    /// Version constraint (e.g. "2.4.0"). Only meaningful when installing a single gem.
    pub version: Option<String>,
}

impl GemInstall {
    fn gem_names(&self) -> Vec<String> {
        if !self.list.is_empty() {
            self.list.clone()
        } else if let Some(name) = &self.name {
            vec![name.clone()]
        } else {
            vec![]
        }
    }

    fn gem_installed(name: &str) -> bool {
        std::process::Command::new("gem")
            .args(["list", "--installed", name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl Action for GemInstall {
    fn summarize(&self) -> String {
        let gems = self.gem_names();
        if gems.is_empty() {
            return String::from("Installing gems");
        }
        format!("Installing gem(s): {}", gems.join(", "))
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let gems = self.gem_names();
        if gems.is_empty() {
            bail!("gem.install requires either 'name' or 'list' to be specified");
        }

        let to_install: Vec<String> = gems
            .into_iter()
            .filter(|name| !Self::gem_installed(name))
            .collect();

        if to_install.is_empty() {
            return Ok(vec![]);
        }

        let mut arguments = vec![String::from("install")];
        arguments.extend(to_install);
        if let Some(v) = &self.version {
            arguments.push(String::from("--version"));
            arguments.push(v.clone());
        }

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("gem"),
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

    const FAKE_GEM: &str = "etch_cli_not_a_real_gem_xyz_zyx_test";

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: gem.install
  name: bundler
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::GemInstall(action)) => {
                assert_eq!(Some("bundler".to_string()), action.action.name);
                assert!(action.action.list.is_empty());
                assert!(action.action.version.is_none());
            }
            _ => panic!("GemInstall didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_list() {
        let yaml = r#"
- action: gem.install
  list:
    - bundler
    - rake
    - test-kitchen
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::GemInstall(action)) => {
                assert_eq!(vec!["bundler", "rake", "test-kitchen"], action.action.list);
                assert!(action.action.name.is_none());
            }
            _ => panic!("GemInstall didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_version() {
        let yaml = r#"
- action: gem.install
  name: bundler
  version: "2.4.0"
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::GemInstall(action)) => {
                assert_eq!(Some("bundler".to_string()), action.action.name);
                assert_eq!(Some("2.4.0".to_string()), action.action.version);
            }
            _ => panic!("GemInstall didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn summarize_includes_gem_name() {
        let action = GemInstall {
            name: Some(String::from("bundler")),
            list: vec![],
            version: None,
        };
        let s = action.summarize();
        assert!(s.contains("bundler"), "expected 'bundler' in: {s}");
    }

    #[test]
    fn summarize_includes_all_list_gems() {
        let action = GemInstall {
            name: None,
            list: vec![String::from("bundler"), String::from("rake")],
            version: None,
        };
        let s = action.summarize();
        assert!(s.contains("bundler"), "expected 'bundler' in: {s}");
        assert!(s.contains("rake"), "expected 'rake' in: {s}");
    }

    #[test]
    fn summarize_with_no_gems_returns_generic_message() {
        let action = GemInstall::default();
        let s = action.summarize();
        assert!(s.contains("gem"), "expected 'gem' in: {s}");
    }

    #[test]
    fn plan_errors_without_name_or_list() {
        let action = GemInstall::default();
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
    fn plan_returns_exec_for_uninstalled_gem() {
        let action = GemInstall {
            name: Some(String::from(FAKE_GEM)),
            list: vec![],
            version: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("gem"),
            "expected 'gem' command in: {display}"
        );
        assert!(
            display.contains(FAKE_GEM),
            "expected gem name in: {display}"
        );
        assert!(
            display.contains("install"),
            "expected 'install' subcommand in: {display}"
        );
    }

    #[test]
    #[serial]
    fn plan_returns_exec_for_uninstalled_list() {
        let action = GemInstall {
            name: None,
            list: vec![String::from(FAKE_GEM), format!("{FAKE_GEM}2")],
            version: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains(FAKE_GEM),
            "expected gem name in: {display}"
        );
    }

    #[test]
    #[serial]
    fn plan_includes_version_flag_when_set() {
        let action = GemInstall {
            name: Some(String::from(FAKE_GEM)),
            list: vec![],
            version: Some(String::from("1.0.0")),
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("--version"),
            "expected --version flag in: {display}"
        );
        assert!(
            display.contains("1.0.0"),
            "expected version value in: {display}"
        );
    }

    #[test]
    fn gem_names_prefers_list_when_both_set() {
        let action = GemInstall {
            name: Some(String::from("rake")),
            list: vec![String::from("bundler")],
            version: None,
        };
        let names = action.gem_names();
        assert_eq!(vec!["bundler".to_string()], names);
    }

    #[test]
    fn gem_names_returns_single_name_as_vec() {
        let action = GemInstall {
            name: Some(String::from("rake")),
            list: vec![],
            version: None,
        };
        let names = action.gem_names();
        assert_eq!(vec!["rake".to_string()], names);
    }

    #[test]
    fn gem_names_empty_when_no_name_or_list() {
        let action = GemInstall::default();
        assert!(action.gem_names().is_empty());
    }

    #[test]
    #[serial]
    fn plan_skips_already_installed_gem() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake_gem = tmp.path().join("gem");
        // Fake gem exits 0 for any invocation — simulates gem being installed
        std::fs::write(&fake_gem, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&fake_gem, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

        let action = GemInstall {
            name: Some(String::from("bundler")),
            list: vec![],
            version: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old_path);

        assert!(
            steps.is_empty(),
            "expected no steps when gem is already installed"
        );
    }

    #[test]
    #[serial]
    fn plan_skips_already_installed_gems_in_list() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake_gem = tmp.path().join("gem");
        std::fs::write(&fake_gem, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&fake_gem, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old_path}", tmp.path().display()));

        let action = GemInstall {
            name: None,
            list: vec![String::from("bundler"), String::from("rake")],
            version: None,
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old_path);

        assert!(
            steps.is_empty(),
            "expected no steps when all gems are already installed"
        );
    }
}
