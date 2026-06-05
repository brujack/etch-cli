use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::bail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeInstall {
    pub name: Option<String>,
    #[serde(default)]
    pub list: Vec<String>,
}

impl ClaudeInstall {
    fn plugin_names(&self) -> Vec<String> {
        if !self.list.is_empty() {
            self.list.clone()
        } else if let Some(name) = &self.name {
            vec![name.clone()]
        } else {
            vec![]
        }
    }

    fn base_name(plugin: &str) -> &str {
        plugin.split('@').next().unwrap_or(plugin)
    }

    fn installed_base_names() -> std::collections::HashSet<String> {
        std::process::Command::new("claude")
            .args(["plugins", "list"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
                super::parse_plugin_list(&stdout)
                    .into_iter()
                    .map(|tok| Self::base_name(&tok).to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Action for ClaudeInstall {
    fn summarize(&self) -> String {
        let names = self.plugin_names();
        if names.is_empty() {
            return String::from("Installing Claude plugins");
        }
        format!("Installing Claude plugin(s): {}", names.join(", "))
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let names = self.plugin_names();
        if names.is_empty() {
            bail!("claude.install requires either 'name' or 'list'");
        }

        let installed = Self::installed_base_names();
        let steps = names
            .into_iter()
            .filter(|n| !installed.contains(Self::base_name(n)))
            .map(|name| Step {
                atom: Box::new(Exec {
                    command: String::from("claude"),
                    arguments: vec![String::from("plugins"), String::from("install"), name],
                    streaming: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            })
            .collect();

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;
    use crate::contexts::Contexts;
    use crate::manifests::Manifest;
    use serial_test::serial;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = "- action: claude.install\n  name: superpowers\n";
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::ClaudeInstall(a)) => {
                assert_eq!(Some("superpowers".to_string()), a.action.name);
                assert!(a.action.list.is_empty());
            }
            _ => panic!("expected ClaudeInstall"),
        }
    }

    #[test]
    fn it_can_be_deserialized_with_list() {
        let yaml = concat!(
            "- action: claude.install\n",
            "  list:\n",
            "    - superpowers\n",
            "    - context7\n",
        );
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::ClaudeInstall(a)) => {
                assert_eq!(vec!["superpowers", "context7"], a.action.list);
                assert!(a.action.name.is_none());
            }
            _ => panic!("expected ClaudeInstall"),
        }
    }

    #[test]
    fn base_name_strips_marketplace() {
        assert_eq!(
            "superpowers",
            ClaudeInstall::base_name("superpowers@claude-plugins-official")
        );
        assert_eq!("foo", ClaudeInstall::base_name("foo@bar"));
        assert_eq!("plain", ClaudeInstall::base_name("plain"));
    }

    #[test]
    fn summarize_includes_plugin_name() {
        let action = ClaudeInstall {
            name: Some(String::from("superpowers")),
            list: vec![],
        };
        let s = action.summarize();
        assert!(s.contains("superpowers"), "expected 'superpowers' in: {s}");
    }

    #[test]
    fn summarize_includes_all_list_plugins() {
        let action = ClaudeInstall {
            name: None,
            list: vec![String::from("superpowers"), String::from("context7")],
        };
        let s = action.summarize();
        assert!(s.contains("superpowers"), "got: {s}");
        assert!(s.contains("context7"), "got: {s}");
    }

    #[test]
    fn summarize_with_no_plugins_returns_generic() {
        let s = ClaudeInstall::default().summarize();
        assert!(s.to_lowercase().contains("claude"), "got: {s}");
    }

    #[test]
    fn plan_errors_without_name_or_list() {
        let result = ClaudeInstall::default().plan(&Manifest::default(), &Contexts::default());
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(msg.contains("name") || msg.contains("list"), "got: {msg}");
    }

    #[test]
    #[serial]
    fn plan_returns_exec_for_uninstalled_plugin() {
        let fake = "etch_cli_fake_plugin_zyx_xyz_test";
        let action = ClaudeInstall {
            name: Some(String::from(fake)),
            list: vec![],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("claude"), "got: {display}");
        assert!(display.contains("install"), "got: {display}");
        assert!(display.contains(fake), "got: {display}");
    }

    #[test]
    #[serial]
    fn plan_generates_step_when_claude_not_in_path() {
        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", "/nonexistent");
        let action = ClaudeInstall {
            name: Some(String::from("superpowers")),
            list: vec![],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        std::env::set_var("PATH", old);
        assert_eq!(
            1,
            steps.len(),
            "fail-safe: generate step when claude not found"
        );
    }

    #[test]
    #[serial]
    fn plan_skips_already_installed_plugin() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake = tmp.path().join("claude");
        std::fs::write(
            &fake,
            "#!/bin/sh\nprintf '❯ superpowers@claude-plugins-official\\n'\nexit 0\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old}", tmp.path().display()));

        let action = ClaudeInstall {
            name: Some(String::from("superpowers")),
            list: vec![],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        std::env::set_var("PATH", old);

        assert!(
            steps.is_empty(),
            "expected no steps — plugin already installed"
        );
    }

    #[test]
    #[serial]
    fn plan_returns_empty_when_all_installed() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake = tmp.path().join("claude");
        std::fs::write(
            &fake,
            "#!/bin/sh\nprintf '❯ superpowers@official\\n❯ context7@upstash\\n'\nexit 0\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old}", tmp.path().display()));

        let action = ClaudeInstall {
            name: None,
            list: vec![String::from("superpowers"), String::from("context7")],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        std::env::set_var("PATH", old);

        assert!(steps.is_empty(), "expected no steps — all installed");
    }

    #[test]
    #[serial]
    fn plan_handles_marketplace_suffix_in_name() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake = tmp.path().join("claude");
        std::fs::write(&fake, "#!/bin/sh\nprintf '❯ foo@bar\\n'\nexit 0\n").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old}", tmp.path().display()));

        let action = ClaudeInstall {
            name: Some(String::from("foo@bar")),
            list: vec![],
        };
        let steps = action
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();
        std::env::set_var("PATH", old);

        assert!(
            steps.is_empty(),
            "expected no steps — foo@bar already installed as foo"
        );
    }
}
