use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeUpgrade {}

impl ClaudeUpgrade {
    fn installed_plugins() -> Vec<String> {
        std::process::Command::new("claude")
            .args(["plugins", "list"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
                super::parse_plugin_list(&stdout)
            })
            .unwrap_or_default()
    }
}

impl Action for ClaudeUpgrade {
    fn summarize(&self) -> String {
        String::from("Upgrading all installed Claude plugins")
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let steps = Self::installed_plugins()
            .into_iter()
            .map(|token| Step {
                atom: Box::new(Exec {
                    command: String::from("claude"),
                    arguments: vec![String::from("plugins"), String::from("update"), token],
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
        let yaml = "- action: claude.upgrade\n";
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::ClaudeUpgrade(_)) => {}
            _ => panic!("expected ClaudeUpgrade"),
        }
    }

    #[test]
    fn summarize_returns_string() {
        let s = ClaudeUpgrade::default().summarize();
        assert!(!s.is_empty());
        assert!(s.to_lowercase().contains("claude"), "got: {s}");
    }

    #[test]
    #[serial]
    fn plan_returns_exec_for_each_installed_plugin() {
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

        let steps = ClaudeUpgrade::default()
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old);

        assert_eq!(2, steps.len());
        let d0 = steps[0].atom.to_string();
        let d1 = steps[1].atom.to_string();
        assert!(d0.contains("superpowers@official"), "got: {d0}");
        assert!(d1.contains("context7@upstash"), "got: {d1}");
        assert!(d0.contains("update"), "got: {d0}");
    }

    #[test]
    #[serial]
    fn plan_returns_empty_when_none_installed() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake = tmp.path().join("claude");
        std::fs::write(&fake, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{old}", tmp.path().display()));

        let steps = ClaudeUpgrade::default()
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old);

        assert!(
            steps.is_empty(),
            "expected no steps when no plugins installed"
        );
    }

    #[test]
    #[serial]
    fn plan_returns_empty_when_claude_not_in_path() {
        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", "/nonexistent");

        let steps = ClaudeUpgrade::default()
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        std::env::set_var("PATH", old);

        assert!(steps.is_empty(), "expected no steps when claude not found");
    }
}
