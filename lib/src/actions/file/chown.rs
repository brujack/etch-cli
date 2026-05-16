use crate::actions::Action;
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

    fn plan(
        &self,
        _: &crate::manifests::Manifest,
        _: &crate::contexts::Contexts,
    ) -> anyhow::Result<Vec<crate::steps::Step>> {
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
                panic!("FileCopy didn't deserialize to the correct type");
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
                panic!("FileCopy didn't deserialize to the correct type");
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
}
