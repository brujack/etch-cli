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
    use crate::test_helpers::EnvGuard;
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

        let _path = EnvGuard::prepend_path(tmp.path());

        let steps = ClaudeUpgrade::default()
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        assert_eq!(2, steps.len());
        let d0 = steps[0].atom.to_string();
        let d1 = steps[1].atom.to_string();
        assert!(d0.contains("superpowers@official"), "got: {d0}");
        assert!(d1.contains("context7@upstash"), "got: {d1}");
        assert!(d0.contains("update"), "got: {d0}");
    }

    #[test]
    #[serial]
    fn plan_updates_a_plugin_listed_in_several_scopes_once() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake = tmp.path().join("claude");
        std::fs::write(
            &fake,
            "#!/bin/sh\nfor scope in project user project; do\n  printf '  ❯ caveman@caveman\\n    Scope: %s\\n\\n  ❯ context7@upstash\\n    Scope: %s\\n\\n' \"$scope\" \"$scope\"\ndone\nexit 0\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

        let _path = EnvGuard::prepend_path(tmp.path());

        let steps = ClaudeUpgrade::default()
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        let atoms: Vec<String> = steps.iter().map(|s| s.atom.to_string()).collect();
        assert_eq!(2, atoms.len(), "got: {atoms:?}");
        assert!(atoms[0].contains("caveman@caveman"), "got: {atoms:?}");
        assert!(atoms[1].contains("context7@upstash"), "got: {atoms:?}");
    }

    #[test]
    #[serial]
    fn plan_returns_empty_when_none_installed() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let fake = tmp.path().join("claude");
        std::fs::write(&fake, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

        let _path = EnvGuard::prepend_path(tmp.path());

        let steps = ClaudeUpgrade::default()
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        assert!(
            steps.is_empty(),
            "expected no steps when no plugins installed"
        );
    }

    #[test]
    #[serial]
    fn plan_returns_empty_when_claude_not_in_path() {
        let _path = EnvGuard::set("PATH", "/nonexistent");

        let steps = ClaudeUpgrade::default()
            .plan(&Manifest::default(), &Contexts::default())
            .unwrap();

        assert!(steps.is_empty(), "expected no steps when claude not found");
    }
}
