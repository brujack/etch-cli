use crate::actions::Action;
use crate::utilities;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{FileAction, FileActionConfig};
use crate::atoms::file::Chown;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChown {
    pub path: String,
    pub user: Option<String>,
    pub group: Option<String>,
    #[serde(flatten)]
    pub config: FileActionConfig,
}

impl FileChown {}

impl FileAction for FileChown {
    fn file_action_config(&self) -> &FileActionConfig {
        &self.config
    }
}

impl Action for FileChown {
    fn summarize(&self) -> String {
        format!("Changing ownership for file {}", self.path)
    }

    fn state_key(&self) -> String {
        self.path.clone()
    }

    fn plan(
        &self,
        _: &crate::manifests::Manifest,
        contexts: &crate::contexts::Contexts,
    ) -> anyhow::Result<Vec<crate::steps::Step>> {
        if self.config.privileged {
            use crate::atoms::command::Exec;
            let privilege_provider =
                utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());
            let ownership = match (&self.user, &self.group) {
                (Some(u), Some(g)) => format!("{}:{}", u, g),
                (Some(u), None) => u.clone(),
                (None, Some(g)) => format!(":{}", g),
                (None, None) => return Ok(vec![]),
            };
            return Ok(vec![crate::steps::Step {
                atom: Box::new(Exec {
                    command: "chown".into(),
                    arguments: vec![ownership, self.path.clone()],
                    privileged: true,
                    privilege_provider,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            }]);
        }

        let steps = vec![crate::steps::Step {
            atom: Box::new(Chown {
                path: self.path.clone().parse()?,
                owner: self.user.clone().unwrap_or("".to_string()),
                group: self.group.clone().unwrap_or("".to_string()),
            }),
            initializers: vec![],
            finalizers: vec![],
        }];

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized_user() {
        let yaml = r#"
- action: file.chown
  path: /home/test/one
  user: test
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileChown(action)) => {
                assert_eq!("/home/test/one", action.action.path);
                assert_eq!("test", action.action.user.unwrap());
                assert_eq!(None, action.action.group);
            }
            _ => {
                panic!("FileChown didn't deserialize to the correct type");
            }
        };
    }

    #[test]
    fn it_can_be_deserialized_group() {
        let yaml = r#"
- action: file.chown
  path: /home/test/one
  group: test
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileChown(action)) => {
                assert_eq!("/home/test/one", action.action.path);
                assert_eq!(None, action.action.user);
                assert_eq!("test", action.action.group.unwrap());
            }
            _ => {
                panic!("FileChown didn't deserialize to the correct type");
            }
        };
    }

    #[test]
    fn plan_returns_chown_step() {
        use super::FileChown;
        use crate::actions::Action;
        let action = FileChown {
            path: String::from("/tmp/testfile"),
            user: Some(String::from("alice")),
            group: Some(String::from("staff")),
            ..Default::default()
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn plan_uses_empty_string_for_missing_user() {
        use super::FileChown;
        use crate::actions::Action;
        let action = FileChown {
            path: String::from("/tmp/testfile"),
            user: None,
            group: Some(String::from("staff")),
            ..Default::default()
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
    }

    #[test]
    fn it_can_be_deserialized_with_privileged() {
        use crate::actions::Actions;
        let yaml = r#"
- action: file.chown
  path: /tmp/file
  user: root
  sudo: true
"#;
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        match actions.pop() {
            Some(Actions::FileChown(action)) => {
                assert!(action.action.config.privileged);
            }
            _ => panic!("FileChown didn't deserialize"),
        }
    }

    #[test]
    fn plan_returns_exec_step_when_privileged() {
        use super::FileChown;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;
        let action = FileChown {
            path: String::from("/tmp/file"),
            user: Some(String::from("root")),
            group: Some(String::from("root")),
            config: FileActionConfig { privileged: true },
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        // Verify the step is an Exec atom (chown command), not the native Chown atom
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("CommandExec"),
            "expected Exec atom, got: {display}"
        );
        assert!(
            display.contains("chown"),
            "expected chown command in Exec atom, got: {display}"
        );
    }

    #[test]
    fn plan_privileged_group_only() {
        use super::FileChown;
        use crate::actions::file::FileActionConfig;
        use crate::actions::Action;
        let action = FileChown {
            path: String::from("/tmp/file"),
            user: None,
            group: Some(String::from("staff")),
            config: FileActionConfig { privileged: true },
        };
        let steps = action
            .plan(
                &crate::manifests::Manifest::default(),
                &crate::contexts::Contexts::default(),
            )
            .unwrap();
        assert_eq!(1, steps.len());
        // Verify the step is an Exec atom (chown command), not the native Chown atom
        let display = steps[0].atom.to_string();
        assert!(
            display.contains("CommandExec"),
            "expected Exec atom, got: {display}"
        );
        assert!(
            display.contains("chown"),
            "expected chown command in Exec atom, got: {display}"
        );
    }
}
